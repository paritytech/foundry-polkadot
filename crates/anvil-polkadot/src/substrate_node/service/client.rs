use crate::{
    AnvilNodeConfig,
    config::ForkChoice,
    substrate_node::{
        genesis::DevelopmentGenesisBlockBuilder,
        lazy_loading::{
            backend::new_backend as new_lazy_loading_backend,
            rpc_client::{RPCClient, Rpc},
        },
        service::{
            Backend,
            backend::StorageOverrides,
            executor::{Executor, WasmExecutor},
        },
    },
};
use parking_lot::Mutex;
use polkadot_sdk::{
    parachains_common::opaque::Block,
    sc_chain_spec::{NoExtension, get_extension},
    sc_client_api::{BadBlocks, ForkBlocks, execution_extensions::ExecutionExtensions},
    sc_service::{
        self, ChainType, GenericChainSpec, KeystoreContainer, LocalCallExecutor, TaskManager,
    },
    sp_blockchain,
    sp_core::storage::well_known_keys::CODE,
    sp_keystore::KeystorePtr,
    sp_runtime::{
        generic::SignedBlock,
        traits::{Block as BlockT, Header as HeaderT},
    },
    sp_storage::StorageKey,
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use substrate_runtime::RuntimeApi;

pub type Client = sc_service::client::Client<Backend, Executor, Block, RuntimeApi>;

pub fn new_client(
    anvil_config: &AnvilNodeConfig,
    config: &mut sc_service::Configuration,
    executor: WasmExecutor,
    storage_overrides: Arc<Mutex<StorageOverrides>>,
    genesis_num: u64,
) -> Result<(Arc<Client>, Arc<Backend>, KeystorePtr, TaskManager), sc_service::error::Error> {
     println!("genesis_num {}", genesis_num);
    let fork_config: Option<(Arc<dyn RPCClient<Block>>, Block)> =
        if let Some(fork_url) = &anvil_config.eth_rpc_url {
            let (rpc_client, checkpoint_block) = setup_fork(anvil_config, config, fork_url, genesis_num)?;
            Some((rpc_client, checkpoint_block))
        } else {
            None
        };


    let backend = new_lazy_loading_backend(fork_config.clone())?;

    // In fork mode, use the checkpoint block as genesis
    // In normal mode, create a new genesis block
    let genesis_block_builder = if let Some((_, checkpoint)) = &fork_config {
        println!("checkpoint {}", checkpoint.header.number);
        // Fork mode: use checkpoint block as genesis
        DevelopmentGenesisBlockBuilder::new_with_checkpoint(
            config.chain_spec.as_storage_builder(),
            !config.no_genesis(),
            backend.clone(),
            executor.clone(),
            checkpoint.clone(),
        )?
    } else {
        // Normal mode: create standard genesis
        DevelopmentGenesisBlockBuilder::new(
            anvil_config.get_genesis_number(),
            config.chain_spec.as_storage_builder(),
            !config.no_genesis(),
            backend.clone(),
            executor.clone(),
        )?
    };

    let keystore_container = KeystoreContainer::new(&config.keystore)?;

    let task_manager = {
        let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
        TaskManager::new(config.tokio_handle.clone(), registry)?
    };

    let chain_spec = &config.chain_spec;
    let fork_blocks =
        get_extension::<ForkBlocks<Block>>(chain_spec.extensions()).cloned().unwrap_or_default();

    let bad_blocks =
        get_extension::<BadBlocks<Block>>(chain_spec.extensions()).cloned().unwrap_or_default();

    let execution_extensions = ExecutionExtensions::new(None, Arc::new(executor.clone()));

    let wasm_runtime_substitutes = HashMap::new();

    let client = {
        let client_config = sc_service::ClientConfig {
            offchain_worker_enabled: config.offchain_worker.enabled,
            offchain_indexing_api: config.offchain_worker.indexing_enabled,
            wasm_runtime_overrides: config.wasm_runtime_overrides.clone(),
            no_genesis: config.no_genesis(),
            wasm_runtime_substitutes,
            enable_import_proof_recording: false,
        };
        let inner_executor = LocalCallExecutor::new(
            backend.clone(),
            executor,
            client_config.clone(),
            execution_extensions,
        )?;
        let executor = Executor::new(inner_executor, storage_overrides, backend.clone());

        Client::new(
            backend.clone(),
            executor,
            Box::new(task_manager.spawn_handle()),
            genesis_block_builder,
            fork_blocks,
            bad_blocks,
            None,
            None,
            client_config,
        )?
    };

    Ok((Arc::new(client), backend, keystore_container.keystore(), task_manager))
}

/// Resolves the block number to fork from, handling both positive and negative block numbers.
/// Negative numbers are subtracted from the latest block number.
fn resolve_fork_block_number(
    rpc_client: &Rpc<Block>,
    fork_choice: &ForkChoice,
) -> Result<u32, sp_blockchain::Error> {
    match fork_choice {
        ForkChoice::Block(block_number) => {
            if *block_number < 0 {
                // Get the latest block from the chain header
                let latest_header = rpc_client
                    .header(None)
                    .map_err(|e| {
                        sp_blockchain::Error::Backend(format!("failed to fetch latest header: {e}"))
                    })?
                    .ok_or_else(|| {
                        sp_blockchain::Error::Backend("latest header not found".into())
                    })?;

                let latest_number: u32 = *latest_header.number();

                let offset: u32 = block_number.abs().try_into().map_err(|_| {
                    sp_blockchain::Error::Backend(format!(
                        "Block number offset too large: {block_number}"
                    ))
                })?;

                Ok(latest_number.saturating_sub(offset))
            } else {
                (*block_number).try_into().map_err(|_| {
                    sp_blockchain::Error::Backend(format!(
                        "Invalid fork block number: {block_number}"
                    ))
                })
            }
        }
    }
}

/// Fetches the checkpoint block and sets up the chain spec for forking
fn setup_fork(
    anvil_config: &AnvilNodeConfig,
    config: &mut sc_service::Configuration,
    fork_url: &str,
    genesis_num: u64,
) -> Result<(Arc<dyn RPCClient<Block>>, Block), sc_service::error::Error> {
    let http_client = jsonrpsee::http_client::HttpClientBuilder::default()
        .max_request_size(u32::MAX)
        .max_response_size(u32::MAX)
        .request_timeout(Duration::from_secs(10))
        .build(fork_url)
        .map_err(|e| sp_blockchain::Error::Backend(format!("failed to build http client: {e}")))?;

    let rpc_client = Rpc::<Block>::new(http_client, 0);

    // Create new chainspec for the forked chain
    let chain_name = rpc_client
        .system_chain()
        .map_err(|e| sp_blockchain::Error::Backend(format!("failed to fetch system_chain: {e}")))?;

    let chain_properties = rpc_client.system_properties().map_err(|e| {
        sp_blockchain::Error::Backend(format!("failed to fetch system_properties: {e}"))
    })?;

     // TODO refactor this to account for fork choice
    // Get block hash from fork_choice config
    // If no fork_choice is specified, we need to fetch the latest block and use its hash
    // for all subsequent requests to avoid inconsistencies if a new block is mined between calls.
    // let block_hash: <Block as BlockT>::Hash = if let Some(fork_choice) = &anvil_config.fork_choice {
    //    // let block_num = resolve_fork_block_number(&rpc_client, fork_choice)?;
    //     let block_num = genesis_num as u32;
    //     rpc_client
    //         .block_hash(Some(block_num))
    //         .map_err(|e| {
    //             sp_blockchain::Error::Backend(format!(
    //                 "failed to fetch block hash for block {block_num}: {e}"
    //             ))
    //         })?
    //         .ok_or_else(|| {
    //             sp_blockchain::Error::Backend(format!("block hash not found for block {block_num}"))
    //         })?
    // } else {
    //     // No fork_choice specified, fetch the latest block header and use its hash
    //     let latest_header = rpc_client
    //         .header(None)
    //         .map_err(|e| {
    //             sp_blockchain::Error::Backend(format!(
    //                 "failed to fetch latest header for fork: {e}"
    //             ))
    //         })?
    //         .ok_or_else(|| {
    //             sp_blockchain::Error::Backend("latest header not found for fork".into())
    //         })?;
    //     latest_header.hash()
    // };

    let block_num = genesis_num as u32;
    let block_hash = rpc_client
            .block_hash(Some(block_num))
            .map_err(|e| {
                sp_blockchain::Error::Backend(format!(
                    "failed to fetch block hash for block {block_num}: {e}"
                ))
            })?
            .ok_or_else(|| {
                sp_blockchain::Error::Backend(format!("block hash not found for block {block_num}"))
            })?;

    let wasm_binary = rpc_client
        .storage(StorageKey(CODE.to_vec()), Some(block_hash))
        .map_err(|e| sp_blockchain::Error::Backend(format!("failed to fetch runtime code: {e}")))?
        .map(|data| data.0)
        .expect("WASM binary not found in forked chain");

    let chain_spec = GenericChainSpec::<NoExtension, ()>::builder(&wasm_binary, None)
        .with_name(chain_name.as_str())
        .with_id("lazy_loading")
        .with_properties(chain_properties)
        .with_chain_type(ChainType::Development)
        .build();

    config.chain_spec = Box::new(chain_spec);

    let checkpoint: SignedBlock<Block> = rpc_client
        .block(Some(block_hash))
        .map_err(|e| {
            sp_blockchain::Error::Backend(format!("failed to fetch checkpoint block: {e}"))
        })?
        .ok_or_else(|| sp_blockchain::Error::Backend("fork checkpoint not found".into()))?;

    Ok((Arc::new(rpc_client), checkpoint.block))
}
