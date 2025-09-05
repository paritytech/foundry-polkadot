use crate::utils::{assert_with_tolerance, response_result_success_inner, TestNode};
use alloy_primitives::U256;
use anvil_core::eth::EthRequest;
use anvil_polkadot::config::{AnvilNodeConfig, SubstrateNodeConfig};
use anvil_rpc::{
    error::{ErrorCode, RpcError},
    response::ResponseResult,
};
use serde_json::json;
use std::time::SystemTime;

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_mining() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config, substrate_node_config).await.unwrap();

    // Assert that we are in the correct mining mode
    assert!(!response_result_success_inner::<bool>(
        node.eth_rpc(EthRequest::GetAutoMine(())).await.unwrap()
    )
    .unwrap());
    assert_eq!(
        response_result_success_inner::<Option<u64>>(
            node.eth_rpc(EthRequest::GetIntervalMining(())).await.unwrap()
        )
        .unwrap(),
        None
    );

    assert!(matches!(
        node.eth_rpc(EthRequest::Mine(Some(U256::from(u64::MAX)), None)).await.unwrap(),
        ResponseResult::Error(RpcError {
            code: ErrorCode::InvalidParams,
            message,
            data: None
        }) if message == "The number of blocks is too large"
    ));
    assert!(matches!(
        node.eth_rpc(EthRequest::Mine(None, Some(U256::from(u64::MAX)))).await.unwrap(),
        ResponseResult::Error(RpcError {
            code: ErrorCode::InvalidParams,
            message,
            data: None
        }) if message == "The interval between blocks is too large"
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_manual_mining_with_no_of_blocks() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config, substrate_node_config).await.unwrap();

    assert_eq!(
        node.substrate_rpc("chain_getHeader", json!([]))
            .await
            .unwrap()
            .get("number")
            .and_then(|v| v.as_str())
            .unwrap(),
        "0x0"
    );

    assert!(matches!(
        node.eth_rpc(EthRequest::Mine(Some(U256::from(2)), None)).await.unwrap(),
        ResponseResult::Success(_)
    ));
    assert_eq!(
        node.substrate_rpc("chain_getHeader", json!([]))
            .await
            .unwrap()
            .get("number")
            .and_then(|v| v.as_str())
            .unwrap(),
        "0x2"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_manual_mining_with_interval() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config, substrate_node_config).await.unwrap();

    assert!(!response_result_success_inner::<bool>(
        node.eth_rpc(EthRequest::GetAutoMine(())).await.unwrap()
    )
    .unwrap());
    assert_eq!(
        response_result_success_inner::<Option<u64>>(
            node.eth_rpc(EthRequest::GetIntervalMining(())).await.unwrap()
        )
        .unwrap(),
        None
    );

    // Manually mine three blocks and force the timestamp to be increasing with 3 seconds.
    assert!(matches!(
        node.eth_rpc(EthRequest::Mine(Some(U256::from(3)), Some(U256::from(3)))).await.unwrap(),
        ResponseResult::Success(_)
    ));
    let hash3 = node.block_hash_by_number(3).await.unwrap();
    let hash2 = node.block_hash_by_number(2).await.unwrap();
    let hash1 = node.block_hash_by_number(1).await.unwrap();
    let timestamp1 = node.get_decoded_timestamp(Some(hash1)).await;
    let timestamp2 = node.get_decoded_timestamp(Some(hash2)).await;
    let timestamp3 = node.get_decoded_timestamp(Some(hash3)).await;
    assert_with_tolerance(
        timestamp2.saturating_sub(timestamp1),
        3000,
        100,
        "Interval between the blocks if greater than the desired value.",
    );
    assert_with_tolerance(
        timestamp3.saturating_sub(timestamp2),
        3000,
        100,
        "Interval between the blocks if greater than the desired value.",
    );
    assert_with_tolerance(
        timestamp3.saturating_sub(timestamp1),
        6000,
        100,
        "Interval between the blocks if greater than the desired value.",
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_interval_mining() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config, substrate_node_config).await.unwrap();
    // enable interval mining
    assert!(matches!(
        node.eth_rpc(EthRequest::SetIntervalMining(3)).await.unwrap(),
        ResponseResult::Success(_)
    ));

    assert_eq!(
        response_result_success_inner::<u64>(
            node.eth_rpc(EthRequest::GetIntervalMining(())).await.unwrap()
        )
        .unwrap(),
        3
    );

    let before_mining = SystemTime::now();
    node.wait_for_block_with_number(3).await.unwrap();
    let after_mining = SystemTime::now();
    assert_eq!(
        node.substrate_rpc("chain_getHeader", json!([]))
            .await
            .unwrap()
            .get("number")
            .and_then(|v| v.as_str())
            .unwrap(),
        "0x3"
    );
    assert_with_tolerance(
        after_mining.duration_since(before_mining).unwrap().as_millis(),
        9000,
        500,
        "Interval between the blocks if greater than the desired value.",
    );
    let hash3 = node.block_hash_by_number(3).await.unwrap();
    let hash2 = node.block_hash_by_number(2).await.unwrap();
    let hash1 = node.block_hash_by_number(1).await.unwrap();
    let timestamp1 = node.get_decoded_timestamp(Some(hash1)).await;
    let timestamp2 = node.get_decoded_timestamp(Some(hash2)).await;
    let timestamp3 = node.get_decoded_timestamp(Some(hash3)).await;
    assert_with_tolerance(
        timestamp2.saturating_sub(timestamp1),
        3000,
        100,
        "Interval between the blocks if greater than the desired value.",
    );
    assert_with_tolerance(
        timestamp3.saturating_sub(timestamp2),
        3000,
        100,
        "Interval between the blocks if greater than the desired value.",
    );
    assert_with_tolerance(
        timestamp3.saturating_sub(timestamp1),
        6000,
        100,
        "Interval between the blocks if greater than the desired value.",
    );
}

// TODO: test for auto mine
// TODO: test for mixed mining
// TODO: test for evm_mine
// TODO: test for evm_mine_detailed
