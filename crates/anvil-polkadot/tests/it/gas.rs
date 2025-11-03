use std::time::Duration;

use crate::utils::{NATIVE_TO_ETH_RATIO, TestNode, unwrap_response};
use alloy_primitives::{Address, U256};
use alloy_rpc_types::TransactionRequest;
use anvil_core::eth::EthRequest;
use anvil_polkadot::config::{AnvilNodeConfig, SubstrateNodeConfig};
use polkadot_sdk::pallet_revive::evm::Account;

#[tokio::test(flavor = "multi_thread")]
async fn test_set_next_fee_multiplier() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    node.wait_for_block_with_timeout(1, Duration::from_millis(500)).await.unwrap();
    let block1_hash = node.block_hash_by_number(1).await.unwrap();
    let block1 = node.get_block_by_hash(block1_hash).await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert_eq!(block1.base_fee_per_gas, polkadot_sdk::sp_core::U256::from(999981));

    // Setting base fee to something lower than 1 mwei will result in a 0 base_fee
    // because 1_000_000 units denominated in 1e18 (ETH precision) represents 1 unit
    // in 1e12 (DOT precision). This is currently the minimum units in ETH that can
    // be represented in DOT balances. Going lower will not be tracked, having 0 balance.
    let new_base_fee = U256::from(600_000);
    let native_to_eth_ratio = U256::from(NATIVE_TO_ETH_RATIO);
    unwrap_response::<()>(
        node.eth_rpc(EthRequest::SetNextBlockBaseFeePerGas(new_base_fee * native_to_eth_ratio))
            .await
            .unwrap(),
    )
    .unwrap();
    let gas_price =
        unwrap_response::<U256>(node.eth_rpc(EthRequest::EthGasPrice(())).await.unwrap()).unwrap();
    assert_eq!(gas_price, U256::ZERO);

    // Now setting the base fee to something that can be visible as a DOT balance.
    let new_base_fee = U256::from(6_000_000_000_000u128);
    unwrap_response::<()>(
        node.eth_rpc(EthRequest::SetNextBlockBaseFeePerGas(new_base_fee * native_to_eth_ratio))
            .await
            .unwrap(),
    )
    .unwrap();

    // Currently the gas_price returned from evm is equivalent to the base_fee.
    let gas_price =
        unwrap_response::<U256>(node.eth_rpc(EthRequest::EthGasPrice(())).await.unwrap()).unwrap();
    assert_eq!(gas_price, new_base_fee / native_to_eth_ratio);

    // We send a regular eth transfer to check the associated effective gas price used by the
    // transaction, after it will be included in a next block. We're interested especially in
    // the tx effective gas price to validate that the base_fee_per_gas set previously is also
    // considered when computing the fees for the tx execution.
    // We could have checked the `base_fee_per_gas` after querying on the latest eth block mined
    // (which could have been empty too) after setting a new base fee, but it will not report the
    // correct base fee because of: https://github.com/paritytech/polkadot-sdk/issues/10177.
    let alith = Account::from(subxt_signer::eth::dev::alith());
    let baltathar = Account::from(subxt_signer::eth::dev::baltathar());
    let alith_initial_balance = node.get_balance(alith.address(), None).await;
    let baltathar_initial_balance = node.get_balance(baltathar.address(), None).await;
    let transfer_amount = U256::from_str_radix("100000000000000000", 10).unwrap();
    let transaction = TransactionRequest::default()
        .value(transfer_amount)
        .from(Address::from(alith.address().to_fixed_bytes()))
        .to(Address::from(baltathar.address().to_fixed_bytes()));
    let tx_hash = node.send_transaction(transaction, None).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    node.wait_for_block_with_timeout(2, Duration::from_millis(400)).await.unwrap();
    tokio::time::sleep(Duration::from_millis(400)).await;
    let transaction_receipt = node.get_transaction_receipt(tx_hash).await;
    // let effective_gas_price =
    //     U256::from_be_bytes(transaction_receipt.effective_gas_price.to_big_endian());
    let gas_used = U256::from_be_bytes(transaction_receipt.gas_used.to_big_endian());
    // assert_eq!(effective_gas_price, new_base_fee / native_to_eth_ratio);
    let alith_final_balance = node.get_balance(alith.address(), None).await;
    let baltathar_final_balance = node.get_balance(baltathar.address(), None).await;
    assert_eq!(
        baltathar_final_balance,
        baltathar_initial_balance + transfer_amount,
        "Baltathar's balance should have changed"
    );
    assert_eq!(
        alith_final_balance,
        alith_initial_balance - transfer_amount - (new_base_fee / native_to_eth_ratio) * gas_used,
        "Alith's balance should have changed"
    );

    let block2_hash = node.block_hash_by_number(2).await.unwrap();
    let block2 = node.get_block_by_hash(block2_hash).await;
    // This will fail ideally once we update to a polkadot-sdk version that includes a fix for
    // https://github.com/paritytech/polkadot-sdk/issues/10177.
    assert_eq!(U256::from_be_bytes(block2.base_fee_per_gas.to_big_endian()), U256::from(5999888));

    // Mining a third block should update the base fee according to the logic that determines
    // the base_fee in relation to how congested the network is.
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    node.wait_for_block_with_timeout(3, Duration::from_millis(500)).await.unwrap();
    let block3_hash = node.block_hash_by_number(3).await.unwrap();
    let block3 = node.get_block_by_hash(block3_hash).await;

    // This will fail ideally once we update to a polkadot-sdk version that includes a fix for
    // https://github.com/paritytech/polkadot-sdk/issues/10177.
    assert_eq!(U256::from_be_bytes(block3.base_fee_per_gas.to_big_endian()), 5999775);
}
