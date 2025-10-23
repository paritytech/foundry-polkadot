use crate::utils::{unwrap_response, TestNode};
use alloy_primitives::{Address, B256, U256};
use alloy_rpc_types::{txpool::TxpoolStatus, TransactionRequest};
use anvil_core::eth::EthRequest;
use anvil_polkadot::{
    api_server::revive_conversions::ReviveAddress,
    config::{AnvilNodeConfig, SubstrateNodeConfig},
};
use polkadot_sdk::pallet_revive::evm::Account;

#[tokio::test(flavor = "multi_thread")]
async fn test_txpool_status() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config, substrate_node_config).await.unwrap();

    unwrap_response::<()>(node.eth_rpc(EthRequest::SetAutomine(true)).await.unwrap()).unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_addr = Address::from(ReviveAddress::new(alith.address()));
    let recipient_addr = Address::repeat_byte(0x42);

    let balance = node.get_balance(alith.address(), None).await;
    assert!(balance > U256::ZERO, "Alith should have balance");

    // Check initial pool status
    let status: TxpoolStatus =
        unwrap_response(node.eth_rpc(EthRequest::TxPoolStatus(())).await.unwrap()).unwrap();
    assert_eq!(status.pending, 0, "Pool should start empty");
    assert_eq!(status.queued, 0, "Pool should start empty");

    // Disable automine so transactions stay in pool
    unwrap_response::<()>(node.eth_rpc(EthRequest::SetAutomine(false)).await.unwrap()).unwrap();

    // Send 3 transactions
    for i in 0..3 {
        let tx = TransactionRequest::default()
            .from(alith_addr)
            .to(recipient_addr)
            .value(U256::from(1000 * (i + 1)))
            .nonce(i);
        node.send_transaction(tx, None).await.unwrap();
    }

    // Verify pool has 3 pending transactions
    let status: TxpoolStatus =
        unwrap_response(node.eth_rpc(EthRequest::TxPoolStatus(())).await.unwrap()).unwrap();
    assert_eq!(status.pending, 3, "Pool should have 3 pending transactions");
    assert_eq!(status.queued, 0, "Pool should have 0 queued transactions");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_drop_transaction() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config, substrate_node_config).await.unwrap();

    unwrap_response::<()>(node.eth_rpc(EthRequest::SetAutomine(true)).await.unwrap()).unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_addr = Address::from(ReviveAddress::new(alith.address()));
    let recipient_addr = Address::repeat_byte(0x42);

    let balance = node.get_balance(alith.address(), None).await;
    assert!(balance > U256::ZERO, "Alith should have balance");

    unwrap_response::<()>(node.eth_rpc(EthRequest::SetAutomine(false)).await.unwrap()).unwrap();

    // Send 2 transactions with nonce 0 and 1
    let tx1 = TransactionRequest::default()
        .from(alith_addr)
        .to(recipient_addr)
        .value(U256::from(1000))
        .nonce(0);
    let tx1_hash = node.send_transaction(tx1, None).await.unwrap();

    let tx2 = TransactionRequest::default()
        .from(alith_addr)
        .to(recipient_addr)
        .value(U256::from(2000))
        .nonce(1);
    let tx2_hash = node.send_transaction(tx2, None).await.unwrap();

    // Verify pool has 2 pending transactions
    let status: TxpoolStatus =
        unwrap_response(node.eth_rpc(EthRequest::TxPoolStatus(())).await.unwrap()).unwrap();
    assert_eq!(status.pending, 2, "Pool should have 2 pending transactions");

    // Drop transaction with nonce 1
    let tx2_hash_b256 = B256::from_slice(tx2_hash.0.as_ref());
    unwrap_response::<()>(node.eth_rpc(EthRequest::DropTransaction(tx2_hash_b256)).await.unwrap())
        .unwrap();

    // Verify only transaction with nonce 0 remains
    let status: TxpoolStatus =
        unwrap_response(node.eth_rpc(EthRequest::TxPoolStatus(())).await.unwrap()).unwrap();
    assert_eq!(status.pending, 1, "Pool should have 1 pending transaction after drop");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_drop_all_transactions() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config, substrate_node_config).await.unwrap();

    unwrap_response::<()>(node.eth_rpc(EthRequest::SetAutomine(true)).await.unwrap()).unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_addr = Address::from(ReviveAddress::new(alith.address()));
    let recipient_addr = Address::repeat_byte(0x42);

    let balance = node.get_balance(alith.address(), None).await;
    assert!(balance > U256::ZERO, "Alith should have balance");

    unwrap_response::<()>(node.eth_rpc(EthRequest::SetAutomine(false)).await.unwrap()).unwrap();

    // Send 3 transactions
    for i in 0..3 {
        let tx = TransactionRequest::default()
            .from(alith_addr)
            .to(recipient_addr)
            .value(U256::from(1000 * (i + 1)))
            .nonce(i);
        node.send_transaction(tx, None).await.unwrap();
    }

    // Verify pool has 3 pending transactions
    let status: TxpoolStatus =
        unwrap_response(node.eth_rpc(EthRequest::TxPoolStatus(())).await.unwrap()).unwrap();
    assert_eq!(status.pending, 3, "Pool should have 3 pending transactions");

    // Drop all transactions
    unwrap_response::<()>(node.eth_rpc(EthRequest::DropAllTransactions()).await.unwrap()).unwrap();

    // Verify pool is empty
    let status: TxpoolStatus =
        unwrap_response(node.eth_rpc(EthRequest::TxPoolStatus(())).await.unwrap()).unwrap();
    assert_eq!(status.pending, 0, "Pool should be empty after dropping all");
    assert_eq!(status.queued, 0, "Pool should be empty after dropping all");
}
