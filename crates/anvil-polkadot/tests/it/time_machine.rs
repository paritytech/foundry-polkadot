use std::time::{SystemTime, UNIX_EPOCH};

use crate::utils::{TestNode, assert_with_tolerance, unwrap_response};
use alloy_primitives::U256;
use anvil_core::eth::EthRequest;
use anvil_polkadot::config::{AnvilNodeConfig, SubstrateNodeConfig};
use anvil_rpc::error::ErrorCode;

// Tests --------- EvmSetTime

#[tokio::test(flavor = "multi_thread")]
async fn test_evm_set_time_invalid_param() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config, substrate_node_config).await.unwrap();
    // Try to set the time too far ahead.
    let err = unwrap_response::<u64>(
        node.eth_rpc(EthRequest::EvmSetTime(U256::from(u64::MAX))).await.unwrap(),
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidParams);
    assert_eq!(err.message, "The timestamp is too big");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_evm_set_time_in_the_past() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config, substrate_node_config).await.unwrap();

    // Mine the first block
    let _ = node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap();

    // Set the timestamp in the past
    assert_eq!(
        unwrap_response::<u64>(
            node.eth_rpc(EthRequest::EvmSetTime(U256::from(10000u64))).await.unwrap()
        )
        .unwrap(),
        0
    );
    // Sleep for a second to check that the time flows correctly after EvmSetTime
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let _ = node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap();
    let second_hash = node.block_hash_by_number(2).await.unwrap();
    let second_timestamp = node.get_decoded_timestamp(Some(second_hash)).await;
    assert_with_tolerance(second_timestamp, 10001000, 200, "Failed to produce block in time.");

    // Set the timestamp further in the past
    assert_eq!(
        unwrap_response::<u64>(
            node.eth_rpc(EthRequest::EvmSetTime(U256::from(1000))).await.unwrap()
        )
        .unwrap(),
        0
    );
    let _ = node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap();
    let third_hash = node.block_hash_by_number(3).await.unwrap();
    let third_timestamp = node.get_decoded_timestamp(Some(third_hash)).await;
    assert_with_tolerance(third_timestamp, 1000000, 200, "Difference between blocks is too large.");

    // Check that the time manipulation is applied to the previously injected time.
    // Current offset should be ~ time since EPOCH - 1000
    let current_offset = unwrap_response::<i64>(
        node.eth_rpc(EthRequest::EvmIncreaseTime(U256::from(0))).await.unwrap(),
    )
    .unwrap();
    // The new offset should be ~time since EPOCH - 2000
    assert_with_tolerance(
        unwrap_response::<i64>(
            node.eth_rpc(EthRequest::EvmIncreaseTime(U256::from(1))).await.unwrap(),
        )
        .unwrap(),
        current_offset + 1,
        200,
        "message",
    );
    let _ = node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap();

    let fourth_hash = node.block_hash_by_number(4).await.unwrap();
    let fourth_timestamp = node.get_decoded_timestamp(Some(fourth_hash)).await;
    assert_with_tolerance(
        fourth_timestamp.saturating_sub(third_timestamp),
        1000,
        200,
        "Difference between blocks timestamp is too large.",
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_evm_set_time() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config, substrate_node_config).await.unwrap();

    let _ = node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap();
    let first_hash = node.block_hash_by_number(1).await.unwrap();
    let first_timestamp = node.get_decoded_timestamp(Some(first_hash)).await;

    // Set the timestamp in the future
    let new_timestamp = first_timestamp.saturating_div(1000).saturating_add(3600);
    assert_with_tolerance(
        unwrap_response::<u64>(
            node.eth_rpc(EthRequest::EvmSetTime(U256::from(new_timestamp))).await.unwrap(),
        )
        .unwrap(),
        3600,
        1,
        "Wrong offset",
    );

    let _ = node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap();
    let second_hash = node.block_hash_by_number(2).await.unwrap();
    let second_timestamp = node.get_decoded_timestamp(Some(second_hash)).await;
    assert_with_tolerance(
        second_timestamp.saturating_sub(first_timestamp).saturating_div(1000),
        3600,
        1,
        "Wrong timestamp",
    );
}

// Tests --------- EvmIncreaseTime
#[tokio::test(flavor = "multi_thread")]
async fn test_evm_increase_time() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config, substrate_node_config).await.unwrap();

    let _ = node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap();
    let first_hash = node.block_hash_by_number(1).await.unwrap();
    let first_timestamp = node.get_decoded_timestamp(Some(first_hash)).await;

    assert_with_tolerance(
        unwrap_response::<i64>(
            node.eth_rpc(EthRequest::EvmIncreaseTime(U256::from(3600))).await.unwrap(),
        )
        .unwrap(),
        3600,
        1,
        "Wrong offset when increasing the timestamp.",
    );
    let _ = node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap();
    let second_hash = node.block_hash_by_number(2).await.unwrap();
    let second_timestamp = node.get_decoded_timestamp(Some(second_hash)).await;
    assert_with_tolerance(
        second_timestamp.saturating_sub(first_timestamp).saturating_div(1000),
        3600,
        1,
        "Wrong timestamp",
    );
    assert_with_tolerance(
        unwrap_response::<i64>(
            node.eth_rpc(EthRequest::EvmIncreaseTime(U256::from(3600))).await.unwrap(),
        )
        .unwrap(),
        7200,
        10,
        "Wrong offset when increasing the timestamp.",
    );
    let _ = node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap();
    let third_hash = node.block_hash_by_number(3).await.unwrap();
    let third_timestamp = node.get_decoded_timestamp(Some(third_hash)).await;
    assert_with_tolerance(
        third_timestamp.saturating_sub(first_timestamp).saturating_div(1000),
        7200,
        1,
        "Wrong timestamp",
    );
    assert_with_tolerance(
        third_timestamp.saturating_sub(second_timestamp).saturating_div(1000),
        3600,
        10,
        "Wrong timestamp",
    );
}

// Tests --------- EvmSetNextBlockTimeStamp

#[tokio::test(flavor = "multi_thread")]
async fn test_evm_set_next_block_timestamp() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config, substrate_node_config).await.unwrap();

    let timestamp =
        SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs();
    let next_timestamp = timestamp + 3600;

    node.eth_rpc(EthRequest::EvmSetNextBlockTimeStamp(U256::from(next_timestamp))).await.unwrap();
    let _ = node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap();
    let first_hash = node.block_hash_by_number(1).await.unwrap();
    let first_timestamp = node.get_decoded_timestamp(Some(first_hash)).await;
    assert_with_tolerance(
        first_timestamp.saturating_sub(timestamp.saturating_mul(1000)),
        3600000,
        200,
        "The time was not moved into the future",
    );
}

// Tests --------- EvmSetBlockTimeStampInterval & EvmRemoveBlockTimeStampInterval

#[tokio::test(flavor = "multi_thread")]
async fn test_evm_set_remove_block_timestamp_interval() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config, substrate_node_config).await.unwrap();

    unwrap_response::<()>(
        node.eth_rpc(EthRequest::EvmSetBlockTimeStampInterval(3600)).await.unwrap(),
    )
    .unwrap();
    let _ = node.eth_rpc(EthRequest::Mine(Some(U256::from(2)), None)).await.unwrap();
    let hash2 = node.block_hash_by_number(2).await.unwrap();
    let hash1 = node.block_hash_by_number(1).await.unwrap();
    let timestamp1 = node.get_decoded_timestamp(Some(hash1)).await;
    let timestamp2 = node.get_decoded_timestamp(Some(hash2)).await;
    assert_with_tolerance(
        timestamp2.saturating_sub(timestamp1),
        3600000,
        100,
        "Interval between the blocks if greater than the desired value.",
    );
    assert!(
        unwrap_response::<bool>(
            node.eth_rpc(EthRequest::EvmRemoveBlockTimeStampInterval(())).await.unwrap()
        )
        .unwrap()
    );

    let _ = node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap();
    let hash3 = node.block_hash_by_number(3).await.unwrap();
    let timestamp3 = node.get_decoded_timestamp(Some(hash3)).await;
    assert!(timestamp3.saturating_sub(timestamp2).saturating_div(1000) < 3600);
}
