use crate::utils::{TestNode, unwrap_response};
use alloy_primitives::{Address, B256, U256};
use alloy_rpc_types::{TransactionRequest, txpool::TxpoolStatus};
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
