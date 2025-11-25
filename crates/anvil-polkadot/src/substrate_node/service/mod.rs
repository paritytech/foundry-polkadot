use crate::{
    AnvilNodeConfig,
    substrate_node::{
        mining_engine::{MiningEngine, MiningMode, run_mining_engine},
        rpc::spawn_rpc_server,
    },
};
use anvil::eth::backend::time::TimeManager;
use codec::{Decode, Encode};
use parking_lot::Mutex;
use polkadot_sdk::{
    cumulus_client_parachain_inherent::MockValidationDataInherentDataProvider,
    cumulus_primitives_core::{GetParachainInfo, relay_chain},
    parachains_common::{Hash, opaque::Block},
    polkadot_primitives::{self},
    sc_basic_authorship, sc_chain_spec,
    sc_client_api::{Backend as BackendT, StateBackend, TrieCacheContext},
    sc_consensus::{self},
    sc_consensus_manual_seal::{
        ManualSealParams, consensus::aura::AuraConsensusDataProvider, run_manual_seal,
    },
    sc_service::{
        self, Configuration, RpcHandlers, SpawnTaskHandle, TaskManager,
        error::Error as ServiceError,
    },
    sc_transaction_pool::{self},
    sp_api::ProvideRuntimeApi,
    sp_arithmetic::traits::UniqueSaturatedInto,
    sp_blockchain,
    sp_consensus_aura::{AuraApi, Slot},
    sp_core::hexdisplay::AsBytesRef,
    sp_timestamp,
};
use std::sync::Arc;
use subxt::{PolkadotConfig, backend::rpc::RpcClient, ext::subxt_rpcs::rpc_params, utils::H256};
use tokio_stream::wrappers::ReceiverStream;

use tokio::runtime::Builder as TokioRtBuilder;

use serde_json::{Map, Value, json};

use indicatif::{ProgressBar, ProgressStyle};

pub use backend::{BackendError, BackendWithOverlay, StorageOverrides};
pub use client::Client;

mod backend;
mod client;
mod consensus;
mod executor;
pub mod storage;

pub type Backend = sc_service::TFullBackend<Block>;

pub type TransactionPoolHandle = sc_transaction_pool::TransactionPoolHandle<Block, Client>;

type SelectChain = sc_consensus::LongestChain<Backend, Block>;

#[derive(Clone)]
pub struct Service {
    pub spawn_handle: SpawnTaskHandle,
    pub client: Arc<Client>,
    pub backend: Arc<Backend>,
    pub tx_pool: Arc<TransactionPoolHandle>,
    pub rpc_handlers: RpcHandlers,
    pub mining_engine: Arc<MiningEngine>,
    pub storage_overrides: Arc<Mutex<StorageOverrides>>,
    pub genesis_block_number: u64,
}

// async fn fork_finalized_head(
//     client: &HttpClient,
//     fork_block_hash: Option<String>,
// ) -> eyre::Result<String> {
//     if let Some(h) = fork_block_hash {
//         return Ok(h);
//     }
//     let res: String = client.request("chain_getFinalizedHead", rpc_params![]).await?;
//     Ok(res)
// }
//
// async fn fork_finalized_head_header(
//     client: &HttpClient,
//     fork_block_hash: String,
// ) -> eyre::Result<String> {
//     let res: String = client.request("chain_getHeader", rpc_params![fork_block_hash]).await?;
//     println!("{}", res);
//     Ok(res)
// }
//
// async fn fork_sync_spec(client: &HttpClient, hash: Option<String>) -> eyre::Result<Vec<u8>> {
//     let pb = ProgressBar::new_spinner();
//     pb.set_style(
//         ProgressStyle::with_template("{spinner:.green} {msg}").unwrap().tick_chars("/|\\- "),
//     );
//     pb.enable_steady_tick(std::time::Duration::from_millis(120));
//     pb.set_message("Downloading sync state spec...");
//
//     let raw = true;
//     let spec_json: serde_json::Value =
//         client.request("sync_state_genSyncSpec", rpc_params![raw, hash]).await?;
//
//     pb.finish_with_message("Sync state spec downloaded ✔");
//
//     Ok(serde_json::to_vec(&spec_json)?)
// }
//
async fn fork_get_all_keys_paged(client: &RpcClient, hash: H256) -> eyre::Result<Vec<String>> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} Fetching key pages... {pos} pages collected",
        )
        .unwrap()
        .tick_chars("/|\\- "),
    );
    pb.enable_steady_tick(std::time::Duration::from_millis(120));

    let mut keys = Vec::new();
    let mut start_key: Option<String> = None;
    let mut page_count: u64 = 0;
    loop {
        let page: Vec<String> = client
            .request("state_getKeysPaged", rpc_params!["0x", 1000u32, start_key.clone(), hash])
            .await?;
        if page.is_empty() {
            break;
        }
        start_key = page.last().cloned();
        keys.extend(page.into_iter());
        page_count += 1;
        pb.set_position(page_count);
    }

    pb.finish_with_message(format!("All keys fetched ✔ (total: {})", keys.len()));
    Ok(keys)
}

async fn fork_storage_map(client: &RpcClient, hash: H256) -> eyre::Result<Map<String, Value>> {
    let keys = fork_get_all_keys_paged(client, hash).await?;

    let pb = ProgressBar::new(keys.len() as u64);
    pb.set_style(
        ProgressStyle::with_template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} values")
            .unwrap()
            .progress_chars("=>-"),
    );
    pb.set_message("Downloading values...");

    let mut storage: Map<String, Value> = Map::new();
    for k in &keys {
        let v: Option<String> =
            client.request("state_getStorage", rpc_params![k.clone(), hash]).await?;
        if let Some(val_hex) = v {
            storage.insert(k.clone(), Value::String(val_hex));
        }
        pb.inc(1);
    }

    pb.finish_with_message("All values downloaded ✔");
    Ok(storage)
}

fn fork_chainspec_from_raw_storage_map(
    storage_map: Map<String, Value>,
) -> sc_service::error::Result<Box<dyn sc_chain_spec::ChainSpec>> {
    let children_default = serde_json::Map::<String, Value>::new();

    let spec_json = json!({
        "name": "Anvil Polkadot (Forked)",
        "id": "anvil-polkadot-forked",
        "chainType": "Development",
        "bootNodes": [],
        "telemetryEndpoints": null,
        "protocolId": null,
        "properties": null,
        "codeSubstitutes": {},
        "consensusEngine": null,
        "genesis": { "raw": { "top": storage_map, "childrenDefault": children_default }}
    });

    let bytes = serde_json::to_vec(&spec_json)
        .map_err(|e| ServiceError::Other(format!("serialize spec json failed: {e}")))?;
    type EmptyExt = Option<()>;
    let new_spec: sc_chain_spec::GenericChainSpec<EmptyExt> =
        sc_chain_spec::GenericChainSpec::from_json_bytes(bytes)
            .map_err(|e| ServiceError::Other(format!("from_json_bytes failed: {e}")))?;
    Ok(Box::new(new_spec))
}

type CreateInherentDataProviders = Box<
    dyn Fn(
            Hash,
            (),
        ) -> futures::future::Ready<
            Result<
                (sp_timestamp::InherentDataProvider, MockValidationDataInherentDataProvider<()>),
                Box<dyn std::error::Error + Send + Sync>,
            >,
        > + Send
        + Sync,
>;

fn create_manual_seal_inherent_data_providers(
    backend: BackendWithOverlay,
    client: Arc<Client>,
    time_manager: Arc<TimeManager>,
) -> CreateInherentDataProviders {
    Box::new(move |block: Hash, ()| {
        let current_para_head = client
            .header(block)
            .expect("Header lookup should succeed")
            .expect("Header passed in as parent should be present in backend.");

        let current_para_block_head =
            Some(polkadot_primitives::HeadData(current_para_head.encode()));

        let next_block_number =
            UniqueSaturatedInto::<u32>::unique_saturated_into(current_para_head.number) + 1;
        let slot_duration = client.runtime_api().slot_duration(current_para_head.hash()).unwrap();
        let para_id = client.runtime_api().parachain_id(current_para_head.hash()).unwrap();
        let next_time = time_manager.next_timestamp();
        let parachain_slot = next_time / slot_duration.as_millis();

        let (slot_in_state, _) = backend.read_relay_slot_info(current_para_head.hash()).unwrap();
        let last_rc_block_number =
            backend.read_last_relay_chain_block_number(current_para_head.hash()).unwrap();

        // Used to set the relay chain slot provided via the proof (which is represented
        // by a set of relay chain state keys). The slot is read from the proof at the moment
        // we call consensus hook to perform validations of the relay chain state. We will
        // check:
        // - Ensures blocks are not produced faster than the specified velocity `V` (however, given
        // the nature of the anvil-polkadot mining strategies, we'll hack the check to never fail)
        // - Verifies parachain slot alignment with relay chain slot (meaning time passes similarly
        // on both chains, and the additional key values set below ensures it)
        let additional_key_values = vec![(
            relay_chain::well_known_keys::CURRENT_SLOT.to_vec(),
            Slot::from(parachain_slot).encode(),
        )];

        // This helps with allowing greater block production velocity per relay chain slot.
        backend.inject_relay_slot_info(current_para_head.hash(), (slot_in_state, 0));

        let mocked_parachain = MockValidationDataInherentDataProvider::<()> {
            current_para_block: next_block_number,
            para_id,
            // This is used behind the scenes to set the relay parent number
            // on top of which we build this block. The new last rc block number
            // known by the parachain will be set to the value bellow when the parachain
            // block is finalized.
            relay_offset: last_rc_block_number + 1,
            current_para_block_head,
            additional_key_values: Some(additional_key_values),
            ..Default::default()
        };

        let timestamp_provider = sp_timestamp::InherentDataProvider::new(next_time.into());

        futures::future::ready(Ok((timestamp_provider, mocked_parachain)))
    })
}

/// Builds a new service for a full client.
pub fn new(
    anvil_config: &AnvilNodeConfig,
    mut config: Configuration,
) -> Result<(Service, TaskManager), ServiceError> {
    let mut genesis_block_number = anvil_config.get_genesis_number();
    if let Some(ref fork_url) = anvil_config.eth_rpc_url {
        let http_url = fork_url.clone();
        let storage_map =
            std::thread::spawn(move || -> eyre::Result<Result<(u64, Map<String, Value>), ()>> {
                let rt = TokioRtBuilder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| eyre::eyre!("tokio rt build error: {e}"))?;
                rt.block_on(async move {
                    let client =
                        subxt::client::OnlineClient::<PolkadotConfig>::from_url(http_url.clone())
                            .await
                            .unwrap();
                    // let mut headers = HeaderMap::new();
                    // headers
                    //     .insert("Accept-Encoding", HeaderValue::from_static("gzip, deflate,
                    // br")); let http = HttpClientBuilder::default()
                    //     .set_headers(headers)
                    //     .build(http_url)
                    //     .map_err(|e| eyre::eyre!("http client build error: {e}"))?;
                    let finalized_block_ref =
                        client.backend().latest_finalized_block_ref().await.unwrap();
                    let finalized_head_header = client
                        .backend()
                        .block_header(finalized_block_ref.hash())
                        .await
                        .unwrap()
                        .unwrap();
                    println!("fork finalized block number {}", finalized_head_header.number);
                    // let try_sync = fork_sync_spec(&http,
                    // Some(finalized_head_hash.clone())).await;
                    // let storage = client.storage().at(finalized_block_ref.clone());
                    let rpc_client = RpcClient::from_url(http_url).await.unwrap();
                    let storage_map =
                        fork_storage_map(&rpc_client, finalized_block_ref.hash()).await.unwrap();
                    // let spec_bytes = fork_chainspec_from_raw_storage_map(storage_map)

                    Ok(Ok((finalized_head_header.number.into(), storage_map)))
                })
            })
            .join()
            .map_err(|_| ServiceError::Other("tokio thread panicked".into()))?
            .map_err(|e| ServiceError::Other(format!("fork fetch failed: {e}")))?;

        match storage_map {
            Ok((genesis_number, storage_map)) => {
                config.chain_spec = fork_chainspec_from_raw_storage_map(storage_map)?;
                genesis_block_number = genesis_number;
            }
            _ => {
                panic!("shouldn't happen")
            }
        }
    }

    let storage_overrides = Arc::new(Mutex::new(StorageOverrides::default()));

    let (client, backend, keystore, mut task_manager) = client::new_client(
        genesis_block_number,
        &config,
        sc_service::new_wasm_executor(&config.executor),
        storage_overrides.clone(),
    )?;

    let transaction_pool = Arc::from(
        sc_transaction_pool::Builder::new(
            task_manager.spawn_essential_handle(),
            client.clone(),
            config.role.is_authority().into(),
        )
        .with_options(config.transaction_pool.clone())
        .build(),
    );

    // Inform the tx pool about imported and finalized blocks.
    task_manager.spawn_handle().spawn(
        "txpool-notifications",
        Some("transaction-pool"),
        sc_transaction_pool::notification_future(client.clone(), transaction_pool.clone()),
    );

    let (seal_engine_command_sender, commands_stream) = tokio::sync::mpsc::channel(1024);
    let commands_stream = ReceiverStream::new(commands_stream);

    let mining_mode =
        MiningMode::new(anvil_config.block_time, anvil_config.mixed_mining, anvil_config.no_mining);
    let time_manager = Arc::new(TimeManager::new_with_milliseconds(
        sp_timestamp::Timestamp::from(
            anvil_config
                .get_genesis_timestamp()
                .checked_mul(1000)
                .ok_or(ServiceError::Application("Genesis timestamp overflow".into()))?,
        )
        .into(),
    ));

    let mining_engine = Arc::new(MiningEngine::new(
        mining_mode,
        transaction_pool.clone(),
        time_manager.clone(),
        seal_engine_command_sender,
    ));

    let rpc_handlers = spawn_rpc_server(
        genesis_block_number,
        &mut task_manager,
        client.clone(),
        config,
        transaction_pool.clone(),
        keystore,
        backend.clone(),
    )?;

    task_manager.spawn_handle().spawn(
        "mining_engine_task",
        Some("consensus"),
        run_mining_engine(mining_engine.clone()),
    );

    let proposer = sc_basic_authorship::ProposerFactory::new(
        task_manager.spawn_handle(),
        client.clone(),
        transaction_pool.clone(),
        None,
        None,
    );

    let aura_digest_provider = AuraConsensusDataProvider::new(client.clone());
    let backend_with_overlay = BackendWithOverlay::new(backend.clone(), storage_overrides.clone());
    let create_inherent_data_providers = create_manual_seal_inherent_data_providers(
        backend_with_overlay,
        client.clone(),
        time_manager,
    );

    let params = ManualSealParams {
        block_import: client.clone(),
        env: proposer,
        client: client.clone(),
        pool: transaction_pool.clone(),
        select_chain: SelectChain::new(backend.clone()),
        commands_stream: Box::pin(commands_stream),
        consensus_data_provider: Some(Box::new(aura_digest_provider)),
        create_inherent_data_providers,
    };
    let authorship_future = run_manual_seal(params);

    task_manager.spawn_essential_handle().spawn_blocking(
        "manual-seal",
        "substrate",
        authorship_future,
    );

    Ok((
        Service {
            spawn_handle: task_manager.spawn_handle(),
            client,
            backend,
            tx_pool: transaction_pool,
            rpc_handlers,
            mining_engine,
            storage_overrides,
            genesis_block_number,
        },
        task_manager,
    ))
}
