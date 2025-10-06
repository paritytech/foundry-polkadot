use std::time::Duration;

use crate::utils::{TestNode, transaction_in_block, unwrap_response};
use alloy_primitives::{Address, B256, Bytes, U256};
use alloy_rpc_types::{Index, TransactionRequest};
use anvil_core::eth::EthRequest;
use anvil_polkadot::{
    api_server::revive_conversions::{AlloyU256, ReviveAddress},
    config::{AnvilNodeConfig, SubstrateNodeConfig},
};
use anvil_rpc::{
    error::{ErrorCode, RpcError},
    response::ResponseResult,
};
use polkadot_sdk::pallet_revive::{
    self,
    evm::{Account, Block, TransactionInfo},
};
use subxt::utils::H160;

#[tokio::test(flavor = "multi_thread")]
async fn test_get_chain_id() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // expected 420420420
    assert_eq!(
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthChainId(())).await.unwrap()).unwrap(),
        "0x190f1b44"
    );
    // expected 420420420
    assert_eq!(
        unwrap_response::<u64>(node.eth_rpc(EthRequest::EthNetworkId(())).await.unwrap()).unwrap(),
        0x190f1b44
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
        U256::from_str_radix("100000000000000000000000", 10).unwrap()
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
    let transaction = TransactionRequest::default()
        .value(transfer_amount)
        .from(Address::from(ReviveAddress::new(alith.address())))
        .to(Address::from(ReviveAddress::new(baltathar.address())));
    let tx_hash0 = node.send_transaction(transaction.clone()).await;
    let tx_hash1 = node.send_transaction(transaction.clone().nonce(1)).await;
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    let tx_hash2 = node.send_transaction(transaction.nonce(2)).await;
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    let hash1 = node.block_hash_by_number(1).await.unwrap();
    let hash2 = node.block_hash_by_number(2).await.unwrap();
    let block1 = node.get_block_by_hash(hash1).await;
    let block2 = node.get_block_by_hash(hash2).await;
    assert!(transaction_in_block(&block1.transactions, tx_hash0));
    assert!(transaction_in_block(&block1.transactions, tx_hash1));
    assert!(transaction_in_block(&block2.transactions, tx_hash2));
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
    let tx_hash = node.send_transaction(transaction).await;
    node.wait_for_block_with_timeout(1, std::time::Duration::from_secs(2)).await.unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
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
async fn test_send_to_uninitialized() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::SetAutomine(true)).await.unwrap()).unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let charleth = Account::from(subxt_signer::eth::dev::charleth());

    let transfer_amount = U256::from_str_radix("1600000000000000000", 10).unwrap();
    let transaction = TransactionRequest::default()
        .value(transfer_amount)
        .from(Address::from(ReviveAddress::new(alith.address())))
        .to(Address::from(ReviveAddress::new(charleth.address())));
    let _tx_hash = node.send_transaction(transaction).await;
    node.wait_for_block_with_timeout(1, std::time::Duration::from_secs(2)).await.unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));

    let alith_final_balance = node.get_balance(alith.address(), None).await;
    assert_eq!(node.get_balance(charleth.address(), None).await, transfer_amount);

    let charlet_initial_balance = node.get_balance(charleth.address(), None).await;
    let transfer_amount = U256::from_str_radix("100000000000", 10).unwrap();
    let transaction = TransactionRequest::default()
        .value(transfer_amount)
        .from(Address::from(ReviveAddress::new(charleth.address())))
        .to(Address::from(ReviveAddress::new(alith.address())));
    let tx_hash = node.send_transaction(transaction).await;
    node.wait_for_block_with_timeout(1, std::time::Duration::from_secs(2)).await.unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    let transaction_receipt = node.get_transaction_receipt(tx_hash).await;
    let alith_final_balance_2 = node.get_balance(alith.address(), None).await;
    let charlet_final_balance = node.get_balance(charleth.address(), None).await;
    assert_eq!(
        charlet_final_balance,
        charlet_initial_balance
            - transfer_amount
            - AlloyU256::from(
                transaction_receipt.gas_used * transaction_receipt.effective_gas_price
            )
            .inner()
    );
    assert_eq!(alith_final_balance_2, alith_final_balance + transfer_amount);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_estimate_gas() {
    // How do we even test this?
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
    let tx_hash = node.send_transaction(transaction).await;
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
    assert!(transaction_in_block(&block_by_number.transactions, tx_hash));
    // Check that GetBlockByNumber fails if the block number does not fit in u32
    // TODO: expand the error conversion for ReviveRpc type
    assert!(matches!(
        node.eth_rpc(EthRequest::EthGetBlockByNumber(
            alloy_eips::BlockNumberOrTag::Number(u64::MAX),
            true
        ))
        .await.unwrap(),
        ResponseResult::Error(RpcError {
            code: ErrorCode::InternalError,
            message,
            data: None
        })if message == "Client error: conversion failed"
    ));
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
    assert!(matches!( node
        .eth_rpc(EthRequest::EthGetTransactionCount(
            Address::from(ReviveAddress::new(alith.address())),
            Some(alloy_eips::BlockId::Number(alloy_eips::BlockNumberOrTag::Number(1))),
        ))
        .await
        .unwrap(), ResponseResult::Error(RpcError {
            code: ErrorCode::InternalError,
            message,
            data: None
        }) if message == "Client error: hash not found"
    ));

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
    let _tx_hash0 = node.send_transaction(transaction.clone()).await;
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    tokio::time::sleep(Duration::from_millis(400)).await;
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
    let _tx_hash0 = node.send_transaction(transaction.clone()).await;
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
    tokio::time::sleep(Duration::from_millis(400)).await;
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
    unwrap_response::<()>(node.eth_rpc(EthRequest::SetAutomine(true)).await.unwrap()).unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let (bytecode, tx_hash) = node.deploy_contract("dummy", alith.address(), 1).await;
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
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
        Bytes::from(bytecode),
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

//#[tokio::test(flavor = "multi_thread")]
//async fn test_get_storage_at() {
//    let anvil_node_config = AnvilNodeConfig::test_config();
//    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
//    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();
//    unwrap_response::<()>(node.eth_rpc(EthRequest::SetAutomine(true)).await.unwrap()).unwrap();
//
//    let alith = Account::from(subxt_signer::eth::dev::alith());
//    let (bytecode, tx_hash) = node.deploy_contract("storage_size", alith.address(), 1).await;
//    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
//    let receipt = node.get_transaction_receipt(tx_hash).await;
//    assert_eq!(receipt.status, Some(pallet_revive::U256::from(1)));
//    let contract_address = receipt.contract_address.unwrap();
//
//    // Storage key: [1, 0, 0, ..., 0]
//    let mut key_bytes = [0u8; 32];
//    key_bytes[0] = 1;
//    let storage_key = U256::from_be_bytes(key_bytes);
//
//    // Verify initial state
//    let initial = unwrap_response::<Bytes>(
//        node.eth_rpc(EthRequest::EthGetStorageAt(
//            Address::from(ReviveAddress::new(contract_address)),
//            storage_key,
//            None,
//        ))
//        .await
//        .unwrap(),
//    )
//    .unwrap();
//    assert_eq!(initial, Bytes::from(vec![0u8; 32]));
//}

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
    let tx_hash0 = node.send_transaction(transaction.clone()).await;
    let tx_hash1 = node
        .send_transaction(
            transaction
                .from(Address::from(ReviveAddress::new(baltathar.address())))
                .to(Address::from(ReviveAddress::new(alith.address()))),
        )
        .await;
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
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

    assert_eq!(first_hash, transaction_info_1.block_hash);
    assert_eq!(transaction_info_1.from, alith.address());
    assert_eq!(tx_hash0, transaction_info_1.hash);

    assert_eq!(first_hash, transaction_info_2.block_hash);
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
    let tx_hash0 = node.send_transaction(transaction.clone()).await;
    let tx_hash1 = node
        .send_transaction(
            transaction
                .from(Address::from(ReviveAddress::new(baltathar.address())))
                .to(Address::from(ReviveAddress::new(alith.address()))),
        )
        .await;
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;

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

    let first_hash = node.block_hash_by_number(1).await.unwrap();
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
    let tx_hash0 = node.send_transaction(transaction.clone()).await;
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;

    let transaction_info_1 = unwrap_response::<Option<TransactionInfo>>(
        node.eth_rpc(EthRequest::EthGetTransactionByHash(B256::from_slice(tx_hash0.as_ref())))
            .await
            .unwrap(),
    )
    .unwrap()
    .unwrap();

    let first_hash = node.block_hash_by_number(1).await.unwrap();
    assert_eq!(first_hash, transaction_info_1.block_hash);
    assert_eq!(transaction_info_1.from, alith.address());
    assert_eq!(tx_hash0, transaction_info_1.hash);
}
