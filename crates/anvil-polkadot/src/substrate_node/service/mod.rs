use crate::{
    AnvilNodeConfig,
    substrate_node::{
        mining_engine::{MiningEngine, MiningMode, run_mining_engine},
        rpc::spawn_rpc_server,
        service::consensus::SameSlotConsensusDataProvider,
    },
};
#[cfg(feature = "forking-support")]
use crate::{
    config::ForkChoice, substrate_node::lazy_loading::backend::Backend as LazyLoadingBackend,
};
use anvil::eth::backend::time::TimeManager;
use codec::Encode;
use parking_lot::Mutex;
use polkadot_sdk::{
    cumulus_client_parachain_inherent::MockValidationDataInherentDataProvider,
    cumulus_primitives_core::{ParaId, relay_chain},
    parachains_common::{Hash, opaque::Block},
    polkadot_primitives::HeadData,
    sc_basic_authorship, sc_consensus,
    sc_consensus_manual_seal::{self},
    sc_service::{
        self, Configuration, RpcHandlers, SpawnTaskHandle, TaskManager,
        error::Error as ServiceError,
    },
    sc_transaction_pool,
    sp_api::ProvideRuntimeApi,
    sp_arithmetic::traits::UniqueSaturatedInto,
    sp_consensus_aura::{AuraApi, Slot},
    sp_timestamp,
};
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;

#[cfg(feature = "forking-support")]
use subxt::PolkadotConfig;

pub use backend::{BackendError, BackendWithOverlay, StorageOverrides};
pub use client::Client;

mod backend;
mod client;
mod consensus;
mod executor;
pub mod storage;

#[cfg(feature = "forking-support")]
pub type Backend = LazyLoadingBackend<Block>;

#[cfg(not(feature = "forking-support"))]
pub type Backend = polkadot_sdk::sc_client_db::Backend<Block>;

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

        let current_para_block_head = Some(HeadData(current_para_head.encode()));

        let next_block_number =
            UniqueSaturatedInto::<u32>::unique_saturated_into(current_para_head.number) + 1;

        let duration = client.runtime_api().slot_duration(current_para_head.hash()).map_err(|e| {
            ServiceError::Other(format!("retrieving slot duration from runtime: {e}"))
        });
        let slot_duration = match duration {
            Ok(duration) => duration,
            Err(e) => return futures::future::ready(Err(Box::new(e))),
        };

        // The usual paraId for assethub
        let para_id = ParaId::new(1000);

        let next_time = time_manager.next_timestamp();
        let parachain_slot = next_time.saturating_div(slot_duration.as_millis());

        let slot_info = backend.read_relay_slot_info(current_para_head.hash());
        let slot_in_state = match slot_info {
            Ok(slot) => slot.0,
            Err(_) => Slot::from(0), // For starting from genesis
        };

        let last_block_number = backend
            .read_last_relay_chain_block_number(current_para_head.hash())
            .map_err(|e| ServiceError::Other(format!("reading last relay block number: {e}")));
        let last_rc_block_number = last_block_number.unwrap_or_default();

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

        // Read the DMQ MQC head from parachain storage to avoid "DMQ head mismatch" errors
        // The storage key is: twox_128("ParachainSystem") + twox_128("LastDmqMqcHead")
        let pallet_prefix = polkadot_sdk::sp_core::twox_128(b"ParachainSystem");
        let storage_prefix = polkadot_sdk::sp_core::twox_128(b"LastDmqMqcHead");
        let mut dmq_storage_key = Vec::new();
        dmq_storage_key.extend_from_slice(&pallet_prefix);
        dmq_storage_key.extend_from_slice(&storage_prefix);

        // Read the MessageQueueChain from storage and extract its head hash
        use polkadot_sdk::sc_client_api::StorageProvider;
        let dmq_mqc_head = client
            .storage(
                current_para_head.hash(),
                &polkadot_sdk::sc_client_api::StorageKey(dmq_storage_key),
            )
            .ok()
            .flatten()
            .and_then(|encoded_data| {
                // MessageQueueChain is just a wrapper around a Hash, decode it
                // The MessageQueueChain stores the head as the last 32 bytes
                if encoded_data.0.len() >= 32 {
                    let mut hash_bytes = [0u8; 32];
                    hash_bytes.copy_from_slice(&encoded_data.0[encoded_data.0.len() - 32..]);
                    Some(polkadot_sdk::cumulus_primitives_core::relay_chain::Hash::from(hash_bytes))
                } else {
                    None
                }
            })
            .unwrap_or_default(); // Use default (zeros) if we can't read it

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
            xcm_config: polkadot_sdk::cumulus_client_parachain_inherent::MockXcmConfig {
                starting_dmq_mqc_head: dmq_mqc_head,
                ..Default::default()
            },
            ..Default::default()
        };

        let timestamp_provider = sp_timestamp::InherentDataProvider::new(next_time.into());

        futures::future::ready(Ok((timestamp_provider, mocked_parachain)))
    })
}

/// Builds a new service for a full client.
pub async fn new(
    anvil_config: &AnvilNodeConfig,
    mut config: Configuration,
) -> Result<(Service, TaskManager), ServiceError> {
    #[allow(unused_mut)] // mut needed when forking-support feature is enabled
    let mut genesis_block_number = anvil_config.get_genesis_number();

    #[cfg(feature = "forking-support")]
    if let Some(ref fork_url) = anvil_config.eth_rpc_url {
        // Convert HTTP(S) URL to WebSocket URL for Substrate RPC
        // http:// -> ws:// (local/zombienet), https:// -> wss:// (production)
        let ws_url = fork_url.replacen("https://", "wss://", 1).replacen("http://", "ws://", 1);
        let fork_choice = anvil_config.fork_choice;

        let client = subxt::client::OnlineClient::<PolkadotConfig>::from_url(ws_url)
            .await
            .map_err(|e| ServiceError::Other(format!("fork connection failed: {e}")))?;

        let finalized_block_ref = client
            .backend()
            .latest_finalized_block_ref()
            .await
            .map_err(|e| ServiceError::Other(format!("failed to get finalized block: {e}")))?;
        let finalized_head_header = client
            .backend()
            .block_header(finalized_block_ref.hash())
            .await
            .map_err(|e| ServiceError::Other(format!("failed to get block header: {e}")))?
            .ok_or_else(|| ServiceError::Other("finalized block header not found".into()))?;
        let finalized_block_number: u64 = finalized_head_header.number.into();

        // Apply fork_choice if specified
        genesis_block_number = match fork_choice {
            Some(ForkChoice::Block(block_num)) => {
                if block_num < 0 {
                    // Negative offset from latest finalized block
                    let offset = (-block_num) as u64;
                    finalized_block_number.saturating_sub(offset)
                } else {
                    // Specific block number
                    block_num as u64
                }
            }
            None => finalized_block_number,
        };
    }

    #[cfg(not(feature = "forking-support"))]
    if anvil_config.eth_rpc_url.is_some() {
        return Err(ServiceError::Other(
            "Forking is not supported. Compile with 'forking-support' feature to enable forking."
                .into(),
        ));
    }

    let storage_overrides =
        Arc::new(Mutex::new(StorageOverrides::new(anvil_config.revive_rpc_block_limit)));
    let executor = sc_service::new_wasm_executor(&config.executor);

    let (client, backend, keystore, mut task_manager) = client::new_client(
        anvil_config,
        &mut config,
        executor,
        storage_overrides.clone(),
        genesis_block_number,
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

    let aura_digest_provider = SameSlotConsensusDataProvider::new();
    let backend_with_overlay = BackendWithOverlay::new(backend.clone(), storage_overrides.clone());
    let create_inherent_data_providers = create_manual_seal_inherent_data_providers(
        backend_with_overlay,
        client.clone(),
        time_manager,
    );

    let params = sc_consensus_manual_seal::ManualSealParams {
        block_import: client.clone(),
        env: proposer,
        client: client.clone(),
        pool: transaction_pool.clone(),
        select_chain: SelectChain::new(backend.clone()),
        commands_stream: Box::pin(commands_stream),
        consensus_data_provider: Some(Box::new(aura_digest_provider)),
        create_inherent_data_providers,
    };
    let authorship_future = sc_consensus_manual_seal::run_manual_seal(params);

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
