use anvil_rpc::{error::RpcError, response::ResponseResult};
use polkadot_sdk::sc_consensus_manual_seal::Error as BlockProducingError;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Block production failed: {0:?}")]
    BlockProducing(BlockProducingError),
    #[error("Current mining mode can not answer this query.")]
    MiningModeMismatch,
    #[error("Current timestamp is newer.")]
    Timestamp,
}

impl From<polkadot_sdk::sc_consensus_manual_seal::Error> for Error {
    fn from(err: polkadot_sdk::sc_consensus_manual_seal::Error) -> Self {
        Self::BlockProducing(err)
    }
}

pub(crate) trait ToRpcResponseResult {
    fn to_rpc_result(self) -> ResponseResult;
}

/// Converts a serializable value into a `ResponseResult`
pub fn to_rpc_result<T: Serialize>(val: T) -> ResponseResult {
    match serde_json::to_value(val) {
        Ok(success) => ResponseResult::Success(success),
        Err(err) => {
            error!(%err, "Failed serialize rpc response");
            ResponseResult::error(RpcError::internal_error())
        }
    }
}

impl<T: Serialize> ToRpcResponseResult for Result<T, Error> {
    fn to_rpc_result(self) -> ResponseResult {
        match self {
            Ok(val) => to_rpc_result(val),
            Err(err) => match err {
                Error::BlockProducing(block_error) => RpcError::internal_error_with(format!(
                    "Block production failed: {block_error:?}"
                )),
                Error::MiningModeMismatch => {
                    RpcError::invalid_params("Current mining mode can not answer this query.")
                }
                Error::Timestamp => RpcError::invalid_params(
                    "Timestamp parameter is older than the timestamp of the last produced block.",
                ),
            }
            .into(),
        }
    }
}
