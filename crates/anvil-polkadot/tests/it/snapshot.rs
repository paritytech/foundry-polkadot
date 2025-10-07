use crate::utils::{TestNode, unwrap_response};
use alloy_primitives::{Address, U256};
use alloy_rpc_types::TransactionRequest;
use anvil::cmd::NodeArgs;
use anvil_core::eth::EthRequest;
use anvil_polkadot::{
    api_server::revive_conversions::{AlloyU256, ReviveAddress},
    config::{AnvilNodeConfig, SubstrateNodeConfig},
};
use polkadot_sdk::pallet_revive::{
    self,
    evm::{Account, HashesOrTransactionInfos},
};
use subxt::utils::H160;

#[tokio::test(flavor = "multi_thread")]
async fn test_evm_revert() {
    let anvil_node_config = AnvilNodeConfig::test_config().with_no_mining(true);
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    assert_eq!(node.best_block_number().await, 0);

    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(1)), None)).await.unwrap())
        .unwrap();
    assert_eq!(node.best_block_number().await, 1);

    let id = unwrap_response::<String>(node.eth_rpc(EthRequest::EvmSnapshot(())).await.unwrap())
        .unwrap();
    assert_eq!(id, "0x1".to_string());

    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(1)), None)).await.unwrap())
        .unwrap();
    assert_eq!(node.best_block_number().await, 2);

    let success = unwrap_response::<bool>(
        node.eth_rpc(EthRequest::EvmRevert(
            U256::from_str_radix(id.trim_start_matches("0x1"), 16).unwrap(),
        ))
        .await
        .unwrap(),
    )
    .unwrap();
    assert!(success);
    assert_eq!(node.best_block_number().await, 1);

    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(5)), None)).await.unwrap())
        .unwrap();
    assert_eq!(node.best_block_number().await, 10);
}
