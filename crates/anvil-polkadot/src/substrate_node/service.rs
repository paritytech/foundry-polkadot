use crate::{
    AnvilNodeConfig,
    genesis::DevelopmentGenesisBlockBuilder,
    substrate_node::mining_engine::{MiningEngine, MiningMode, run_mining_engine},
    substrate_node::mining_engine::{MiningEngine, MiningMode, run_mining_engine},
};
use anvil::eth::backend::time::TimeManager;
use jsonrpsee::RpcModule;
use polkadot_sdk::{
    sc_basic_authorship,
    sc_chain_spec::ChainSpec,
    sc_client_api::{Backend as ClientBackend, HeaderBackend},
    sc_client_db::{BlocksPruning, PruningMode},
    sc_consensus, sc_consensus_manual_seal,
    sc_executor::WasmExecutor,
    sc_network_types::{self, multiaddr::Multiaddr},
    sc_rpc::{
        author::AuthorApiServer,
        chain::ChainApiServer,
        offchain::OffchainApiServer,
        state::{ChildStateApiServer, StateApiServer},
        system::{Request, SystemApiServer, SystemInfo},
    },
    sc_rpc_api::DenyUnsafe,
    sc_rpc_spec_v2::{
        archive::ArchiveApiServer,
        chain_head::ChainHeadApiServer,
        chain_spec::ChainSpecApiServer,
        transaction::{TransactionApiServer, TransactionBroadcastApiServer},
    },
    sc_service::{
        self, Configuration, RpcHandlers, SpawnTaskHandle, TaskManager,
        error::Error as ServiceError,
    },
    sc_telemetry::TelemetryHandle,
    sc_transaction_pool::{self, TransactionPoolWrapper},
    sc_utils::mpsc::{TracingUnboundedSender, tracing_unbounded},
    sp_io,
    sp_keystore::KeystorePtr,
    sp_timestamp,
    substrate_frame_rpc_system::SystemApiServer as _,
};
use std::sync::Arc;
use substrate_runtime::{OpaqueBlock as Block, RuntimeApi};
use tokio_stream::wrappers::ReceiverStream;

pub type FullClient =
    sc_service::TFullClient<Block, RuntimeApi, WasmExecutor<sp_io::SubstrateHostFunctions>>;

pub type Backend = sc_service::TFullBackend<Block>;
pub type TransactionPoolHandle = sc_transaction_pool::TransactionPoolHandle<Block, FullClient>;
type SelectChain = sc_consensus::LongestChain<Backend, Block>;
type TFullParts<TBl, TRtApi, TExec> = (
    sc_service::TFullClient<TBl, TRtApi, TExec>,
    Arc<sc_service::TFullBackend<TBl>>,
    sc_service::KeystoreContainer,
    sc_service::TaskManager,
);

pub struct Service {
    pub task_manager: TaskManager,
    pub client: Arc<FullClient>,
    pub backend: Arc<Backend>,
    pub tx_pool: Arc<TransactionPoolHandle>,
    pub rpc_handlers: RpcHandlers,
    pub mining_engine: Arc<MiningEngine>,
}

/// Create the initial parts of a full node with a customizable genesis block builder.
fn new_full_parts_with_custom_genesis(
    genesis_block_number: u64,
    config: &Configuration,
    telemetry: Option<TelemetryHandle>,
    executor: WasmExecutor<sp_io::SubstrateHostFunctions>,
) -> Result<TFullParts<Block, RuntimeApi, WasmExecutor<sp_io::SubstrateHostFunctions>>, ServiceError>
{
    let backend = sc_service::new_db_backend(config.db_config())?;

    let genesis_block_builder = DevelopmentGenesisBlockBuilder::new(
        genesis_block_number,
        config.chain_spec.as_storage_builder(),
        !config.no_genesis(),
        backend.clone(),
        executor.clone(),
    )?;

    sc_service::new_full_parts_with_genesis_builder(
        config,
        telemetry,
        executor,
        backend,
        genesis_block_builder,
        false,
    )
}

/// Builds a new service for a full client.
pub fn new(anvil_config: &AnvilNodeConfig, config: Configuration) -> Result<Service, ServiceError> {
    let (client, backend, keystore_container, mut task_manager) =
        new_full_parts_with_custom_genesis(
            anvil_config.get_genesis_number(),
            &config,
            None,
            sc_service::new_wasm_executor(&config.executor),
        )?;
    let client = Arc::new(client);

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
        sp_timestamp::Timestamp::from(anvil_config.get_genesis_timestamp()).into(),
    ));
    let mining_engine = Arc::new(MiningEngine::new(
        mining_mode,
        transaction_pool.clone(),
        time_manager.clone(),
        seal_engine_command_sender,
    ));
    let rpc_handlers = spawn_rpc_server(
        anvil_config.get_genesis_number(),
        &mut task_manager,
        client.clone(),
        config,
        transaction_pool.clone(),
        keystore_container.keystore(),
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

    let create_inherent_data_providers = {
        move |_, ()| {
            let next_timestamp = time_manager.next_timestamp();
            async move { Ok(sp_timestamp::InherentDataProvider::new(next_timestamp.into())) }
        }
    };

    let params = sc_consensus_manual_seal::ManualSealParams {
        block_import: client.clone(),
        env: proposer,
        client: client.clone(),
        pool: transaction_pool.clone(),
        select_chain: SelectChain::new(backend.clone()),
        commands_stream: Box::pin(commands_stream),
        consensus_data_provider: None,
        create_inherent_data_providers,
    };
    let authorship_future = sc_consensus_manual_seal::run_manual_seal(params);

    task_manager.spawn_essential_handle().spawn_blocking(
        "manual-seal",
        "substrate",
        authorship_future,
    );

    Ok(Service {
        task_manager,
        client,
        backend,
        tx_pool: transaction_pool,
        rpc_handlers,
        mining_engine,
    })
}

// Re-implement RPC module generation without the check on the genesis block number
fn custom_gen_rpc_module(
    genesis_number: u64,
    spawn_handle: SpawnTaskHandle,
    client: Arc<FullClient>,
    transaction_pool: Arc<TransactionPoolWrapper<Block, FullClient>>,
    keystore: KeystorePtr,
    system_rpc_tx: TracingUnboundedSender<Request<Block>>,
    impl_name: String,
    impl_version: String,
    chain_spec: &dyn ChainSpec,
    state_pruning: &Option<PruningMode>,
    blocks_pruning: BlocksPruning,
    backend: Arc<Backend>,
) -> Result<RpcModule<()>, ServiceError> {
    let rpc_builder = {
        let client = client.clone();
        let pool = transaction_pool.clone();

        Box::new(move |_| {
            let rpc_builder_ext: Result<_, ServiceError> = Ok(
                polkadot_sdk::substrate_frame_rpc_system::System::new(client.clone(), pool.clone())
                    .into_rpc(),
            );
            rpc_builder_ext
        })
    };

    let system_info = SystemInfo {
        chain_name: chain_spec.name().into(),
        impl_name,
        impl_version,
        properties: chain_spec.properties(),
        chain_type: chain_spec.chain_type(),
    };

    let mut rpc_api = RpcModule::new(());
    let task_executor = Arc::new(spawn_handle);

    let (chain, state, child_state) = {
        let chain =
            polkadot_sdk::sc_rpc::chain::new_full(client.clone(), task_executor.clone()).into_rpc();
        let (state, child_state) =
            polkadot_sdk::sc_rpc::state::new_full(client.clone(), task_executor.clone());
        let state = state.into_rpc();
        let child_state = child_state.into_rpc();

        (chain, state, child_state)
    };

    const MAX_TRANSACTION_PER_CONNECTION: usize = 16;

    let transaction_broadcast_rpc_v2 =
        polkadot_sdk::sc_rpc_spec_v2::transaction::TransactionBroadcast::new(
            client.clone(),
            transaction_pool.clone(),
            task_executor.clone(),
            MAX_TRANSACTION_PER_CONNECTION,
        )
        .into_rpc();

    let transaction_v2 = polkadot_sdk::sc_rpc_spec_v2::transaction::Transaction::new(
        client.clone(),
        transaction_pool.clone(),
        task_executor.clone(),
        None,
    )
    .into_rpc();

    let chain_head_v2 = polkadot_sdk::sc_rpc_spec_v2::chain_head::ChainHead::new(
        client.clone(),
        backend.clone(),
        task_executor.clone(),
        // Defaults to sensible limits for the `ChainHead`.
        polkadot_sdk::sc_rpc_spec_v2::chain_head::ChainHeadConfig::default(),
    )
    .into_rpc();

    let is_archive_node = state_pruning.as_ref().map(|sp| sp.is_archive()).unwrap_or(false)
        && blocks_pruning.is_archive();
    let genesis_hash = client.hash(genesis_number as u32).ok().flatten().unwrap();
    if is_archive_node {
        let archive_v2 = polkadot_sdk::sc_rpc_spec_v2::archive::Archive::new(
            client.clone(),
            backend.clone(),
            genesis_hash,
            task_executor.clone(),
        )
        .into_rpc();
        rpc_api.merge(archive_v2).map_err(|e| ServiceError::Application(e.into()))?;
    }

    let chain_spec_v2 = polkadot_sdk::sc_rpc_spec_v2::chain_spec::ChainSpec::new(
        chain_spec.name().into(),
        genesis_hash,
        chain_spec.properties(),
    )
    .into_rpc();

    let author = polkadot_sdk::sc_rpc::author::Author::new(
        client,
        transaction_pool,
        keystore,
        task_executor.clone(),
    )
    .into_rpc();

    let system = polkadot_sdk::sc_rpc::system::System::new(system_info, system_rpc_tx).into_rpc();

    if let Some(storage) = backend.offchain_storage() {
        let offchain = polkadot_sdk::sc_rpc::offchain::Offchain::new(storage).into_rpc();

        rpc_api.merge(offchain).map_err(|e| ServiceError::Application(e.into()))?;
    }

    // Part of the RPC v2 spec.
    rpc_api.merge(transaction_v2).map_err(|e| ServiceError::Application(e.into()))?;
    rpc_api.merge(transaction_broadcast_rpc_v2).map_err(|e| ServiceError::Application(e.into()))?;
    rpc_api.merge(chain_head_v2).map_err(|e| ServiceError::Application(e.into()))?;
    rpc_api.merge(chain_spec_v2).map_err(|e| ServiceError::Application(e.into()))?;

    // Part of the old RPC spec.
    rpc_api.merge(chain).map_err(|e| ServiceError::Application(e.into()))?;
    rpc_api.merge(author).map_err(|e| ServiceError::Application(e.into()))?;
    rpc_api.merge(system).map_err(|e| ServiceError::Application(e.into()))?;
    rpc_api.merge(state).map_err(|e| ServiceError::Application(e.into()))?;
    rpc_api.merge(child_state).map_err(|e| ServiceError::Application(e.into()))?;
    // Additional [`RpcModule`]s defined in the node to fit the specific blockchain
    let extra_rpcs = rpc_builder(task_executor)?;
    rpc_api.merge(extra_rpcs).map_err(|e| ServiceError::Application(e.into()))?;

    Ok(rpc_api)
}

fn spawn_rpc_server(
    genesis_number: u64,
    task_manager: &mut TaskManager,
    client: Arc<FullClient>,
    mut config: Configuration,
    transaction_pool: Arc<TransactionPoolWrapper<Block, FullClient>>,
    keystore: KeystorePtr,
    backend: Arc<Backend>,
) -> Result<RpcHandlers, ServiceError> {
    let (system_rpc_tx, system_rpc_rx) = tracing_unbounded("mpsc_system_rpc", 10_000);

    let rpc_id_provider = config.rpc.id_provider.take();

    let gen_rpc_module = || {
        custom_gen_rpc_module(
            genesis_number,
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool.clone(),
            keystore.clone(),
            system_rpc_tx.clone(),
            config.impl_name.clone(),
            config.impl_version.clone(),
            config.chain_spec.as_ref(),
            &config.state_pruning,
            config.blocks_pruning,
            backend.clone(),
        )
    };

    let rpc_server_handle = sc_service::start_rpc_servers(
        &config.rpc,
        config.prometheus_registry(),
        &config.tokio_handle,
        gen_rpc_module,
        rpc_id_provider,
    )?;

    let listen_addrs = rpc_server_handle
        .listen_addrs()
        .iter()
        .map(|socket_addr| {
            let mut multiaddr: Multiaddr = socket_addr.ip().into();
            multiaddr.push(sc_network_types::multiaddr::Protocol::Tcp(socket_addr.port()));
            multiaddr
        })
        .collect();

    let in_memory_rpc = {
        let mut module = gen_rpc_module()?;
        module.extensions_mut().insert(DenyUnsafe::No);
        module
    };

    let in_memory_rpc_handle = RpcHandlers::new(Arc::new(in_memory_rpc), listen_addrs);

    task_manager.keep_alive((config.base_path, rpc_server_handle, system_rpc_rx));

    Ok(in_memory_rpc_handle)
}
