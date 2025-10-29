use crate::utils::{unwrap_response, TestNode};
use alloy_primitives::{Address, B256, U256};
use alloy_rpc_types::{
    txpool::{TxpoolContent, TxpoolInspect, TxpoolStatus},
    TransactionRequest,
};
use anvil_core::eth::EthRequest;
use anvil_polkadot::{
    api_server::revive_conversions::ReviveAddress,
    config::{AnvilNodeConfig, SubstrateNodeConfig},
};
use polkadot_sdk::pallet_revive::evm::{Account, TransactionInfo};

#[tokio::test(flavor = "multi_thread")]
async fn test_txpool_status() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config, substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_addr = Address::from(ReviveAddress::new(alith.address()));
    let recipient_addr = Address::repeat_byte(0x42);

    let status: TxpoolStatus =
        unwrap_response(node.eth_rpc(EthRequest::TxPoolStatus(())).await.unwrap()).unwrap();
    assert_eq!(status.pending, 0);
    assert_eq!(status.queued, 0);

    for i in 0..3 {
        let tx = TransactionRequest::default()
            .from(alith_addr)
            .to(recipient_addr)
            .value(U256::from(1000 * (i + 1)))
            .nonce(i);
        node.send_transaction(tx, None).await.unwrap();
    }

    let status: TxpoolStatus =
        unwrap_response(node.eth_rpc(EthRequest::TxPoolStatus(())).await.unwrap()).unwrap();
    assert_eq!(status.pending, 3);
    assert_eq!(status.queued, 0);

    let tx_future = TransactionRequest::default()
        .from(alith_addr)
        .to(recipient_addr)
        .value(U256::from(5000))
        .nonce(5);
    node.send_transaction(tx_future, None).await.unwrap();

    let status: TxpoolStatus =
        unwrap_response(node.eth_rpc(EthRequest::TxPoolStatus(())).await.unwrap()).unwrap();
    assert_eq!(status.pending, 3);
    assert_eq!(status.queued, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_drop_transaction() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config, substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_addr = Address::from(ReviveAddress::new(alith.address()));
    let recipient_addr = Address::repeat_byte(0x42);

    let tx1 =
        TransactionRequest::default().from(alith_addr).to(recipient_addr).value(U256::from(1000));
    node.send_transaction(tx1, None).await.unwrap();

    let tx2 = TransactionRequest::default()
        .from(alith_addr)
        .to(recipient_addr)
        .value(U256::from(2000))
        .nonce(1);
    let tx2_hash = node.send_transaction(tx2, None).await.unwrap();

    let tx_future = TransactionRequest::default()
        .from(alith_addr)
        .to(recipient_addr)
        .value(U256::from(5000))
        .nonce(5);
    let tx_future_hash = node.send_transaction(tx_future, None).await.unwrap();

    let status: TxpoolStatus =
        unwrap_response(node.eth_rpc(EthRequest::TxPoolStatus(())).await.unwrap()).unwrap();
    assert_eq!(status.pending, 2);
    assert_eq!(status.queued, 1);

    let tx2_hash_b256 = B256::from_slice(tx2_hash.0.as_ref());
    let dropped_hash = unwrap_response::<Option<B256>>(
        node.eth_rpc(EthRequest::DropTransaction(tx2_hash_b256)).await.unwrap(),
    )
    .unwrap();
    assert_eq!(dropped_hash, Some(tx2_hash_b256));

    let status: TxpoolStatus =
        unwrap_response(node.eth_rpc(EthRequest::TxPoolStatus(())).await.unwrap()).unwrap();
    assert_eq!(status.pending, 1);
    assert_eq!(status.queued, 1);

    let tx_future_hash_b256 = B256::from_slice(tx_future_hash.0.as_ref());
    let dropped_hash = unwrap_response::<Option<B256>>(
        node.eth_rpc(EthRequest::DropTransaction(tx_future_hash_b256)).await.unwrap(),
    )
    .unwrap();
    assert_eq!(dropped_hash, Some(tx_future_hash_b256));

    let status: TxpoolStatus =
        unwrap_response(node.eth_rpc(EthRequest::TxPoolStatus(())).await.unwrap()).unwrap();
    assert_eq!(status.pending, 1);
    assert_eq!(status.queued, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_drop_all_transactions() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config, substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_addr = Address::from(ReviveAddress::new(alith.address()));
    let recipient_addr = Address::repeat_byte(0x42);

    for i in 0..3 {
        let tx = TransactionRequest::default()
            .from(alith_addr)
            .to(recipient_addr)
            .value(U256::from(1000 * (i + 1)))
            .nonce(i);
        node.send_transaction(tx, None).await.unwrap();
    }

    let tx_future = TransactionRequest::default()
        .from(alith_addr)
        .to(recipient_addr)
        .value(U256::from(5000))
        .nonce(5);
    node.send_transaction(tx_future, None).await.unwrap();

    let status: TxpoolStatus =
        unwrap_response(node.eth_rpc(EthRequest::TxPoolStatus(())).await.unwrap()).unwrap();
    assert_eq!(status.pending, 3);
    assert_eq!(status.queued, 1);

    unwrap_response::<()>(node.eth_rpc(EthRequest::DropAllTransactions()).await.unwrap()).unwrap();

    let status: TxpoolStatus =
        unwrap_response(node.eth_rpc(EthRequest::TxPoolStatus(())).await.unwrap()).unwrap();
    assert_eq!(status.pending, 0);
    assert_eq!(status.queued, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_txpool_inspect() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config, substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_addr = Address::from(ReviveAddress::new(alith.address()));
    let recipient_addr = Address::repeat_byte(0x42);

    let inspect: TxpoolInspect =
        unwrap_response(node.eth_rpc(EthRequest::TxPoolInspect(())).await.unwrap()).unwrap();
    assert!(inspect.pending.is_empty());
    assert!(inspect.queued.is_empty());

    for i in 0..3 {
        let tx = TransactionRequest::default()
            .from(alith_addr)
            .to(recipient_addr)
            .value(U256::from(1000 * (i + 1)))
            .nonce(i);
        node.send_transaction(tx, None).await.unwrap();
    }

    let tx_future = TransactionRequest::default()
        .from(alith_addr)
        .to(recipient_addr)
        .value(U256::from(5000))
        .nonce(5);
    node.send_transaction(tx_future, None).await.unwrap();

    let inspect: TxpoolInspect =
        unwrap_response(node.eth_rpc(EthRequest::TxPoolInspect(())).await.unwrap()).unwrap();

    assert_eq!(inspect.pending.len(), 1);
    assert_eq!(inspect.queued.len(), 1);

    let pending_txs = inspect.pending.get(&alith_addr).unwrap();
    assert_eq!(pending_txs.len(), 3);

    for i in 0..3 {
        let summary = pending_txs.get(&i.to_string()).unwrap();
        assert_eq!(summary.to.unwrap(), recipient_addr);
        assert_eq!(summary.value, U256::from(1000 * (i + 1)));
        assert!(summary.gas > 0);
    }

    let queued_txs = inspect.queued.get(&alith_addr).unwrap();
    assert_eq!(queued_txs.len(), 1);

    let summary = queued_txs.get("5").unwrap();
    assert_eq!(summary.to.unwrap(), recipient_addr);
    assert_eq!(summary.value, U256::from(5000));
    assert!(summary.gas > 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_txpool_content() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config, substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_addr = Address::from(ReviveAddress::new(alith.address()));
    let recipient_addr = Address::repeat_byte(0x42);

    let content: TxpoolContent<TransactionInfo> =
        unwrap_response(node.eth_rpc(EthRequest::TxPoolContent(())).await.unwrap()).unwrap();
    assert!(content.pending.is_empty());
    assert!(content.queued.is_empty());

    for i in 0..3 {
        let tx = TransactionRequest::default()
            .from(alith_addr)
            .to(recipient_addr)
            .value(U256::from(1000 * (i + 1)))
            .nonce(i);
        node.send_transaction(tx, None).await.unwrap();
    }

    let tx_future = TransactionRequest::default()
        .from(alith_addr)
        .to(recipient_addr)
        .value(U256::from(5000))
        .nonce(5);
    node.send_transaction(tx_future, None).await.unwrap();

    let content: TxpoolContent<TransactionInfo> =
        unwrap_response(node.eth_rpc(EthRequest::TxPoolContent(())).await.unwrap()).unwrap();

    assert_eq!(content.pending.len(), 1);
    assert_eq!(content.queued.len(), 1);

    let pending_txs = content.pending.get(&alith_addr).unwrap();
    assert_eq!(pending_txs.len(), 3);

    for i in 0..3 {
        let tx_info = pending_txs.get(&i.to_string()).unwrap();
        let from_addr = Address::from_slice(tx_info.from.as_bytes());
        assert_eq!(from_addr, alith_addr);
        assert!(tx_info.hash != Default::default());
    }

    let queued_txs = content.queued.get(&alith_addr).unwrap();
    assert_eq!(queued_txs.len(), 1);

    let tx_info = queued_txs.get("5").unwrap();
    let from_addr = Address::from_slice(tx_info.from.as_bytes());
    assert_eq!(from_addr, alith_addr);
    assert!(tx_info.hash != Default::default());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_remove_pool_transactions() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config, substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_addr = Address::from(ReviveAddress::new(alith.address()));

    let baltathar = Account::from(subxt_signer::eth::dev::baltathar());
    let baltathar_addr = Address::from(ReviveAddress::new(baltathar.address()));

    let recipient_addr = Address::repeat_byte(0x42);

    // Send 3 transactions from Alith
    for i in 0..3 {
        let tx = TransactionRequest::default()
            .from(alith_addr)
            .to(recipient_addr)
            .value(U256::from(1000 * (i + 1)))
            .nonce(i);
        node.send_transaction(tx, None).await.unwrap();
    }

    // Send 2 transactions from Baltathar
    for i in 0..2 {
        let tx = TransactionRequest::default()
            .from(baltathar_addr)
            .to(recipient_addr)
            .value(U256::from(2000 * (i + 1)))
            .nonce(i);
        node.send_transaction(tx, None).await.unwrap();
    }

    let status: TxpoolStatus =
        unwrap_response(node.eth_rpc(EthRequest::TxPoolStatus(())).await.unwrap()).unwrap();
    assert_eq!(status.pending, 5);
    assert_eq!(status.queued, 0);

    // Remove all transactions from Alith
    unwrap_response::<()>(
        node.eth_rpc(EthRequest::RemovePoolTransactions(alith_addr)).await.unwrap(),
    )
    .unwrap();

    let status: TxpoolStatus =
        unwrap_response(node.eth_rpc(EthRequest::TxPoolStatus(())).await.unwrap()).unwrap();
    assert_eq!(status.pending, 2);
    assert_eq!(status.queued, 0);

    // Verify only Baltathar's transactions remain
    let content: TxpoolContent<TransactionInfo> =
        unwrap_response(node.eth_rpc(EthRequest::TxPoolContent(())).await.unwrap()).unwrap();

    assert_eq!(content.pending.len(), 1);
    assert!(content.pending.contains_key(&baltathar_addr));
    assert!(!content.pending.contains_key(&alith_addr));

    let baltathar_txs = content.pending.get(&baltathar_addr).unwrap();
    assert_eq!(baltathar_txs.len(), 2);
}
