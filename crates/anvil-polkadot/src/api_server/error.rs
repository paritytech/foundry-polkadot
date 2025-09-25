use crate::substrate_node::mining_engine::MiningError;
use anvil_rpc::{error::RpcError, response::ResponseResult};
use polkadot_sdk::pallet_revive_eth_rpc::{EthRpcError, client::ClientError};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Block mining failed: {0}")]
    Mining(#[from] MiningError),
    #[error("Rpc Endpoint not implemented")]
    RpcUnimplemented,
    #[error("Invalid params: {0}")]
    InvalidParams(String),
    #[error("Revive call failed: {0:?}")]
    Revive(ClientError),
    #[error("ETH RPC ERROR {0:?}")]
    EthRpc(EthRpcError),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Helper trait to easily convert results to rpc results
pub(crate) trait ToRpcResponseResult {
    fn to_rpc_result(self) -> ResponseResult;
}

/// Converts a serializable value into a `ResponseResult`.
fn to_rpc_result<T: Serialize>(val: T) -> ResponseResult {
    match serde_json::to_value(val) {
        Ok(success) => ResponseResult::Success(success),
        Err(err) => {
            error!(%err, "Failed serialize rpc response");
            ResponseResult::error(RpcError::internal_error())
        }
    }
}

impl<T: Serialize> ToRpcResponseResult for Result<T> {
    fn to_rpc_result(self) -> ResponseResult {
        match self {
            Ok(val) => to_rpc_result(val),
            Err(err) => RpcError::internal_error_with(err.to_string()).into(),
        }
    }
}
