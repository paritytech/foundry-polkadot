use super::ApiRequest;
use crate::substrate_node::{mining_engine::MiningEngine, service::Service};
use anvil_core::eth::EthRequest;
use anvil_rpc::{error::RpcError, response::ResponseResult};
use foundry_common::sh_println;
use futures::{channel::mpsc, StreamExt};
use std::sync::Arc;

pub struct ApiServer {
    req_receiver: mpsc::Receiver<ApiRequest>,
    mining_engine: Arc<MiningEngine>,
}

impl ApiServer {
    pub fn new(substrate_service: &Service, req_receiver: mpsc::Receiver<ApiRequest>) -> Self {
        Self { req_receiver, mining_engine: substrate_service.mining_engine.clone() }
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
            EthRequest::Mine(blocks, interval) => self.mining_engine.mine(blocks, interval).await,
            EthRequest::SetIntervalMining(interval) => {
                self.mining_engine.set_interval_mining(interval)
            }
            EthRequest::GetIntervalMining(()) => self.mining_engine.get_interval_mining(),
            EthRequest::GetAutoMine(()) => self.mining_engine.get_auto_mine(),
            EthRequest::SetAutomine(enabled) => self.mining_engine.set_auto_mine(enabled),
            _ => ResponseResult::Error(RpcError::internal_error()),
        }
    }
}
