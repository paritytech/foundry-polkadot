use crate::{logging::LoggingManager, substrate_node::service::Service};
use anvil_core::eth::EthRequest;
use anvil_rpc::response::ResponseResult;
use futures::channel::{mpsc, oneshot};
use polkadot_sdk::sc_service::TaskManager;
use server::ApiServer;

mod error;
pub mod revive_conversions;
mod server;

pub type ApiHandle = mpsc::Sender<ApiRequest>;

pub struct ApiRequest {
    pub req: EthRequest,
    pub resp_sender: oneshot::Sender<ResponseResult>,
}

pub fn spawn(
    substrate_service: &Service,
    task_manager: &TaskManager,
    logging_manager: LoggingManager,
) -> ApiHandle {
    let (api_handle, receiver) = mpsc::channel(100);

    let spawn_handle = task_manager.spawn_essential_handle();
    let service = substrate_service.clone();
    spawn_handle.spawn("anvil-api-server", "anvil", async move {
        let api_server = ApiServer::new(service, receiver, logging_manager).await;
        api_server.run().await;
    });

    api_handle
}
