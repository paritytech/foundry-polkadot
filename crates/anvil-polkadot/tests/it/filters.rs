use crate::{
    revert::{revert, snapshot},
    utils::{TestNode, unwrap_response},
};
use alloy_primitives::U256;
use anvil_core::eth::EthRequest;
use anvil_polkadot::{
    api_server::filters::Filters,
    config::{AnvilNodeConfig, SubstrateNodeConfig},
};
use subxt::utils::H256;

#[tokio::test(flavor = "multi_thread")]
async fn test_block_filter_receives_new_blocks() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Create a block filter
    let id =
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthNewBlockFilter(())).await.unwrap())
            .unwrap();

    // Mine a new block
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    // Get block hash
    let block_hash = node.block_hash_by_number(1).await.unwrap();

    let block_hashes_notified = unwrap_response::<Vec<H256>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(id.clone())).await.unwrap(),
    )
    .unwrap();

    // Check that hash for block number 1 is in between hashes we were notified about
    assert!(block_hashes_notified.contains(&block_hash));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_filter_returns_empty_when_no_new_blocks() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Create a block filter
    let id =
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthNewBlockFilter(())).await.unwrap())
            .unwrap();

    let block_hashes_notified = unwrap_response::<Vec<H256>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(id.clone())).await.unwrap(),
    )
    .unwrap();
    assert_eq!(block_hashes_notified.len(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_filter_only_returns_new_blocks_since_last_poll() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Create a block filter
    let id =
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthNewBlockFilter(())).await.unwrap())
            .unwrap();

    // Mine a new block
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    let block_hashes_notified = unwrap_response::<Vec<H256>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(id.clone())).await.unwrap(),
    )
    .unwrap();
    assert!(!block_hashes_notified.is_empty());
    assert!(block_hashes_notified.contains(&node.block_hash_by_number(1).await.unwrap()));
    // Mine some more blocks
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(5)), None)).await.unwrap())
        .unwrap();
    let block_hashes_notified = unwrap_response::<Vec<H256>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(id.clone())).await.unwrap(),
    )
    .unwrap();

    for (index, hash) in block_hashes_notified.iter().enumerate() {
        assert_eq!(*hash, node.block_hash_by_number(index as u32 + 2).await.unwrap());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_uninstall_filter() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Create a block filter
    let id =
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthNewBlockFilter(())).await.unwrap())
            .unwrap();
    // Check that we can successfully rmeove the filter
    assert!(
        unwrap_response::<bool>(
            node.eth_rpc(EthRequest::EthUninstallFilter(id.clone())).await.unwrap()
        )
        .unwrap()
    );
    // Check that we can not remove the filter a second time
    assert!(
        !unwrap_response::<bool>(
            node.eth_rpc(EthRequest::EthUninstallFilter(id.clone())).await.unwrap()
        )
        .unwrap()
    );

    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(5)), None)).await.unwrap())
        .unwrap();
    // Try to poll a filter that does not exist
    assert_eq!(
        unwrap_response::<Vec<H256>>(
            node.eth_rpc(EthRequest::EthGetFilterChanges(id.clone())).await.unwrap(),
        )
        .unwrap()
        .len(),
        0
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_multiple_filters_receive_same_blocks() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Create two block filters
    let id1 =
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthNewBlockFilter(())).await.unwrap())
            .unwrap();
    let id2 =
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthNewBlockFilter(())).await.unwrap())
            .unwrap();

    // Mine a few blocks
    unwrap_response::<()>(
        node.eth_rpc(EthRequest::Mine(Some(U256::from(10)), None)).await.unwrap(),
    )
    .unwrap();
    let block_hashes_notified1 = unwrap_response::<Vec<H256>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(id1.clone())).await.unwrap(),
    )
    .unwrap();
    let block_hashes_notified2 = unwrap_response::<Vec<H256>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(id2.clone())).await.unwrap(),
    )
    .unwrap();
    assert!(!block_hashes_notified1.is_empty());
    assert_eq!(block_hashes_notified1, block_hashes_notified2);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_filter_after_snapshot() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Create a block filter
    let id =
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthNewBlockFilter(())).await.unwrap())
            .unwrap();
    // Mine two blocks
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(2)), None)).await.unwrap())
        .unwrap();
    // Create a snapshot
    let zero = snapshot(&mut node, U256::ZERO).await;
    // Mine two more blocks (bn3 and bn4)
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(2)), None)).await.unwrap())
        .unwrap();
    // Get the hashes of the mined blocks
    let block_hashes_notified = unwrap_response::<Vec<H256>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(id.clone())).await.unwrap(),
    )
    .unwrap();

    // Check that the received hashes are equal with the hashes of blocks 1..4
    for (index, hash) in block_hashes_notified.iter().enumerate() {
        assert_eq!(*hash, node.block_hash_by_number(index as u32).await.unwrap());
    }
    // Go back to the block number 2.
    revert(&mut node, zero, 2, true).await;
    // Mine blocks [3,4,5,6,7]
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(5)), None)).await.unwrap())
        .unwrap();
    // Get the hashes of the mined blocks
    let block_hashes_notified = unwrap_response::<Vec<H256>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(id.clone())).await.unwrap(),
    )
    .unwrap();
    // Check that the received hashes are equal with the hashes of blocks 3..7
    for (index, hash) in block_hashes_notified.iter().enumerate() {
        assert_eq!(*hash, node.block_hash_by_number(index as u32 + 3).await.unwrap());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_filter_is_evicted() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new_inner(
        anvil_node_config.clone(),
        substrate_node_config,
        Filters::with_keepalive(std::time::Duration::from_secs(2)),
    )
    .await
    .unwrap();

    // Create a block filter
    let id =
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthNewBlockFilter(())).await.unwrap())
            .unwrap();
    // Mine a new block
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    // Get the hashes of the mined blocks
    let block_hashes_notified = unwrap_response::<Vec<H256>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(id.clone())).await.unwrap(),
    )
    .unwrap();
    assert!(!block_hashes_notified.is_empty());
    // Wait for the filter to expire
    tokio::time::sleep(std::time::Duration::from_secs(4)).await;
    // Mine five blocks
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(5)), None)).await.unwrap())
        .unwrap();
    let block_hashes_notified = unwrap_response::<Vec<H256>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(id.clone())).await.unwrap(),
    )
    .unwrap();
    assert_eq!(block_hashes_notified.len(), 0);
}
