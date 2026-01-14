use crate::api_server::{error::Error, revive_conversions::AlloyU256};
use alloy_primitives::{Address, B256};
use alloy_rpc_types::trace::parity::{
    Action, CallAction, CallOutput, CallType, CreateAction, CreateOutput, CreationMethod,
    LocalizedTransactionTrace, SelfdestructAction, TraceOutput, TransactionTrace,
};
use pallet_revive_eth_rpc::EthRpcError;
use polkadot_sdk::pallet_revive::evm::{
    Block, CallTrace, CallType as ReviveCallType, HashesOrTransactionInfos, Trace, TransactionInfo,
    TransactionTrace as ReviveTransactionTrace,
};

/// Builds a Parity block trace from a vector of TransactionTrace objects returned by the
/// debug_traceBlockByNumber endpoint of pallet revive and a Substrate Block object recovered from
/// pallet revive. The block must be "hydrated" with all the transactions details.
/// This is used to build the output for Parity client's RPC method `trace_block`.
pub fn parity_block_trace_builder(
    traces: Vec<ReviveTransactionTrace>,
    block: Block,
) -> Result<Vec<LocalizedTransactionTrace>, Error> {
    let mut parity_block_traces = Vec::new();
    let HashesOrTransactionInfos::TransactionInfos(transaction_infos) = block.transactions else {
        return Err(Error::InternalError(
            "Block transactions infos are not available in the block".to_string(),
        ));
    };
    for revive_transaction_trace in traces {
        let tx_info = transaction_infos
            .iter()
            .find(|item| item.hash == revive_transaction_trace.tx_hash)
            .ok_or(Error::InternalError("Transaction info not found".to_string()))?;
        let parity_transaction_trace = parity_transaction_trace_builder(
            revive_transaction_trace.trace,
            Some(tx_info.clone()),
        )?;
        parity_block_traces.extend(parity_transaction_trace);
    }
    Ok(parity_block_traces)
}

/// Builds a Parity transaction trace from a Trace object returned by the debug_traceTransaction
/// endpoint of pallet revive and a TransactionInfo object recovered from pallet revive.
/// This is used to build the output for Parity client's RPC method `trace_transaction`.
pub fn parity_transaction_trace_builder(
    trace: Trace,
    tx_info: Option<TransactionInfo>,
) -> Result<Vec<LocalizedTransactionTrace>, Error> {
    let call_trace = match trace {
        Trace::Call(call_trace) => call_trace,
        Trace::Prestate(_) => {
            return Err(Error::InternalError("Trace is not a call trace".to_string()));
        }
    };
    let mut parity_tx_traces = Vec::new();
    let mut next_traces = vec![(vec![], call_trace)];
    while let Some((trace_address, trace)) = next_traces.pop() {
        let transaction_trace =
            parity_transaction_trace_from_call_trace(trace.clone(), trace_address.clone())?;
        let localized_trace = LocalizedTransactionTrace {
            trace: transaction_trace,
            block_hash: tx_info
                .as_ref()
                .map(|tx_info| B256::from_slice(tx_info.block_hash.as_ref())),
            block_number: tx_info
                .as_ref()
                .map(|tx_info| tx_info.block_number.try_into().unwrap_or_default()),
            transaction_hash: tx_info
                .as_ref()
                .map(|tx_info| B256::from_slice(tx_info.hash.as_ref())),
            transaction_position: tx_info
                .as_ref()
                .map(|tx_info| tx_info.transaction_index.try_into().unwrap_or_default()),
        };
        parity_tx_traces.push(localized_trace);
        for (call_index, call) in trace.calls.iter().enumerate() {
            let mut new_trace_address = trace_address.clone();
            new_trace_address.push(call_index);
            next_traces.push((new_trace_address, call.clone()));
        }
    }
    Ok(parity_tx_traces)
}

/// Builds a Parity TransactionTrace from a CallTrace from pallet revive and a trace address,
/// which is built with the indices of the path from the transaction root callto the specific call
/// in the call tree.
fn parity_transaction_trace_from_call_trace(
    trace: CallTrace,
    trace_address: Vec<usize>,
) -> Result<TransactionTrace, Error> {
    match trace.call_type {
        ReviveCallType::Call | ReviveCallType::StaticCall | ReviveCallType::DelegateCall => {
            Ok(TransactionTrace {
                action: Action::Call(CallAction {
                    from: Address::from_slice(trace.from.as_ref()),
                    call_type: CallType::Call,
                    gas: trace.gas.try_into().map_err(|_| EthRpcError::ConversionError)?,
                    input: trace.input.0.into(),
                    to: Address::from_slice(trace.to.as_ref()),
                    value: AlloyU256::from(trace.value.unwrap_or_default()).inner(),
                }),
                error: trace.error,
                result: Some(TraceOutput::Call(CallOutput {
                    gas_used: trace
                        .gas_used
                        .try_into()
                        .map_err(|_| EthRpcError::ConversionError)?,
                    output: trace.output.0.into(),
                })),
                subtraces: trace
                    .child_call_count
                    .try_into()
                    .map_err(|_| EthRpcError::ConversionError)?,
                trace_address,
            })
        }
        ReviveCallType::Create | ReviveCallType::Create2 => {
            let creation_method = match trace.call_type {
                ReviveCallType::Create => CreationMethod::Create,
                ReviveCallType::Create2 => CreationMethod::Create2,
                _ => unreachable!("Unexpected call type: should be Create or Create2"),
            };
            Ok(TransactionTrace {
                action: Action::Create(CreateAction {
                    from: Address::from_slice(trace.from.as_ref()),
                    gas: trace.gas.try_into().map_err(|_| EthRpcError::ConversionError)?,
                    init: trace.input.0.into(),
                    value: AlloyU256::from(trace.value.unwrap_or_default()).inner(),
                    creation_method,
                }),
                error: trace.error,
                result: Some(TraceOutput::Create(CreateOutput {
                    address: Address::from_slice(trace.to.as_ref()),
                    code: Default::default(),
                    gas_used: trace
                        .gas_used
                        .try_into()
                        .map_err(|_| EthRpcError::ConversionError)?,
                })),
                subtraces: trace
                    .child_call_count
                    .try_into()
                    .map_err(|_| EthRpcError::ConversionError)?,
                trace_address,
            })
        }
        ReviveCallType::Selfdestruct => Ok(TransactionTrace {
            action: Action::Selfdestruct(SelfdestructAction {
                address: Address::from_slice(trace.from.as_ref()),
                balance: AlloyU256::from(trace.value.unwrap_or_default()).inner(),
                refund_address: Address::from_slice(trace.to.as_ref()),
            }),
            error: trace.error,
            result: None,
            subtraces: trace
                .child_call_count
                .try_into()
                .map_err(|_| EthRpcError::ConversionError)?,
            trace_address,
        }),
    }
}
