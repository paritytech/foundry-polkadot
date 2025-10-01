use crate::utils::{TestNode, unwrap_response};
use alloy_eips::BlockId;
use alloy_primitives::{Address, U256};
use alloy_rpc_types::TransactionRequest;
use anvil_core::eth::EthRequest;
use anvil_polkadot::{
    api_server::revive_conversions::{AlloyU256, ReviveAddress},
    config::{AnvilNodeConfig, SubstrateNodeConfig},
};
use polkadot_sdk::pallet_revive::evm::Account;
use subxt::utils::H160;

#[tokio::test(flavor = "multi_thread")]
async fn test_impersonate_account() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let latest_block = Some(BlockId::latest());

    let alith_account = Account::from(subxt_signer::eth::dev::alith());
    let alith_addr = Address::from(ReviveAddress::new(alith_account.address()));
    let dest_addr = Address::random();
    let dest_h160 = H160::from_slice(dest_addr.as_slice());
    let transfer_amount = U256::from_str_radix("1600000000000000000", 10).unwrap();

    // Create a random account with some balance.
    unwrap_response::<()>(node.eth_rpc(EthRequest::SetAutomine(true)).await.unwrap()).unwrap();
    let alith_initial_balance = node.get_balance(alith_account.address(), latest_block).await;
    let dest_initial_balance = node.get_balance(dest_h160, latest_block).await;

    let transaction =
        TransactionRequest::default().value(transfer_amount).from(alith_addr).to(dest_addr);
    let tx_hash = node.send_transaction(transaction, Some(1)).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let receipt_info = node.get_transaction_receipt(tx_hash).await;

    // Assert on balances after first transfer.
    let alith_final_balance = node.get_balance(alith_account.address(), latest_block).await;
    let dest_final_balance = node.get_balance(dest_h160, latest_block).await;
    let existential_deposit = U256::from_str_radix("100000000000000000000", 10).unwrap();
    assert_eq!(
        alith_final_balance,
        alith_initial_balance
            - AlloyU256::from(receipt_info.effective_gas_price * receipt_info.gas_used).inner()
            - transfer_amount
            - existential_deposit,
        "alith's balance should have changed"
    );
    assert_eq!(
        dest_final_balance,
        dest_initial_balance + transfer_amount,
        "dest's balance should have changed"
    );

    // Impersonate destination
    unwrap_response::<()>(node.eth_rpc(EthRequest::ImpersonateAccount(dest_addr)).await.unwrap())
        .unwrap();
    let transfer_amount = U256::from_str_radix("100000000000", 10).unwrap();
    let transaction =
        TransactionRequest::default().value(transfer_amount).from(dest_addr).to(alith_addr);
    let tx_hash = node.send_transaction(transaction, Some(2)).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let receipt_info = node.get_transaction_receipt(tx_hash).await;

    // Assert on balances after second transfer.
    let alith_balance = node.get_balance(alith_account.address(), latest_block).await;
    let dest_balance = node.get_balance(dest_h160, latest_block).await;
    assert_eq!(alith_final_balance, alith_balance - transfer_amount);
    assert_eq!(
        dest_final_balance
            - transfer_amount
            - AlloyU256::from(receipt_info.effective_gas_price * receipt_info.gas_used).inner(),
        dest_balance
    );
    let dest_final_balance = dest_balance;
    let alith_final_balance = alith_balance;

    // Stop impersonating destination, and assert on error when retrying the same transfer.
    unwrap_response::<()>(
        node.eth_rpc(EthRequest::StopImpersonatingAccount(dest_addr)).await.unwrap(),
    )
    .unwrap();
    let transfer_amount = U256::from_str_radix("10000000", 10).unwrap();
    let transaction =
        TransactionRequest::default().value(transfer_amount).from(dest_addr).to(alith_addr);
    let err = node.send_transaction(transaction.clone(), Some(3)).await.unwrap_err();
    assert!(err.to_string().starts_with(
        r#"Expected success but got error: RpcError { code: InternalError, message: "Account not found for address"#
    ));

    // Start impersonating any address now
    unwrap_response::<()>(node.eth_rpc(EthRequest::AutoImpersonateAccount(true)).await.unwrap())
        .unwrap();

    // Transfer at block 3 (same as for previous failed transfer, which did not result in a block
    // being produced).
    let tx_hash = node.send_transaction(transaction, Some(3)).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let receipt_info = node.get_transaction_receipt(tx_hash).await;

    // Assert on balances after third transfer.
    let alith_balance = node.get_balance(alith_account.address(), latest_block).await;
    let dest_balance = node.get_balance(dest_h160, latest_block).await;
    assert_eq!(alith_final_balance, alith_balance - transfer_amount);
    assert_eq!(
        dest_final_balance
            - transfer_amount
            - AlloyU256::from(receipt_info.effective_gas_price * receipt_info.gas_used).inner(),
        dest_balance
    );

    // Stop impersonating destination, and assert on error when retrying the same transfer.
    unwrap_response::<()>(node.eth_rpc(EthRequest::AutoImpersonateAccount(false)).await.unwrap())
        .unwrap();
    let transfer_amount = U256::from_str_radix("10000000", 10).unwrap();
    let transaction =
        TransactionRequest::default().value(transfer_amount).from(dest_addr).to(alith_addr);
    let err = node.send_transaction(transaction.clone(), Some(4)).await.unwrap_err();
    assert!(err.to_string().starts_with(
        r#"Expected success but got error: RpcError { code: InternalError, message: "Account not found for address"#
    ));
}
