use crate::{
    abi::SimpleStorageCaller::{self as SimpleStorageCaller},
    utils::{
        TestNode, get_contract_code, get_contract_code_with_args, is_transaction_in_block,
        unwrap_response,
    },
};
use alloy_dyn_abi::DynSolValue;
use alloy_primitives::{Address, B256, Bytes, U256};
use alloy_rpc_types::{
    TransactionInput, TransactionRequest,
    trace::{
        geth::{
            GethDebugBuiltInTracerType, GethDebugTracerType, GethDebugTracingCallOptions,
            GethDebugTracingOptions, GethTrace,
        },
        parity::{
            Action as ParityAction, CallAction as ParityCallAction, CallType as ParityCallType,
            LocalizedTransactionTrace, TraceOutput as ParityTraceOutput,
        },
    },
};
use alloy_serde::WithOtherFields;
use alloy_sol_types::SolCall;
use anvil_core::eth::EthRequest;
use anvil_polkadot::{
    api_server::revive_conversions::ReviveAddress,
    config::{AnvilNodeConfig, SubstrateNodeConfig},
};
use polkadot_sdk::pallet_revive::evm::Account;

#[tokio::test(flavor = "multi_thread")]
async fn test_traces() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Simple value transfer transaction
    let alith = Account::from(subxt_signer::eth::dev::alith());
    let baltathar = Account::from(subxt_signer::eth::dev::baltathar());
    let from = Address::from(ReviveAddress::new(alith.address()));
    let to = Address::from(ReviveAddress::new(baltathar.address()));
    let value = U256::from(1e18);

    let tx = TransactionRequest::default().from(from).to(to).value(value);
    let tx_hash = node.send_transaction(tx.clone()).await.unwrap();
    // Ensure the tx is mined
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    // debug_traceTransaction should return a CallTracer frame matching the tx
    let debug_resp = node
        .eth_rpc(EthRequest::DebugTraceTransaction(
            B256::from_slice(tx_hash.as_ref()),
            GethDebugTracingOptions::default(),
        ))
        .await
        .unwrap();
    let geth_trace: GethTrace = unwrap_response(debug_resp).unwrap();
    let call_frame = match geth_trace {
        GethTrace::CallTracer(frame) => frame,
        other => panic!("expected CallTracer trace, got {other:?}"),
    };

    assert_eq!(call_frame.from, from);
    assert_eq!(call_frame.to, Some(to));
    assert!(call_frame.input.is_empty());
    // Output is Some(0x) because the call is successful but without any meaningful output.
    assert!(call_frame.output.is_some());
    assert!(call_frame.output.unwrap().is_empty());
    assert!(call_frame.calls.is_empty());
    assert!(call_frame.error.is_none());
    assert!(call_frame.revert_reason.is_none());
    assert_eq!(call_frame.value, Some(value));

    // trace_transaction should return a single parity trace matching the same call
    let trace_resp = node
        .eth_rpc(EthRequest::TraceTransaction(B256::from_slice(tx_hash.as_ref())))
        .await
        .unwrap();
    let parity_traces: Vec<LocalizedTransactionTrace> = unwrap_response(trace_resp).unwrap();
    assert_eq!(parity_traces.len(), 1);
    let localized_trace = &parity_traces[0];

    // Basic metadata
    assert_eq!(localized_trace.transaction_hash, Some(B256::from_slice(tx_hash.as_ref())));
    assert_eq!(localized_trace.block_number, Some(1u64));
    let block_hash = node.block_hash_by_number(1).await.unwrap();
    let block = node.get_block_by_hash(block_hash).await;
    assert_eq!(localized_trace.block_hash, Some(B256::from_slice(block.hash.as_ref())));
    assert_eq!(localized_trace.transaction_position, Some(1u64));

    let transaction_trace = localized_trace.trace.clone();

    // Inner TransactionTrace action & result
    match &transaction_trace.action {
        ParityAction::Call(ParityCallAction {
            from: act_from,
            to: act_to,
            value: act_value,
            call_type: act_call_type,
            ..
        }) => {
            assert_eq!(*act_from, from);
            assert_eq!(*act_to, to);
            assert_eq!(*act_value, value);
            assert_eq!(*act_call_type, ParityCallType::Call);
        }
        other => panic!("expected parity Call action, got {other:?}"),
    }

    assert!(transaction_trace.error.is_none());
    match &transaction_trace.result {
        Some(ParityTraceOutput::Call(call_out)) => {
            assert!(call_out.output.is_empty());
        }
        other => panic!("expected parity Call result, got {other:?}"),
    }
    // No nested subtraces for simple transfer
    assert_eq!(transaction_trace.subtraces, 0);
    assert!(transaction_trace.trace_address.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_trace_block() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let baltathar = Account::from(subxt_signer::eth::dev::baltathar());
    let dorothy = Account::from(subxt_signer::eth::dev::dorothy());

    let alith_addr = Address::from(ReviveAddress::new(alith.address()));
    let baltathar_addr = Address::from(ReviveAddress::new(baltathar.address()));
    let dorothy_addr = Address::from(ReviveAddress::new(dorothy.address()));

    let value_0 = U256::from(1e18);
    let value_1 = U256::from(2e18);
    let value_2 = U256::from(3e18);

    // Queue three different transfer transactions in the same block:
    //  - alith -> baltathar
    //  - alith -> dorothy
    //  - baltathar -> alith
    let tx_0 = TransactionRequest::default().from(alith_addr).to(baltathar_addr).value(value_0);
    let tx_1 = TransactionRequest::default().from(alith_addr).to(dorothy_addr).value(value_1);
    let tx_2 = TransactionRequest::default().from(baltathar_addr).to(alith_addr).value(value_2);

    let tx_hash_0 = node.send_transaction(tx_0).await.unwrap();
    let tx_hash_1 = node.send_transaction(tx_1.nonce(1)).await.unwrap();
    let tx_hash_2 = node.send_transaction(tx_2).await.unwrap();

    // Mine a single block including all three transactions.
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    // Sanity check: all three transactions should be mined in block 1.
    let block1 = node.get_block_by_hash(node.block_hash_by_number(1).await.unwrap()).await;
    assert!(is_transaction_in_block(&block1.transactions, tx_hash_0));
    assert!(is_transaction_in_block(&block1.transactions, tx_hash_1));
    assert!(is_transaction_in_block(&block1.transactions, tx_hash_2));

    // trace_block for block 1 should return three traces, one per transaction.
    let block_traces: Vec<LocalizedTransactionTrace> = unwrap_response(
        node.eth_rpc(EthRequest::TraceBlock(alloy_eips::BlockNumberOrTag::Number(1)))
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(block_traces.len(), 3, "expected three traces for the three transfers in block 1");

    // Collect the transaction hashes present in the block traces.
    let mut traced_hashes: Vec<B256> =
        block_traces.iter().filter_map(|t| t.transaction_hash).collect();
    traced_hashes.sort();

    let mut expected_hashes = vec![
        B256::from_slice(tx_hash_0.as_ref()),
        B256::from_slice(tx_hash_1.as_ref()),
        B256::from_slice(tx_hash_2.as_ref()),
    ];
    expected_hashes.sort();
    assert_eq!(traced_hashes, expected_hashes);

    // Each trace should be a simple top-level CALL with no subtraces and expected (from, to,
    // value).
    let mut expected_calls = vec![
        (alith_addr, baltathar_addr, value_0),
        (alith_addr, dorothy_addr, value_1),
        (baltathar_addr, alith_addr, value_2),
    ];

    for localized_trace in &block_traces {
        let trace = &localized_trace.trace;
        assert!(trace.trace_address.is_empty(), "top-level trace should have empty trace_address");
        assert_eq!(trace.subtraces, 0, "simple transfers should not have nested subtraces");
        match &trace.action {
            ParityAction::Call(ParityCallAction {
                from: act_from,
                to: act_to,
                value: act_value,
                ..
            }) => {
                let triple = (*act_from, *act_to, *act_value);
                if let Some(pos) =
                    expected_calls.iter().position(|(f, t, v)| (*f, *t, *v) == triple)
                {
                    expected_calls.remove(pos);
                } else {
                    panic!("unexpected (from, to, value) in trace_block: {triple:?}");
                }
            }
            other => panic!("expected parity Call action for simple transfer, got {other:?}"),
        }
    }
    assert!(expected_calls.is_empty(), "not all expected transfers were seen in trace_block");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_trace_nested_calls() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let storage_contract_code = get_contract_code("SimpleStorage");
    let tx_hash_deploy_storage =
        node.deploy_contract(&storage_contract_code.init, alith.address()).await;
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    let receipt_deploy_storage = node.get_transaction_receipt(tx_hash_deploy_storage).await;
    let storage_contract_address =
        Address::from(ReviveAddress::new(receipt_deploy_storage.contract_address.unwrap()));
    let caller_constructor_arg = DynSolValue::Address(storage_contract_address);
    let caller_contract_code =
        get_contract_code_with_args("SimpleStorageCaller", vec![caller_constructor_arg]);
    let tx_hash_deploy_caller =
        node.deploy_contract(&caller_contract_code.init, alith.address()).await;
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    let receipt_deploy_caller = node.get_transaction_receipt(tx_hash_deploy_caller).await;
    let caller_contract_address =
        Address::from(ReviveAddress::new(receipt_deploy_caller.contract_address.unwrap()));

    // First nested call: SimpleStorageCaller.callSetValue(511)
    let call_set_value_data =
        SimpleStorageCaller::callSetValueCall::new((U256::from(511),)).abi_encode();
    let call_set_value_tx = TransactionRequest::default()
        .from(Address::from(ReviveAddress::new(alith.address())))
        .to(caller_contract_address)
        .input(TransactionInput::both(call_set_value_data.into()));
    let call_set_value_tx_hash = node.send_transaction(call_set_value_tx.clone()).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    let _call_receipt = node.get_transaction_receipt(call_set_value_tx_hash).await;

    // Second nested call: SimpleStorageCaller.callGetValue()
    let call_get_value_data = SimpleStorageCaller::callGetValueCall::new(()).abi_encode();
    let call_get_value_tx = TransactionRequest::default()
        .from(Address::from(ReviveAddress::new(alith.address())))
        .to(caller_contract_address)
        .input(TransactionInput::both(call_get_value_data.into()));
    let call_get_value_tx_hash = node.send_transaction(call_get_value_tx.clone()).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    // Verify that the nested call chain actually set and returns the expected value.
    let call_get_value_call = TransactionRequest::default()
        .from(Address::from(ReviveAddress::new(alith.address())))
        .to(caller_contract_address)
        .input(TransactionInput::both(
            SimpleStorageCaller::callGetValueCall::new(()).abi_encode().into(),
        ));
    let value_bytes: Bytes = unwrap_response(
        node.eth_rpc(EthRequest::EthCall(
            WithOtherFields::new(call_get_value_call),
            None,
            None,
            None,
        ))
        .await
        .unwrap(),
    )
    .unwrap();
    let call_get_value_result =
        SimpleStorageCaller::callGetValueCall::abi_decode_returns(&value_bytes.0).unwrap();
    assert_eq!(U256::from(511), call_get_value_result);

    // --- debug_traceTransaction: ensure nested call structure is present ---
    let nested_resp = node
        .eth_rpc(EthRequest::DebugTraceTransaction(
            B256::from_slice(call_get_value_tx_hash.as_ref()),
            GethDebugTracingOptions::default()
                .with_tracer(GethDebugTracerType::from(GethDebugBuiltInTracerType::CallTracer)),
        ))
        .await
        .unwrap();
    let nested_trace: GethTrace = unwrap_response(nested_resp).unwrap();
    let top_frame = match nested_trace {
        GethTrace::CallTracer(frame) => frame,
        other => panic!("expected CallTracer trace for nested calls, got {other:?}"),
    };
    // Top-level call should be to the caller contract
    assert_eq!(top_frame.to, Some(caller_contract_address));
    // There should be at least one nested call into the SimpleStorage contract
    assert_eq!(
        top_frame.calls.len(),
        1,
        "expected exactly one nested call from SimpleStorageCaller"
    );
    let nested_call = &top_frame.calls[0];
    // Nested call should originate from the caller contract and target the storage contract
    assert_eq!(nested_call.from, caller_contract_address);
    assert_eq!(nested_call.to, Some(storage_contract_address));
    // No further nesting under the storage call
    assert!(
        nested_call.calls.is_empty(),
        "expected no further nesting under the SimpleStorage call"
    );

    // --- debug_traceCall: simulate the same call and ensure structure matches ---
    let debug_call_opts = GethDebugTracingCallOptions::default().with_tracing_options(
        GethDebugTracingOptions::default()
            .with_tracer(GethDebugTracerType::from(GethDebugBuiltInTracerType::CallTracer)),
    );
    let debug_call_resp = node
        .eth_rpc(EthRequest::DebugTraceCall(
            WithOtherFields::new(call_get_value_tx),
            None,
            debug_call_opts,
        ))
        .await
        .unwrap();
    let debug_call_trace: GethTrace = unwrap_response(debug_call_resp).unwrap();
    let debug_call_frame = match debug_call_trace {
        GethTrace::CallTracer(frame) => frame,
        other => panic!("expected CallTracer trace for debug_traceCall, got {other:?}"),
    };
    assert_eq!(debug_call_frame.to, Some(caller_contract_address));
    assert_eq!(
        debug_call_frame.calls.len(),
        1,
        "expected exactly one nested call in debug_traceCall as well"
    );

    // --- trace_transaction: parity-style traces with multiple entries and trace addresses ---
    let parity_resp = node
        .eth_rpc(EthRequest::TraceTransaction(B256::from_slice(call_get_value_tx_hash.as_ref())))
        .await
        .unwrap();
    let parity_traces: Vec<LocalizedTransactionTrace> = unwrap_response(parity_resp).unwrap();
    assert!(
        parity_traces.len() >= 2,
        "expected at least 2 parity traces (top-level + nested), got {}",
        parity_traces.len()
    );

    // Find top-level and first nested trace by their trace_address
    let mut root_trace = None;
    let mut child_trace = None;
    for t in &parity_traces {
        if t.trace.trace_address.is_empty() {
            root_trace = Some(t);
        } else if t.trace.trace_address == vec![0] {
            child_trace = Some(t);
        }
    }
    let root_trace = root_trace.expect("missing root parity trace");
    let child_trace = child_trace.expect("missing child parity trace with trace_address [0]");

    // Root trace should be the call into the caller contract
    assert!(
        root_trace.trace.trace_address.is_empty(),
        "root trace should have empty trace_address"
    );
    assert_eq!(root_trace.trace.subtraces, 1, "root trace should report exactly one subtrace");
    match &root_trace.trace.action {
        ParityAction::Call(ParityCallAction { to, .. }) => {
            assert_eq!(*to, caller_contract_address);
        }
        other => panic!("expected root parity Call action, got {other:?}"),
    }

    // Child trace should be the call into the storage contract
    assert_eq!(
        child_trace.trace.trace_address,
        vec![0],
        "child trace should have trace_address [0]"
    );
    assert_eq!(child_trace.trace.subtraces, 0, "child trace should not have further subtraces");
    match &child_trace.trace.action {
        ParityAction::Call(ParityCallAction { to, .. }) => {
            assert_eq!(*to, storage_contract_address);
        }
        other => panic!("expected child parity Call action, got {other:?}"),
    }
}
