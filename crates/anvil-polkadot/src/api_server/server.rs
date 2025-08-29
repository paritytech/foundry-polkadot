use super::ApiRequest;
use crate::substrate_node::{
    error::ToRpcResponseResult, mining_engine::MiningEngine, service::Service,
};
use anvil_core::eth::EthRequest;
use anvil_rpc::{error::RpcError, response::ResponseResult};
use foundry_common::sh_println;
use futures::{channel::mpsc, StreamExt};
use std::sync::Arc;

use polkadot_sdk::{sc_consensus_manual_seal::EngineCommand, sp_core};

pub struct ApiServer {
    req_receiver: mpsc::Receiver<ApiRequest>,
    mining_engine: Arc<MiningEngine>,
    seal_command_sender: futures::channel::mpsc::Sender<EngineCommand<sp_core::H256>>,
}

impl ApiServer {
    pub fn new(substrate_service: &Service, req_receiver: mpsc::Receiver<ApiRequest>) -> Self {
        Self {
            req_receiver,
            mining_engine: substrate_service.mining_engine.clone(),
            seal_command_sender: substrate_service.seal_command_sender.clone(),
        }
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
            EthRequest::Mine(blocks, interval) => self
                .mining_engine
                .mine(blocks, interval, self.seal_command_sender.clone())
                .await
                .to_rpc_result(),
            EthRequest::SetIntervalMining(interval) => {
                self.mining_engine.set_interval_mining(interval).to_rpc_result()
            }
            EthRequest::GetIntervalMining(()) => {
                self.mining_engine.get_interval_mining().to_rpc_result()
            }
            EthRequest::GetAutoMine(()) => self.mining_engine.get_auto_mine().to_rpc_result(),
            EthRequest::SetAutomine(enabled) => {
                self.mining_engine.set_auto_mine(enabled).to_rpc_result()
            }
            _ => ResponseResult::Error(RpcError::internal_error()),
        }
    }
}
