use crate::{
    abi::{
        SimpleStorage::{self as SimpleStorage},
        SimpleStorageCaller::{self as SimpleStorageCaller},
    },
    utils::{
        TestNode, get_contract_code, get_contract_code_with_args, is_transaction_in_block,
        multicall_get_coinbase, unwrap_response,
    },
};
use alloy_dyn_abi::DynSolValue;
use alloy_primitives::{Address, B256, Bytes, U256, map::HashSet};
use alloy_rpc_types::{
    Index, TransactionInput, TransactionRequest,
    anvil::{Metadata as AnvilMetadata, NodeInfo},
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
use alloy_sol_types::{SolCall, SolEvent};
use anvil_core::eth::EthRequest;
use anvil_polkadot::{
    api_server::revive_conversions::{AlloyU256, ReviveAddress},
    config::{AnvilNodeConfig, SubstrateNodeConfig},
};
use anvil_rpc::error::ErrorCode;
use polkadot_sdk::{
    pallet_revive::{
        self,
        evm::{Account, Block, FeeHistoryResult, FilterResults, TransactionInfo},
    },
    sp_core::{H256, keccak_256},
};
use subxt::utils::H160;

#[tokio::test(flavor = "multi_thread")]
async fn test_get_chain_id() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // expected 31337, default value from the Anvil config
    assert_eq!(
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthChainId(())).await.unwrap()).unwrap(),
        "0x7a69"
    );
    // expected 31337, default value from the Anvil config
    assert_eq!(
        unwrap_response::<u64>(node.eth_rpc(EthRequest::EthNetworkId(())).await.unwrap()).unwrap(),
        0x7a69
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_start_balance() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    assert_eq!(
        node.get_balance(
            H160::from_slice(subxt_signer::eth::dev::alith().public_key().to_account_id().as_ref()),
            None
        )
        .await,
        U256::from_str_radix("10000000000000000000000", 10).unwrap()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_block_by_hash() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let baltathar = Account::from(subxt_signer::eth::dev::baltathar());
    let transfer_amount = U256::from_str_radix("100000000000000000", 10).unwrap();
    let alith_addr = Address::from(ReviveAddress::new(alith.address()));
    let baltathar_addr = Address::from(ReviveAddress::new(baltathar.address()));
    let transaction =
        TransactionRequest::default().value(transfer_amount).from(alith_addr).to(baltathar_addr);
    let tx_hash0 = node.send_transaction(transaction.clone()).await.unwrap();
    let tx_hash1 = node.send_transaction(transaction.clone().nonce(1)).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    let tx_hash2 = node.send_transaction(transaction.nonce(2)).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    let hash1 = node.block_hash_by_number(1).await.unwrap();
    let hash2 = node.block_hash_by_number(2).await.unwrap();
    let block1 = node.get_block_by_hash(hash1).await;
    let block2 = node.get_block_by_hash(hash2).await;
    assert!(is_transaction_in_block(&block1.transactions, tx_hash0));
    assert!(is_transaction_in_block(&block1.transactions, tx_hash1));
    assert!(is_transaction_in_block(&block2.transactions, tx_hash2));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_send_transaction() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::SetAutomine(true)).await.unwrap()).unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let baltathar = Account::from(subxt_signer::eth::dev::baltathar());
    let alith_initial_balance = node.get_balance(alith.address(), None).await;
    let baltathar_initial_balance = node.get_balance(baltathar.address(), None).await;
    let transfer_amount = U256::from_str_radix("100000000000000000", 10).unwrap();
    let transaction = TransactionRequest::default()
        .value(transfer_amount)
        .from(Address::from(ReviveAddress::new(alith.address())))
        .to(Address::from(ReviveAddress::new(baltathar.address())));
    let tx_hash = node.send_transaction(transaction).await.unwrap();
    let transaction_receipt = node.get_transaction_receipt(tx_hash).await;

    assert_eq!(transaction_receipt.block_number, pallet_revive::U256::from(1));
    assert_eq!(transaction_receipt.transaction_index, pallet_revive::U256::from(1));
    assert_eq!(transaction_receipt.transaction_hash, tx_hash);

    let alith_final_balance = node.get_balance(alith.address(), None).await;
    let baltathar_final_balance = node.get_balance(baltathar.address(), None).await;
    assert_eq!(
        baltathar_final_balance,
        baltathar_initial_balance + transfer_amount,
        "Baltathar's balance should have changed"
    );
    assert_eq!(
        alith_final_balance,
        alith_initial_balance
            - transfer_amount
            - AlloyU256::from(
                transaction_receipt.effective_gas_price * transaction_receipt.gas_used
            )
            .inner(),
        "Alith's balance should have changed"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_estimate_gas() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();
    let alith = Account::from(subxt_signer::eth::dev::alith());
    let baltathar = Account::from(subxt_signer::eth::dev::baltathar());
    let transfer_amount = U256::from_str_radix("100000000000000000", 10).unwrap();
    let transaction = TransactionRequest::default()
        .value(transfer_amount)
        .from(Address::from(ReviveAddress::new(alith.address())))
        .to(Address::from(ReviveAddress::new(baltathar.address())));

    let estimated_gas: pallet_revive::U256 = unwrap_response(
        node.eth_rpc(EthRequest::EthEstimateGas(
            WithOtherFields::new(transaction.clone()),
            None,
            None,
            None,
        ))
        .await
        .unwrap(),
    )
    .unwrap();
    let tx_hash = node.send_transaction(transaction).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    let receipt = node.get_transaction_receipt(tx_hash).await;
    // https://github.com/paritytech/polkadot-sdk/blob/b21cbb58ab50d5d10371393967537f6f221bb92f/substrate/frame/revive/src/primitives.rs#L76
    // eth_gas that is returned by estimate_gas holds both the storage deposit and
    // the weight, hence it is expected to be higher than the
    // gas amount actually used.
    assert!(estimated_gas > receipt.gas_used);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_gas_price() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let gas_price =
        unwrap_response::<U256>(node.eth_rpc(EthRequest::EthGasPrice(())).await.unwrap()).unwrap();
    assert_eq!(gas_price, U256::from(1000000));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_block_by_number() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let baltathar = Account::from(subxt_signer::eth::dev::baltathar());
    let transfer_amount = U256::from_str_radix("100000000000000000", 10).unwrap();
    let transaction = TransactionRequest::default()
        .value(transfer_amount)
        .from(Address::from(ReviveAddress::new(alith.address())))
        .to(Address::from(ReviveAddress::new(baltathar.address())));
    let tx_hash = node.send_transaction(transaction).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    let block_by_number = unwrap_response::<Block>(
        node.eth_rpc(EthRequest::EthGetBlockByNumber(
            alloy_eips::BlockNumberOrTag::Number(1),
            false,
        ))
        .await
        .unwrap(),
    )
    .unwrap();
    assert!(is_transaction_in_block(&block_by_number.transactions, tx_hash));
    // Check that GetBlockByNumber fails if the block number does not fit in u32
    // TODO: expand the error conversion for ReviveRpc type
    let err = unwrap_response::<Option<Block>>(
        node.eth_rpc(EthRequest::EthGetBlockByNumber(
            alloy_eips::BlockNumberOrTag::Number(u64::MAX),
            true,
        ))
        .await
        .unwrap(),
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::InternalError);
    assert_eq!(err.message, "Revive call failed: Client error: conversion failed");
    // Assert that we can not find blocks that do not exist.
    assert_eq!(
        unwrap_response::<Option<Block>>(
            node.eth_rpc(EthRequest::EthGetBlockByNumber(
                alloy_eips::BlockNumberOrTag::Number(2),
                true
            ))
            .await
            .unwrap()
        )
        .unwrap(),
        None
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_eth_block_number() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    assert_eq!(
        unwrap_response::<U256>(node.eth_rpc(EthRequest::EthBlockNumber(())).await.unwrap())
            .unwrap(),
        U256::from(0)
    );
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(3)), None)).await.unwrap())
        .unwrap();
    assert_eq!(
        unwrap_response::<U256>(node.eth_rpc(EthRequest::EthBlockNumber(())).await.unwrap())
            .unwrap(),
        U256::from(3)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_eth_get_transaction_count() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();
    let alith = Account::from(subxt_signer::eth::dev::alith());

    // Get transaction count from a block that does not exist yet
    let err = unwrap_response::<pallet_revive::U256>(
        node.eth_rpc(EthRequest::EthGetTransactionCount(
            Address::from(ReviveAddress::new(alith.address())),
            Some(alloy_eips::BlockId::Number(alloy_eips::BlockNumberOrTag::Number(1))),
        ))
        .await
        .unwrap(),
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidParams);
    assert_eq!(err.message, "Block number not found");

    assert_eq!(
        unwrap_response::<U256>(
            node.eth_rpc(EthRequest::EthGetTransactionCount(
                Address::from(ReviveAddress::new(alith.address())),
                Some(alloy_eips::BlockId::Number(alloy_eips::BlockNumberOrTag::Number(0))),
            ))
            .await
            .unwrap()
        )
        .unwrap(),
        U256::from(0)
    );

    let transfer_amount = U256::from_str_radix("100000000000000000", 10).unwrap();
    let transaction = TransactionRequest::default()
        .value(transfer_amount)
        .from(Address::from(ReviveAddress::new(alith.address())))
        .to(Address::from(ReviveAddress::new(
            Account::from(subxt_signer::eth::dev::alith()).address(),
        )));
    let _tx_hash0 = node.send_transaction(transaction.clone()).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    assert_eq!(
        unwrap_response::<U256>(
            node.eth_rpc(EthRequest::EthGetTransactionCount(
                Address::from(ReviveAddress::new(alith.address())),
                None,
            ))
            .await
            .unwrap()
        )
        .unwrap(),
        U256::from(1)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_transaction_count_by_hash_number() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();
    let alith = Account::from(subxt_signer::eth::dev::alith());

    let transfer_amount = U256::from_str_radix("100000000000000000", 10).unwrap();
    let transaction = TransactionRequest::default()
        .value(transfer_amount)
        .from(Address::from(ReviveAddress::new(alith.address())))
        .to(Address::from(ReviveAddress::new(
            Account::from(subxt_signer::eth::dev::alith()).address(),
        )));
    let _tx_hash0 = node.send_transaction(transaction.clone()).await.unwrap();
    // Check that we get None for missing block
    assert_eq!(
        unwrap_response::<Option<U256>>(
            node.eth_rpc(EthRequest::EthGetTransactionCountByNumber(
                alloy_eips::BlockNumberOrTag::Number(1)
            ))
            .await
            .unwrap()
        )
        .unwrap(),
        None
    );
    // Check that there are no transactions in genesis
    assert_eq!(
        unwrap_response::<Option<U256>>(
            node.eth_rpc(EthRequest::EthGetTransactionCountByNumber(
                alloy_eips::BlockNumberOrTag::Latest
            ))
            .await
            .unwrap()
        )
        .unwrap()
        .unwrap(),
        U256::from(0)
    );
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    assert_eq!(
        unwrap_response::<Option<U256>>(
            node.eth_rpc(EthRequest::EthGetTransactionCountByHash(B256::from_slice(
                node.block_hash_by_number(1).await.unwrap().as_ref()
            )))
            .await
            .unwrap()
        )
        .unwrap()
        .unwrap(),
        U256::from(1)
    );
    // There should be a transaction in block number 1
    assert_eq!(
        unwrap_response::<Option<U256>>(
            node.eth_rpc(EthRequest::EthGetTransactionCountByNumber(
                alloy_eips::BlockNumberOrTag::Latest
            ))
            .await
            .unwrap()
        )
        .unwrap()
        .unwrap(),
        U256::from(1)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_code_at() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Check random address
    let code = unwrap_response::<Bytes>(
        node.eth_rpc(EthRequest::EthGetCodeAt(Address::random(), None)).await.unwrap(),
    )
    .unwrap();

    assert!(code.is_empty(), "Contract code should be empty");
    let alith = Account::from(subxt_signer::eth::dev::alith());
    let contract_code = get_contract_code("SimpleStorage");
    let tx_hash = node.deploy_contract(&contract_code.init, alith.address()).await;
    let _ = node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap();
    let receipt = node.get_transaction_receipt(tx_hash).await;
    assert_eq!(receipt.status, Some(pallet_revive::U256::from(1)));
    let contract_address = receipt.contract_address.unwrap();

    let code = unwrap_response::<Bytes>(
        node.eth_rpc(EthRequest::EthGetCodeAt(
            Address::from(ReviveAddress::new(contract_address)),
            None,
        ))
        .await
        .unwrap(),
    )
    .unwrap();

    assert!(!code.is_empty(), "Contract code should not be empty");
    assert_eq!(
        code,
        Bytes::from(contract_code.runtime.unwrap()),
        "Retrieved code should exactly match deployed bytecode"
    );

    let code = unwrap_response::<Bytes>(
        node.eth_rpc(EthRequest::EthGetCodeAt(
            Address::from(ReviveAddress::new(contract_address)),
            Some(alloy_eips::BlockId::Number(alloy_eips::BlockNumberOrTag::Number(0))),
        ))
        .await
        .unwrap(),
    )
    .unwrap();
    assert!(code.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_transaction_by_hash_and_index() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let baltathar = Account::from(subxt_signer::eth::dev::baltathar());
    let transfer_amount = U256::from_str_radix("100000000000000000", 10).unwrap();
    let transaction = TransactionRequest::default()
        .value(transfer_amount)
        .from(Address::from(ReviveAddress::new(alith.address())))
        .to(Address::from(ReviveAddress::new(baltathar.address())));
    let tx_hash0 = node.send_transaction(transaction.clone()).await.unwrap();
    let tx_hash1 = node
        .send_transaction(
            transaction
                .from(Address::from(ReviveAddress::new(baltathar.address())))
                .to(Address::from(ReviveAddress::new(alith.address()))),
        )
        .await
        .unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    assert_eq!(
        unwrap_response::<Option<TransactionInfo>>(
            node.eth_rpc(EthRequest::EthGetTransactionByBlockHashAndIndex(
                B256::from_slice(node.block_hash_by_number(0).await.unwrap().as_ref()),
                Index(1)
            ))
            .await
            .unwrap()
        )
        .unwrap(),
        None
    );

    let first_hash = node.block_hash_by_number(1).await.unwrap();
    let transaction_info_1 = unwrap_response::<Option<TransactionInfo>>(
        node.eth_rpc(EthRequest::EthGetTransactionByBlockHashAndIndex(
            B256::from_slice(first_hash.as_ref()),
            Index(1),
        ))
        .await
        .unwrap(),
    )
    .unwrap()
    .unwrap();
    let transaction_info_2 = unwrap_response::<Option<TransactionInfo>>(
        node.eth_rpc(EthRequest::EthGetTransactionByBlockHashAndIndex(
            B256::from_slice(first_hash.as_ref()),
            Index(2),
        ))
        .await
        .unwrap(),
    )
    .unwrap()
    .unwrap();

    let eth_first_hash = node.resolve_ethereum_hash(first_hash).unwrap();
    assert_eq!(eth_first_hash, transaction_info_1.block_hash);
    assert_eq!(transaction_info_1.from, alith.address());
    assert_eq!(tx_hash0, transaction_info_1.hash);

    assert_eq!(eth_first_hash, transaction_info_2.block_hash);
    assert_eq!(transaction_info_2.from, baltathar.address());
    assert_eq!(tx_hash1, transaction_info_2.hash);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_transaction_by_number_and_index() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let baltathar = Account::from(subxt_signer::eth::dev::baltathar());
    let transfer_amount = U256::from_str_radix("100000000000000000", 10).unwrap();
    let transaction = TransactionRequest::default()
        .value(transfer_amount)
        .from(Address::from(ReviveAddress::new(alith.address())))
        .to(Address::from(ReviveAddress::new(baltathar.address())));
    let tx_hash0 = node.send_transaction(transaction.clone()).await.unwrap();
    let tx_hash1 = node
        .send_transaction(
            transaction
                .from(Address::from(ReviveAddress::new(baltathar.address())))
                .to(Address::from(ReviveAddress::new(alith.address()))),
        )
        .await
        .unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    let transaction_info_1 = unwrap_response::<Option<TransactionInfo>>(
        node.eth_rpc(EthRequest::EthGetTransactionByBlockNumberAndIndex(
            alloy_eips::BlockNumberOrTag::Latest,
            Index(1),
        ))
        .await
        .unwrap(),
    )
    .unwrap()
    .unwrap();
    let transaction_info_2 = unwrap_response::<Option<TransactionInfo>>(
        node.eth_rpc(EthRequest::EthGetTransactionByBlockNumberAndIndex(
            alloy_eips::BlockNumberOrTag::Number(1),
            Index(2),
        ))
        .await
        .unwrap(),
    )
    .unwrap()
    .unwrap();

    let first_hash = node.eth_block_hash_by_number(1).await.unwrap();
    assert_eq!(first_hash, transaction_info_1.block_hash);
    assert_eq!(transaction_info_1.from, alith.address());
    assert_eq!(tx_hash0, transaction_info_1.hash);

    assert_eq!(first_hash, transaction_info_2.block_hash);
    assert_eq!(transaction_info_2.from, baltathar.address());
    assert_eq!(tx_hash1, transaction_info_2.hash);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_transaction_by_hash() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let baltathar = Account::from(subxt_signer::eth::dev::baltathar());
    let transfer_amount = U256::from_str_radix("100000000000000000", 10).unwrap();
    let transaction = TransactionRequest::default()
        .value(transfer_amount)
        .from(Address::from(ReviveAddress::new(alith.address())))
        .to(Address::from(ReviveAddress::new(baltathar.address())));
    let tx_hash0 = node.send_transaction(transaction.clone()).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    let transaction_info = unwrap_response::<Option<TransactionInfo>>(
        node.eth_rpc(EthRequest::EthGetTransactionByHash(B256::from_slice(tx_hash0.as_ref())))
            .await
            .unwrap(),
    )
    .unwrap()
    .unwrap();

    let first_hash = node.eth_block_hash_by_number(1).await.unwrap();
    assert_eq!(first_hash, transaction_info.block_hash);
    assert_eq!(transaction_info.from, alith.address());
    assert_eq!(tx_hash0, transaction_info.hash);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_storage() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::SetAutomine(true)).await.unwrap()).unwrap();
    let alith = Account::from(subxt_signer::eth::dev::alith());

    // Test retrieving the storage of an EOA account (alith)
    let stored_value = node.get_storage_at(U256::from(0), alith.address()).await;
    assert_eq!(stored_value, 0);

    // Test retrieving the storage of a non-existant account.
    let random_addr = Address::random();
    let stored_value =
        node.get_storage_at(U256::from(0), ReviveAddress::from(random_addr).inner()).await;
    assert_eq!(stored_value, 0);

    let contract_code = get_contract_code("SimpleStorage");
    let tx_hash = node.deploy_contract(&contract_code.init, alith.address()).await;
    let receipt = node.get_transaction_receipt(tx_hash).await;
    let contract_address = receipt.contract_address.unwrap();

    // Check the default value for slot 0.
    let stored_value = node.get_storage_at(U256::from(0), contract_address).await;
    assert_eq!(stored_value, 0);

    // Set a new value for the slot 0.
    let set_value_data = SimpleStorage::setValueCall::new((U256::from(511),)).abi_encode();
    let call_tx = TransactionRequest::default()
        .from(Address::from(ReviveAddress::new(alith.address())))
        .to(Address::from(ReviveAddress::new(contract_address)))
        .input(TransactionInput::both(set_value_data.into()));

    let _call_tx_hash = node.send_transaction(call_tx).await.unwrap();

    // Check that the value was updated
    let stored_value = node.get_storage_at(U256::from(0), contract_address).await;
    assert_eq!(stored_value, 511);
    // Check value that has not been set
    let stored_value = node.get_storage_at(U256::from(1), contract_address).await;
    assert_eq!(stored_value, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_fee_history() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::SetAutomine(true)).await.unwrap()).unwrap();
    let fee_history = unwrap_response::<FeeHistoryResult>(
        node.eth_rpc(EthRequest::EthFeeHistory(
            U256::from(0),
            alloy_eips::BlockNumberOrTag::Latest,
            vec![],
        ))
        .await
        .unwrap(),
    )
    .unwrap();
    assert!(fee_history.base_fee_per_gas.is_empty());
    assert!(fee_history.gas_used_ratio.is_empty());
    assert!(fee_history.reward.is_empty());

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let baltathar = Account::from(subxt_signer::eth::dev::baltathar());
    let transfer_amount = U256::from_str_radix("100000000000", 10).unwrap();
    let transaction = TransactionRequest::default()
        .value(transfer_amount)
        .from(Address::from(ReviveAddress::new(alith.address())))
        .to(Address::from(ReviveAddress::new(baltathar.address())));

    for i in 0..10 {
        let _hash = node.send_transaction(transaction.clone().nonce(i)).await.unwrap();
    }
    let fee_history = unwrap_response::<FeeHistoryResult>(
        node.eth_rpc(EthRequest::EthFeeHistory(
            U256::from(10),
            alloy_eips::BlockNumberOrTag::Latest,
            vec![],
        ))
        .await
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fee_history.gas_used_ratio.len(), 10);
    // The `SlowAdjustingFeeUpdate` logic decreases the base_fee block by block if the
    // activity contained within them is low.
    let base_fees =
        [1_000_000, 999981, 999962, 999944, 999925, 999907, 999888, 999869, 999851, 999832, 999832];
    for (idx, base_fee) in fee_history.base_fee_per_gas.into_iter().enumerate() {
        assert_eq!(base_fee, pallet_revive::U256::from(base_fees[idx]));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_max_fee_per_gas() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    assert_eq!(
        "0x0",
        unwrap_response::<String>(
            node.eth_rpc(EthRequest::EthMaxPriorityFeePerGas(())).await.unwrap()
        )
        .unwrap()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_accounts() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let dorothy = Account::from(subxt_signer::eth::dev::dorothy()).address();
    let accounts =
        unwrap_response::<Vec<H160>>(node.eth_rpc(EthRequest::EthAccounts(())).await.unwrap())
            .unwrap();
    assert_eq!(accounts.len(), 12);

    // Test that retrieving the accounts multiple times will yield them in the same order.
    for _ in 0..3 {
        let res =
            unwrap_response::<Vec<H160>>(node.eth_rpc(EthRequest::EthAccounts(())).await.unwrap())
                .unwrap();

        assert_eq!(res, accounts);
    }

    node.eth_rpc(EthRequest::ImpersonateAccount(Address::from(ReviveAddress::new(accounts[0]))))
        .await
        .unwrap();
    node.eth_rpc(EthRequest::ImpersonateAccount(Address::from(ReviveAddress::new(dorothy))))
        .await
        .unwrap();
    let accounts_with_impersonation =
        unwrap_response::<Vec<H160>>(node.eth_rpc(EthRequest::EthAccounts(())).await.unwrap())
            .unwrap();
    assert_eq!(accounts_with_impersonation.len(), 13);
    assert!(accounts_with_impersonation.contains(&dorothy));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_logs() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_address = ReviveAddress::new(alith.address());
    let contract_code = get_contract_code("SimpleStorage");
    let tx_hash = node.deploy_contract(&contract_code.init, alith.address()).await;
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    let receipt = node.get_transaction_receipt(tx_hash).await;
    let contract_address = receipt.contract_address.unwrap();

    for i in 0..2 {
        let set_value_data = SimpleStorage::setValueCall::new((U256::from(511 + i),)).abi_encode();
        let call_tx = TransactionRequest::default()
            .from(Address::from(alith_address))
            .to(Address::from(ReviveAddress::new(contract_address)))
            .input(TransactionInput::both(set_value_data.into()))
            .nonce(i + 1);

        let _call_tx_hash = node.send_transaction(call_tx).await.unwrap();
    }
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    let filter = alloy_rpc_types::Filter::new()
        .address(Address::from(ReviveAddress::new(contract_address)))
        .from_block(0)
        .to_block(2);
    let logs = match unwrap_response::<FilterResults>(
        node.eth_rpc(EthRequest::EthGetLogs(filter)).await.unwrap(),
    )
    .unwrap()
    {
        FilterResults::Logs(entries) => entries,
        _ => panic!("This should be a vec of logs."),
    };

    let mut tx_indices = HashSet::from([1, 2]);
    tx_indices.remove(&(logs[1].transaction_index.try_into().unwrap()));
    tx_indices.remove(&(logs[2].transaction_index.try_into().unwrap()));
    assert_eq!(logs.len(), 3);
    assert_eq!(logs[1].block_number, pallet_revive::U256::from(2));
    assert_eq!(logs[2].block_number, pallet_revive::U256::from(2));
    assert_eq!(logs[0].transaction_hash, tx_hash);
    assert_eq!(tx_indices.len(), 0);
    // Check that our topic is the ValueChanged event.
    let event_hash = keccak_256(b"ValueChanged(address,uint256,uint256)");
    assert_eq!(logs[2].topics[0], H256::from(event_hash));
    // Assert the values changed
    let data = logs[2].data.as_ref().unwrap();
    let decoded_data = SimpleStorage::ValueChanged::abi_decode_data(&data.0).unwrap();

    // Assert the old value
    assert_eq!(decoded_data.0, U256::from(511));
    // Assert the new value
    assert_eq!(decoded_data.1, U256::from(512));

    // Assert the changer address
    let changer_topic = logs[2].topics[1].as_bytes();
    let mut changer = [0u8; 20];
    changer.copy_from_slice(&changer_topic[12..32]);
    assert_eq!(alith_address.inner(), H160(changer));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_call() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::SetAutomine(true)).await.unwrap()).unwrap();
    let alith = Account::from(subxt_signer::eth::dev::alith());

    let contract_code = get_contract_code("SimpleStorage");
    let tx_hash = node.deploy_contract(&contract_code.init, alith.address()).await;
    let receipt = node.get_transaction_receipt(tx_hash).await;
    let contract_address = receipt.contract_address.unwrap();

    let set_value_data = SimpleStorage::setValueCall::new((U256::from(511),)).abi_encode();
    let call_tx = TransactionRequest::default()
        .from(Address::from(ReviveAddress::new(alith.address())))
        .to(Address::from(ReviveAddress::new(contract_address)))
        .input(TransactionInput::both(set_value_data.into()));

    let _call_tx_hash = node.send_transaction(call_tx).await.unwrap();

    let get_value_data = SimpleStorage::getValueCall::new(()).abi_encode();
    let call_tx = TransactionRequest::default()
        .from(Address::from(ReviveAddress::new(alith.address())))
        .to(Address::from(ReviveAddress::new(contract_address)))
        .input(TransactionInput::both(get_value_data.into()));
    let res: Bytes = unwrap_response(
        node.eth_rpc(EthRequest::EthCall(WithOtherFields::new(call_tx), None, None, None))
            .await
            .unwrap(),
    )
    .unwrap();
    let value = SimpleStorage::getValueCall::abi_decode_returns(&res.0).unwrap();
    assert_eq!(U256::from(511), value);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_coinbase() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::SetAutomine(true)).await.unwrap()).unwrap();

    // Deploy multicall contract
    let alith_addr = Account::from(subxt_signer::eth::dev::alith()).address();
    let contract_code = get_contract_code("Multicall");
    let tx_hash = node.deploy_contract(&contract_code.init, alith_addr).await;

    // Get contract address.
    let receipt = node.get_transaction_receipt(tx_hash).await;
    assert_eq!(receipt.status, Some(pallet_revive::U256::from(1)));
    let contract_address = Address::from(receipt.contract_address.unwrap().to_fixed_bytes());
    let alith_addr = Address::from(alith_addr.to_fixed_bytes());

    // Make a get coinbase contract call.
    let coinbase = multicall_get_coinbase(&mut node, alith_addr, contract_address).await;
    assert_eq!(coinbase, Address::ZERO);
    assert_eq!(
        unwrap_response::<Address>(node.eth_rpc(EthRequest::EthCoinbase(())).await.unwrap())
            .unwrap(),
        Address::ZERO,
    );

    let new_coinbase = Address::random();
    node.eth_rpc(EthRequest::SetCoinbase(new_coinbase)).await.unwrap();
    assert_eq!(
        unwrap_response::<Address>(node.eth_rpc(EthRequest::EthCoinbase(())).await.unwrap())
            .unwrap(),
        new_coinbase
    );

    let coinbase = multicall_get_coinbase(&mut node, alith_addr, contract_address).await;
    assert_eq!(coinbase, new_coinbase);
    assert_eq!(
        unwrap_response::<Address>(node.eth_rpc(EthRequest::EthCoinbase(())).await.unwrap())
            .unwrap(),
        new_coinbase,
    );

    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(3)), None)).await.unwrap())
        .unwrap();
    assert_eq!(
        unwrap_response::<U256>(node.eth_rpc(EthRequest::EthBlockNumber(())).await.unwrap())
            .unwrap(),
        U256::from(4)
    );
    assert_eq!(
        unwrap_response::<Address>(node.eth_rpc(EthRequest::EthCoinbase(())).await.unwrap())
            .unwrap(),
        new_coinbase
    );
    let coinbase = multicall_get_coinbase(&mut node, alith_addr, contract_address).await;
    assert_eq!(coinbase, new_coinbase);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_anvil_node_info() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let node_info =
        unwrap_response::<NodeInfo>(node.eth_rpc(EthRequest::NodeInfo(())).await.unwrap()).unwrap();

    // Check initial state - should be at genesis block
    assert_eq!(node_info.current_block_number, 0);
    assert_eq!(node_info.hard_fork, "Prague".to_string());
    assert_eq!(node_info.transaction_order, "fifo");
    assert_eq!(node_info.environment.chain_id, 0x7a69);

    // Verify fork config is empty (forking not supported)
    assert_eq!(node_info.fork_config.fork_url, None);
    assert_eq!(node_info.fork_config.fork_block_number, None);
    assert_eq!(node_info.fork_config.fork_retry_backoff, None);

    let genesis_block_hash = node.block_hash_by_number(0).await.unwrap();
    assert_eq!(node_info.current_block_hash, B256::from_slice(genesis_block_hash.as_ref()));
    let block = node.get_block_by_hash(genesis_block_hash).await;
    assert_eq!(block.gas_limit, node_info.environment.gas_limit.into());
    assert_eq!(block.base_fee_per_gas, node_info.environment.base_fee.into());
    assert_eq!(block.base_fee_per_gas, node_info.environment.gas_price.into());

    // Mine some blocks and check that node_info updates
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(3)), None)).await.unwrap())
        .unwrap();

    let node_info_after =
        unwrap_response::<NodeInfo>(node.eth_rpc(EthRequest::NodeInfo(())).await.unwrap()).unwrap();

    // Block number should have increased
    assert_eq!(node_info_after.current_block_number, 3);

    // Timestamp should be greater or equal (may have advanced)
    assert!(node_info_after.current_block_timestamp >= node_info.current_block_timestamp);
    assert_eq!(node_info_after.environment.chain_id, node_info.environment.chain_id);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_anvil_metadata() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let metadata = unwrap_response::<AnvilMetadata>(
        node.eth_rpc(EthRequest::AnvilMetadata(())).await.unwrap(),
    )
    .unwrap();

    assert!(metadata.client_version.contains("anvil-polkadot"));
    assert_eq!(metadata.latest_block_number, 0);
    assert_eq!(metadata.chain_id, 0x7a69);

    // Verify forked_network is None (forking not supported)
    assert_eq!(metadata.forked_network, None);

    // Initial snapshots should be empty
    assert!(metadata.snapshots.is_empty());

    // Get current block hash for comparison
    let block_hash = node.block_hash_by_number(0).await.unwrap();
    assert_eq!(metadata.latest_block_hash, B256::from_slice(block_hash.as_ref()));

    // Create a snapshot and verify it appears in metadata
    let snapshot_id = U256::from_str_radix(
        unwrap_response::<String>(node.eth_rpc(EthRequest::EvmSnapshot(())).await.unwrap())
            .unwrap()
            .trim_start_matches("0x"),
        16,
    )
    .unwrap();

    let metadata_after_snapshot = unwrap_response::<AnvilMetadata>(
        node.eth_rpc(EthRequest::AnvilMetadata(())).await.unwrap(),
    )
    .unwrap();

    // Should have one snapshot
    assert_eq!(metadata_after_snapshot.snapshots.len(), 1);
    assert!(metadata_after_snapshot.snapshots.contains_key(&snapshot_id));

    // Mine some blocks and check that metadata updates
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(5)), None)).await.unwrap())
        .unwrap();

    let metadata_after_mining = unwrap_response::<AnvilMetadata>(
        node.eth_rpc(EthRequest::AnvilMetadata(())).await.unwrap(),
    )
    .unwrap();

    // Block number should have increased
    assert_eq!(metadata_after_mining.latest_block_number, 5);
    // Snapshot should still be present
    assert!(metadata_after_mining.snapshots.contains_key(&snapshot_id));
}

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
    let value = U256::from(1_000_000_000_000_000_000u64);

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

    // Disable automining so we can pack multiple transactions into the same block.
    unwrap_response::<()>(node.eth_rpc(EthRequest::SetAutomine(false)).await.unwrap()).unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let baltathar = Account::from(subxt_signer::eth::dev::baltathar());
    let dorothy = Account::from(subxt_signer::eth::dev::dorothy());

    let alith_addr = Address::from(ReviveAddress::new(alith.address()));
    let baltathar_addr = Address::from(ReviveAddress::new(baltathar.address()));
    let dorothy_addr = Address::from(ReviveAddress::new(dorothy.address()));

    let value_0 = U256::from(1_000_000_000_000_000_000u64);
    let value_1 = U256::from(2_000_000_000_000_000_000u64);
    let value_2 = U256::from(3_000_000_000_000_000_000u64);

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
                    panic!("unexpected (from, to, value) in trace_block: {:?}", triple);
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
    assert!(!debug_call_frame.calls.is_empty(), "expected nested calls in debug_traceCall as well");

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
