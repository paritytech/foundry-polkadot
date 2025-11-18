use crate::{
    AnvilNodeConfig,
    api_server::filters::Filters,
    logging::LoggingManager,
    substrate_node::{
        impersonation::ImpersonationManager, revert::RevertManager, service::Service,
    },
};
use anvil_core::eth::EthRequest;
use anvil_rpc::response::ResponseResult;
use futures::channel::{mpsc, oneshot};
use server::ApiServer;
use subxt_signer::eth::Keypair;

pub mod error;
pub mod filters;
pub mod revive_conversions;
mod server;
mod signer;
pub mod trace_helpers;
mod txpool_helpers;

pub use txpool_helpers::TxpoolTransactionInfo;

pub type ApiHandle = mpsc::Sender<ApiRequest>;

pub struct ApiRequest {
    pub req: EthRequest,
    pub resp_sender: oneshot::Sender<ResponseResult>,
}

pub fn spawn(
    config: &AnvilNodeConfig,
    substrate_service: &Service,
    logging_manager: LoggingManager,
    revert_manager: RevertManager,
    filters: Filters,
) -> ApiHandle {
    let (api_handle, receiver) = mpsc::channel(100);

    let service = substrate_service.clone();
    let mut impersonation_manager = ImpersonationManager::default();
    impersonation_manager.set_auto_impersonate_account(config.enable_auto_impersonate);
    let mut signers = config.signer_accounts.clone();
    signers.extend(config.genesis.iter().flat_map(|genesis| genesis.alloc.values()).filter_map(
        |acc| {
            let private_key = acc.private_key?;
            Keypair::from_secret_key(*private_key).ok()
        },
    ));
    let revive_rpc_block_limit = config.revive_rpc_block_limit;
    substrate_service.spawn_handle.spawn("anvil-api-server", "anvil", async move {
        let api_server = ApiServer::new(
            service,
            receiver,
            logging_manager,
            revert_manager,
            impersonation_manager,
            signers,
            filters,
            revive_rpc_block_limit,
        )
        .await
        .unwrap_or_else(|err| panic!("Failed to spawn the API server: {err}"));
        api_server.run().await;
    });

    api_handle
}
