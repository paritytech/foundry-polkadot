use alloy_primitives::{Address, U256};
use anvil_core::eth::EthRequest;
use anvil_polkadot::{
    api_server::revive_conversions::ReviveAddress,
    config::{AnvilNodeConfig, SubstrateNodeConfig},
};
use polkadot_sdk::pallet_revive::evm::Account;

use crate::utils::{TestNode, unwrap_response};

#[tokio::test(flavor = "multi_thread")]
async fn test_impersonate_account() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();
    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_addr = Address::from(ReviveAddress::new(alith.address()));
    let charleth_addr = Address::from(ReviveAddress::new(
        Account::from(subxt_signer::eth::dev::charleth()).address(),
    ));
    let transfer_amount = U256::from_str_radix("147946870520689664", 10).unwrap();

    // Create a random account with some balance.
    unwrap_response::<()>(node.eth_rpc(EthRequest::SetAutomine(true)).await.unwrap()).unwrap();
    let alith_initial_balance = node.get_eth_balance(alith_addr, None).await;
    let charleth_initial_balance = node.get_eth_balance(charleth_addr, None).await;
    let _tx_hash = node.eth_transfer(alith_addr, charleth_addr, transfer_amount, 1).await;
    let alith_final_balance = node.get_eth_balance(alith_addr, Some(1)).await;
    let charleth_final_balance = node.get_eth_balance(charleth_addr, Some(1)).await;
    assert_ne!(alith_final_balance, alith_initial_balance, "alith's balance should have changed");
    assert_ne!(
        charleth_final_balance, charleth_initial_balance,
        "charleth's balance should have changed"
    );

    // Impersonate destination
    unwrap_response::<()>(
        node.eth_rpc(EthRequest::ImpersonateAccount(charleth_addr)).await.unwrap(),
    )
    .unwrap();
    // FIX: test fails here due to charleth having balance 0, per revive reported error.
    // called `Result::unwrap()` on an `Err` value: "Expected success but got error: RpcError {
    // code: InternalError, message: \"Revive call failed:
    // TransactError(EthTransactError::Message(\\\"insufficient funds for gas * price + value:
    // address 0x798d4ba9baf0064ec19eb4f0a1a45785ae9d6dfc have 0 (supplied gas
    // 52429550000000000)\\\"))\", data: None }"
    //
    // I suspect this might be a local storage issue, but can't tell how previous calls to
    // EthGetBalance detect a balance change for `charleth` after the first transfer.
    let _tx_hash = node
        .eth_transfer(charleth_addr, alith_addr, U256::from_str_radix("1", 10).unwrap(), 2)
        .await;
    let alith_balance = node.get_eth_balance(alith_addr, Some(2)).await;
    let charleth_balance = node.get_eth_balance(charleth_addr, Some(2)).await;
    assert_ne!(alith_final_balance, alith_balance);
    assert_ne!(charleth_final_balance, charleth_balance);
}
