use super::ApiRequest;
use crate::{logging::LoggingManager, substrate_node::service::Service};
use anvil_core::eth::EthRequest;
use anvil_rpc::{error::RpcError, response::ResponseResult};
use foundry_common::sh_println;
use futures::{channel::mpsc, StreamExt};

pub struct ApiServer {
    req_receiver: mpsc::Receiver<ApiRequest>,
    logging_manager: LoggingManager,
}

impl ApiServer {
    pub fn new(
        _substrate_service: &Service,
        req_receiver: mpsc::Receiver<ApiRequest>,
        logging_manager: LoggingManager,
    ) -> Self {
        Self { req_receiver, logging_manager }
    }

    pub async fn run(mut self) {
        while let Some(msg) = self.req_receiver.next().await {
            sh_println!("GOT REQUEST: {:?}", msg.req).unwrap();

            let resp = self.execute(msg.req).await;

            msg.resp_sender.send(resp).expect("Dropped receiver");
        }
    }

    pub async fn execute(&mut self, req: EthRequest) -> ResponseResult {
        match req {
            EthRequest::SetLogging(enabled) => {
                sh_println!("anvil_setLoggingEnabled called with enabled = {}", enabled).unwrap();

                // Update the logging manager state
                self.logging_manager.set_enabled(enabled);

                // Log the state change using the appropriate targets
                tracing::warn!(target: "node::user", "anvil_setLoggingEnabled logging set to {}", enabled);
                tracing::warn!(target: "node::console", "Console logging enabled = {}", enabled);

                ResponseResult::Success(serde_json::Value::Bool(true))
            }
            _ => ResponseResult::Error(RpcError::internal_error()),
        }
    }
}
