use crate::utils::{unwrap_response, TestNode};
use alloy_eips::BlockId;
use alloy_primitives::{Address, U256};
use alloy_rpc_types::TransactionRequest;
use alloy_serde::WithOtherFields;
use anvil_core::eth::EthRequest;
use anvil_polkadot::{
    api_server::revive_conversions::ReviveAddress,
    config::{AnvilNodeConfig, SubstrateNodeConfig},
};
use polkadot_sdk::pallet_revive::evm::{Account, Block};
use subxt::utils::H256;

#[tokio::test(flavor = "multi_thread")]
async fn test_get_chain_id() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    assert_eq!(
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthChainId(())).await.unwrap()).unwrap(),
        "0x190f1b44"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_start_balance() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    assert_eq!(
        unwrap_response::<U256>(
            node.eth_rpc(EthRequest::EthGetBalance(
                Address::from(ReviveAddress::new(
                    Account::from(subxt_signer::eth::dev::alith()).address(),
                )),
                None,
            ))
            .await
            .unwrap()
        )
        .unwrap(),
        U256::from_str_radix("100000000000000000000000", 10).unwrap()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_block_by_hash() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    let hash1 = node.block_hash_by_number(1).await.unwrap();
    let hash0 = node.block_hash_by_number(0).await.unwrap();
    let bl1 = unwrap_response::<Block>(
        node.eth_rpc(EthRequest::EthGetBlockByHash(hash1.as_fixed_bytes().into(), false))
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(hash0, bl1.parent_hash);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_send_transaction() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::SetAutomine(true)).await.unwrap()).unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let baltathar = Account::from(subxt_signer::eth::dev::baltathar());
    let alith_initial_balance = unwrap_response::<U256>(
        node.eth_rpc(EthRequest::EthGetBalance(
            Address::from(ReviveAddress::new(alith.address())),
            None,
        ))
        .await
        .unwrap(),
    )
    .unwrap();

    let baltathar_initial_balance = unwrap_response::<U256>(
        node.eth_rpc(EthRequest::EthGetBalance(
            Address::from(ReviveAddress::new(baltathar.address())),
            None,
        ))
        .await
        .unwrap(),
    )
    .unwrap();

    let transfer_amount = U256::from_str_radix("100000000000000000", 10).unwrap();
    let transaction = TransactionRequest::default()
        .value(transfer_amount)
        .from(Address::from(ReviveAddress::new(alith.address())))
        .to(Address::from(ReviveAddress::new(baltathar.address())));
    let _tx_hash = unwrap_response::<H256>(
        node.eth_rpc(EthRequest::EthSendTransaction(Box::new(WithOtherFields::new(
            transaction.clone(),
        ))))
        .await
        .unwrap(),
    )
    .unwrap();
    node.wait_for_block_with_timeout(1, std::time::Duration::from_secs(2)).await.unwrap();

    let alith_final_balance = unwrap_response::<U256>(
        node.eth_rpc(EthRequest::EthGetBalance(
            Address::from(ReviveAddress::new(alith.address())),
            Some(BlockId::Number(alloy_eips::BlockNumberOrTag::Number(1))),
        ))
        .await
        .unwrap(),
    )
    .unwrap();

    let baltathar_final_balance = unwrap_response::<U256>(
        node.eth_rpc(EthRequest::EthGetBalance(
            Address::from(ReviveAddress::new(baltathar.address())),
            Some(BlockId::Number(alloy_eips::BlockNumberOrTag::Number(1))),
        ))
        .await
        .unwrap(),
    )
    .unwrap();
    assert_ne!(alith_final_balance, alith_initial_balance, "Alith's balance should have changed");
    assert_ne!(
        baltathar_final_balance, baltathar_initial_balance,
        "Baltathar's balance should have changed"
    );
}
