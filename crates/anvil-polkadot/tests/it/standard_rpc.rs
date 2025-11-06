use std::time::Duration;

use crate::{
    abi::SimpleStorage::{self as SimpleStorage},
    utils::{
        BlockWaitTimeout, TestNode, get_contract_code, is_transaction_in_block,
        multicall_get_coinbase, unwrap_response,
    },
};
use alloy_primitives::{Address, B256, Bytes, U256, map::HashSet};
use alloy_rpc_types::{
    Index, TransactionInput, TransactionRequest,
    anvil::{Metadata as AnvilMetadata, NodeInfo},
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
    let tx_hash0 = node.send_transaction(transaction.clone(), None).await.unwrap();
    let tx_hash1 = node.send_transaction(transaction.clone().nonce(1), None).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    let tx_hash2 = node.send_transaction(transaction.nonce(2), None).await.unwrap();
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
    let tx_hash = node
        .send_transaction(transaction, Some(BlockWaitTimeout::new(1, Duration::from_secs(1))))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
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
    let tx_hash = node.send_transaction(transaction, None).await.unwrap();
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
    let tx_hash = node.send_transaction(transaction, None).await.unwrap();
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
    let _tx_hash0 = node.send_transaction(transaction.clone(), None).await.unwrap();
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
    let _tx_hash0 = node.send_transaction(transaction.clone(), None).await.unwrap();
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
    let tx_hash = node.deploy_contract(&contract_code.init, alith.address(), None).await;
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
    let tx_hash0 = node.send_transaction(transaction.clone(), None).await.unwrap();
    let tx_hash1 = node
        .send_transaction(
            transaction
                .from(Address::from(ReviveAddress::new(baltathar.address())))
                .to(Address::from(ReviveAddress::new(alith.address()))),
            None,
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
    let tx_hash0 = node.send_transaction(transaction.clone(), None).await.unwrap();
    let tx_hash1 = node
        .send_transaction(
            transaction
                .from(Address::from(ReviveAddress::new(baltathar.address())))
                .to(Address::from(ReviveAddress::new(alith.address()))),
            None,
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
    let tx_hash0 = node.send_transaction(transaction.clone(), None).await.unwrap();
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
    let tx_hash = node.deploy_contract(&contract_code.init, alith.address(), Some(1)).await;
    tokio::time::sleep(Duration::from_millis(400)).await;
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

    let _call_tx_hash = node
        .send_transaction(
            call_tx,
            Some(BlockWaitTimeout { block_number: 2, timeout: Duration::from_millis(1000) }),
        )
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(400)).await;

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
        let _hash = node
            .send_transaction(
                transaction.clone().nonce(i),
                Some(BlockWaitTimeout::new((i + 1) as u32, Duration::from_secs(1))),
            )
            .await
            .unwrap();
    }

    // Wait a bit for block provider to catch up with all minted blocks.
    tokio::time::sleep(Duration::from_secs(1)).await;
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
    assert!(fee_history.base_fee_per_gas.iter().all(|&v| v == pallet_revive::U256::from(1000000)));
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
    let tx_hash = node.deploy_contract(&contract_code.init, alith.address(), None).await;
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

        let _call_tx_hash = node.send_transaction(call_tx, None).await.unwrap();
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
    let tx_hash = node.deploy_contract(&contract_code.init, alith.address(), Some(1)).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let receipt = node.get_transaction_receipt(tx_hash).await;
    let contract_address = receipt.contract_address.unwrap();

    let set_value_data = SimpleStorage::setValueCall::new((U256::from(511),)).abi_encode();
    let call_tx = TransactionRequest::default()
        .from(Address::from(ReviveAddress::new(alith.address())))
        .to(Address::from(ReviveAddress::new(contract_address)))
        .input(TransactionInput::both(set_value_data.into()));

    let _call_tx_hash = node
        .send_transaction(
            call_tx,
            Some(BlockWaitTimeout { block_number: 2, timeout: Duration::from_millis(1000) }),
        )
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(400)).await;

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
    let tx_hash = node.deploy_contract(&contract_code.init, alith_addr, Some(1)).await;
    tokio::time::sleep(Duration::from_millis(400)).await;

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
