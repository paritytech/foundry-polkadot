use alloy_primitives::{Address, U256};
use anvil_core::eth::EthRequest;
use anvil_polkadot::{
    api_server::{error::Error, revive_conversions::ReviveAddress},
    config::{AnvilNodeConfig, SubstrateNodeConfig},
};
use polkadot_sdk::pallet_revive::evm::Account;

use crate::utils::{TestNode, unwrap_response};

#[tokio::test(flavor = "multi_thread")]
async fn test_impersonate_account() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let alith_account = Account::from(subxt_signer::eth::dev::alith());
    let alith_addr = Address::from(ReviveAddress::new(alith_account.address()));
    let dest_addr = Address::random();
    let transfer_amount = U256::from_str_radix("1600000000000000000", 10).unwrap();

    // Create a random account with some balance.
    unwrap_response::<()>(node.eth_rpc(EthRequest::SetAutomine(true)).await.unwrap()).unwrap();
    let alith_initial_balance = node.get_eth_balance(alith_addr, None).await;
    let dest_initial_balance = node.get_eth_balance(dest_addr, None).await;
    let _tx_hash = node.eth_transfer(alith_addr, dest_addr, transfer_amount, 1).await;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Assert on balances after first transfer.
    let alith_final_balance = node.get_eth_balance(alith_addr, Some(1)).await;
    let dest_final_balance = node.get_eth_balance(dest_addr, Some(1)).await;
    // gas paid 101231061064271000000 (gas_price * gas + transfer_amount)
    assert_eq!(
        alith_final_balance,
        alith_initial_balance
            - U256::from(1600000000000000000u64)
            - U256::from_str_radix("101231061064271000000", 10).unwrap(),
        "alith's balance should have changed"
    );
    assert_eq!(
        dest_final_balance,
        dest_initial_balance + U256::from(1600000000000000000u64),
        "dest's balance should have changed"
    );

    // Impersonate destination
    unwrap_response::<()>(node.eth_rpc(EthRequest::ImpersonateAccount(dest_addr)).await.unwrap())
        .unwrap();
    let transfer_amount = U256::from_str_radix("100000000000", 10).unwrap();
    let _tx_hash = node.eth_transfer(dest_addr, alith_addr, transfer_amount, 2).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Assert on balances after second transfer.
    let alith_balance = node.get_eth_balance(alith_addr, Some(2)).await;
    let dest_balance = node.get_eth_balance(dest_addr, Some(2)).await;
    assert_eq!(alith_final_balance, alith_balance - transfer_amount);
    // gas here is 760108157000000000
    assert_eq!(
        dest_final_balance - U256::from(760108157000000000u64) - transfer_amount,
        dest_balance
    );

    // Stop impersonating destination, and assert on error when retrying the same transfer.
    unwrap_response::<()>(
        node.eth_rpc(EthRequest::StopImpersonatingAccount(dest_addr)).await.unwrap(),
    )
    .unwrap();
    let err = node.eth_transfer(dest_addr, alith_addr, transfer_amount, 2).await.unwrap_err();
    assert!(err.to_string().starts_with(r#"Expected success but got error: RpcError { code: InvalidParams, message: "Account not found for address"#));

    // Start impersonating any address now
    // FIX: fails with invalid transaction - outdated transaction.
    unwrap_response::<()>(node.eth_rpc(EthRequest::ImpersonateAccount(dest_addr)).await.unwrap())
        .unwrap();
    let _tx_hash = node.eth_transfer(dest_addr, alith_addr, transfer_amount, 3).await.unwrap();

    // Assert on balances after second transfer.
    let alith_balance = node.get_eth_balance(alith_addr, Some(2)).await;
    let dest_balance = node.get_eth_balance(dest_addr, Some(2)).await;
    assert_eq!(alith_final_balance, alith_balance - transfer_amount - transfer_amount);
    // gas here is 760108157000000000
    assert_eq!(
        dest_final_balance
            - U256::from(2 * 760108157000000000u64)
            - transfer_amount
            - transfer_amount,
        dest_balance
    );
}
