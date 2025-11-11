use anvil_core::eth::EthRequest;

use crate::utils::unwrap_response;

#[tokio::test(flavor = "multi_thread")]
async fn test_block_filters_receive_new_block() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let filter_id =
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthNewBlockFilter(())).await.unwrap())
            .unwrap();
}
