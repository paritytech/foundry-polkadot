use std::time::Duration;

use crate::utils::{TestNode, unwrap_response};
use alloy_primitives::U256;
use anvil_core::eth::EthRequest;
use anvil_polkadot::config::{AnvilNodeConfig, SubstrateNodeConfig};

#[tokio::test(flavor = "multi_thread")]
async fn test_set_next_fee_multiplier() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    node.wait_for_block_with_timeout(1, Duration::from_millis(500)).await.unwrap();
    let block1_hash = node.block_hash_by_number(1).await.unwrap();
    let block1 = node.get_block_by_hash(block1_hash).await;
    assert_eq!(block1.base_fee_per_gas, polkadot_sdk::sp_core::U256::from(1_000_000));

    unwrap_response::<()>(
        node.eth_rpc(EthRequest::SetNextBlockBaseFeePerGas(U256::from(60000000000000000u128)))
            .await
            .unwrap(),
    )
    .unwrap();

    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    node.wait_for_block_with_timeout(2, Duration::from_millis(500)).await.unwrap();
    let block2_hash = node.block_hash_by_number(2).await.unwrap();
    let block2 = node.get_block_by_hash(block2_hash).await;
    assert_eq!(block2.base_fee_per_gas, polkadot_sdk::sp_core::U256::from(1_000_000));

    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    node.wait_for_block_with_timeout(3, Duration::from_millis(500)).await.unwrap();
    let block3_hash = node.block_hash_by_number(3).await.unwrap();
    let block3 = node.get_block_by_hash(block3_hash).await;
    assert_eq!(block3.base_fee_per_gas, polkadot_sdk::sp_core::U256::from(60_000));
}
