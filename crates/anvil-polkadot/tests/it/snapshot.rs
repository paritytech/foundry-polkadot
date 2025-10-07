use crate::utils::{TestNode, unwrap_response};
use alloy_primitives::U256;
use anvil_core::eth::EthRequest;
use anvil_polkadot::config::{AnvilNodeConfig, SubstrateNodeConfig};

#[tokio::test(flavor = "multi_thread")]
async fn test_evm_revert() {
    let anvil_node_config = AnvilNodeConfig::test_config().with_no_mining(true);
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Assert on initial best block number.
    assert_eq!(node.best_block_number().await, 0);

    // Mine 5 blocks and assert on the new best block.
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(5)), None)).await.unwrap())
        .unwrap();
    assert_eq!(node.best_block_number().await, 5);

    // Snapshot at block number 5.
    let id = unwrap_response::<String>(node.eth_rpc(EthRequest::EvmSnapshot(())).await.unwrap())
        .unwrap();
    assert_eq!(id, "0x1".to_string());

    // Mine 5 more blocks.
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(5)), None)).await.unwrap())
        .unwrap();
    assert_eq!(node.best_block_number().await, 10);

    // Snapshot again at block number 10.
    let id = unwrap_response::<String>(node.eth_rpc(EthRequest::EvmSnapshot(())).await.unwrap())
        .unwrap();
    assert_eq!(id, "0x2".to_string());
    assert_eq!(node.best_block_number().await, 10);

    // Mine 5 more blocks.
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(5)), None)).await.unwrap())
        .unwrap();
    assert_eq!(node.best_block_number().await, 15);

    // Revert to the second snapshot and assert best block number is 10.
    let snapshot_id = U256::from_str_radix(id.trim_start_matches("0x"), 16).unwrap();
    assert_eq!(snapshot_id, U256::from(2));
    let success =
        unwrap_response::<bool>(node.eth_rpc(EthRequest::EvmRevert(snapshot_id)).await.unwrap())
            .unwrap();
    assert!(success);
    assert_eq!(node.best_block_number().await, 10);

    // Check mining works fine after reverting.
    unwrap_response::<()>(
        node.eth_rpc(EthRequest::Mine(Some(U256::from(10)), None)).await.unwrap(),
    )
    .unwrap();
    assert_eq!(node.best_block_number().await, 20);

    // Test the case of revert -> mine -> snapshot -> revert (the block number remains the same
    // when snapshot was done).
    let id = unwrap_response::<String>(node.eth_rpc(EthRequest::EvmSnapshot(())).await.unwrap())
        .unwrap();
    assert_eq!(id, "0x3".to_string());
    assert_eq!(node.best_block_number().await, 20);

    let snapshot_id = U256::from_str_radix(id.trim_start_matches("0x"), 16).unwrap();
    assert_eq!(snapshot_id, U256::from(3));
    let success =
        unwrap_response::<bool>(node.eth_rpc(EthRequest::EvmRevert(snapshot_id)).await.unwrap())
            .unwrap();
    assert!(success);
    assert_eq!(node.best_block_number().await, 20);

    // Test the case of revert id -> revert same id.
    let success =
        unwrap_response::<bool>(node.eth_rpc(EthRequest::EvmRevert(U256::ONE)).await.unwrap())
            .unwrap();
    assert!(success);
    assert_eq!(node.best_block_number().await, 5);

    let success =
        unwrap_response::<bool>(node.eth_rpc(EthRequest::EvmRevert(U256::ONE)).await.unwrap())
            .unwrap();
    assert!(!success);

    // Test reverting down to genesis.
    // The snapshot at genesis block is automatically created
    // at node startup.
    let success =
        unwrap_response::<bool>(node.eth_rpc(EthRequest::EvmRevert(U256::ZERO)).await.unwrap())
            .unwrap();
    assert!(success);
    assert_eq!(node.best_block_number().await, 0);
}
