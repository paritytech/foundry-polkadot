use std::time::Duration;

use crate::substrate_node::{mining_engine::MiningError, service::BackendError};
use alloy_primitives::{B256, hex};
use anvil_rpc::{
    error::{ErrorCode, RpcError},
    response::ResponseResult,
};
use pallet_revive_eth_rpc::{EthRpcError, client::ClientError};
use polkadot_sdk::{
    pallet_revive::{EthTransactError, evm::decode_revert_reason},
    sp_api,
};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Block mining failed: {0}")]
    Mining(#[from] MiningError),
    #[error("Rpc Endpoint not implemented")]
    RpcUnimplemented,
    #[error("Invalid params: {0}")]
    InvalidParams(String),
    #[error("Revive call failed: {0}")]
    ReviveRpc(#[from] EthRpcError),
    #[error(transparent)]
    Backend(#[from] BackendError),
    #[error("Nonce overflowing the substrate nonce type")]
    NonceOverflow,
    #[error(transparent)]
    RuntimeApi(#[from] sp_api::ApiError),
    #[error("Error encountered while creating a BalanceWithDust from a U256 balance")]
    BalanceConversion,
    #[error("Internal error: {0}")]
    InternalError(String),
    #[error("No signer available")]
    NoSignerAvailable,
    #[error("Contract reverted: {0:?}")]
    EthTransact(EthTransactError),
    #[error("Transaction {hash} was added to the mempool but wasn't confirmed within {duration:?}")]
    TransactionConfirmationTimeout { hash: B256, duration: Duration },
}

impl From<subxt::Error> for Error {
    fn from(err: subxt::Error) -> Self {
        Self::ReviveRpc(EthRpcError::ClientError(err.into()))
    }
}

impl From<ClientError> for Error {
    fn from(err: ClientError) -> Self {
        match err {
            ClientError::TransactError(err) => Self::EthTransact(err),
            err => Self::ReviveRpc(EthRpcError::ClientError(err)),
        }
    }
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
            Err(err) => match err {
                Error::Mining(mining_error) => match mining_error {
                    MiningError::BlockProducing(error) => {
                        RpcError::internal_error_with(format!("Failed to produce a block: {error}"))
                            .into()
                    }
                    MiningError::MiningModeMismatch => {
                        RpcError::invalid_params("Current mining mode can not answer this query.")
                            .into()
                    }
                    MiningError::Timestamp => {
                        RpcError::invalid_params("Current timestamp is newer.").into()
                    }
                    MiningError::ClosedChannel => {
                        RpcError::internal_error_with("Communication channel was dropped.").into()
                    }
                },
                Error::RpcUnimplemented => RpcError::internal_error_with("Not implemented").into(),
                Error::InvalidParams(error_message) => {
                    RpcError::invalid_params(error_message).into()
                }
                Error::EthTransact(EthTransactError::Data(data)) => {
                    let message = match decode_revert_reason(&data) {
                        Some(reason) => format!("execution reverted: {reason}"),
                        None => "execution reverted".to_string(),
                    };

                    let data = format!("0x{}", hex::encode(data));
                    RpcError {
                        code: ErrorCode::ExecutionError,
                        message: message.into(),
                        data: Some(data.into()),
                    }
                    .into()
                }
                Error::EthTransact(EthTransactError::Message(message)) => {
                    let message = format!("execution reverted: {message}");

                    RpcError {
                        // This is the error code that revive eth rpc uses for this error.
                        code: ErrorCode::ServerError(-32000),
                        message: message.into(),
                        data: None,
                    }
                    .into()
                }
                err => RpcError::internal_error_with(format!("{err}")).into(),
            },
        }
    }
}
