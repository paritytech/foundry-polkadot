use crate::{logging::LoggingManager, substrate_node::service::Service};
use anvil_core::eth::EthRequest;
use anvil_rpc::response::ResponseResult;
use futures::channel::{mpsc, oneshot};
use server::ApiServer;

mod error;
mod server;

pub type ApiHandle = mpsc::Sender<ApiRequest>;

pub struct ApiRequest {
    pub req: EthRequest,
    pub resp_sender: oneshot::Sender<ResponseResult>,
}

pub fn spawn(substrate_service: &Service, logging_manager: LoggingManager) -> ApiHandle {
    let (api_handle, receiver) = mpsc::channel(100);

    let spawn_handle = substrate_service.task_manager.spawn_essential_handle();
    let rpc_handlers = substrate_service.rpc_handlers.clone();
    let mining_engine = substrate_service.mining_engine.clone();
    spawn_handle.spawn("anvil-api-server", "anvil", async move {
        let api_server =
            ApiServer::new(mining_engine, rpc_handlers, receiver, logging_manager).await;
        api_server.run().await;
    });

    api_handle
}
