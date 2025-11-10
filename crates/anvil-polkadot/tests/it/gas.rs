use std::time::Duration;

use crate::utils::{TestNode, unwrap_response};
use alloy_primitives::{Address, U256};
use alloy_rpc_types::TransactionRequest;
use anvil_core::eth::EthRequest;
use anvil_polkadot::config::{AnvilNodeConfig, INITIAL_BASE_FEE, SubstrateNodeConfig};
use polkadot_sdk::pallet_revive::evm::Account;
use rstest::rstest;
use std::ops::Not;

#[tokio::test(flavor = "multi_thread")]
#[rstest]
#[case(false)]
#[case(true)]
async fn test_set_next_fee_multiplier(#[case] rpc_driven: bool) {
    // 1e18 denomination.
    let new_base_fee = U256::from(6_000_000);
    let anvil_node_config = AnvilNodeConfig::test_config()
        .with_base_fee(rpc_driven.not().then_some(new_base_fee.to::<u64>()));
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let gas_price =
        unwrap_response::<U256>(node.eth_rpc(EthRequest::EthGasPrice(())).await.unwrap()).unwrap();

    if rpc_driven {
        assert_eq!(gas_price.to::<u128>(), INITIAL_BASE_FEE);
        unwrap_response::<()>(
            node.eth_rpc(EthRequest::SetNextBlockBaseFeePerGas(new_base_fee)).await.unwrap(),
        )
        .unwrap();
    } else {
        assert_eq!(gas_price, new_base_fee);
    }

    // Currently the gas_price returned from evm is equivalent to the base_fee.
    let gas_price =
        unwrap_response::<U256>(node.eth_rpc(EthRequest::EthGasPrice(())).await.unwrap()).unwrap();
    assert_eq!(gas_price, new_base_fee);

    // We send a regular eth transfer to check the associated effective gas price used by the
    // transaction, after it will be included in a next block. We're interested especially in
    // the tx effective gas price to validate that the base_fee_per_gas set previously is also
    // considered when computing the fees for the tx execution.
    // We could have checked the `base_fee_per_gas` after querying the latest eth block mined
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
    node.wait_for_block_with_timeout(1, Duration::from_millis(400)).await.unwrap();
    tokio::time::sleep(Duration::from_millis(400)).await;
    let transaction_receipt = node.get_transaction_receipt(tx_hash).await;
    let effective_gas_price =
        U256::from_be_bytes(transaction_receipt.effective_gas_price.to_big_endian());
    let gas_used = U256::from_be_bytes(transaction_receipt.gas_used.to_big_endian());
    assert_eq!(effective_gas_price, new_base_fee);
    let alith_final_balance = node.get_balance(alith.address(), None).await;
    let baltathar_final_balance = node.get_balance(baltathar.address(), None).await;
    assert_eq!(
        baltathar_final_balance,
        baltathar_initial_balance + transfer_amount,
        "Baltathar's balance should have changed"
    );
    assert_eq!(
        alith_final_balance,
        alith_initial_balance - transfer_amount - effective_gas_price * gas_used,
        "Alith's balance should have changed"
    );

    let block1_hash = node.block_hash_by_number(1).await.unwrap();
    let block1 = node.get_block_by_hash(block1_hash).await;
    // This will fail ideally once we update to a polkadot-sdk version that includes a fix for
    // https://github.com/paritytech/polkadot-sdk/issues/10177. The reported base_fer_per_gas
    // should be the previously set `new_base_fee`.
    assert_eq!(U256::from_be_bytes(block1.base_fee_per_gas.to_big_endian()), U256::from(5999888));

    // Mining a second block should update the base fee according to the logic that determines
    // the base_fee in relation to how congested the network is.
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    node.wait_for_block_with_timeout(2, Duration::from_millis(500)).await.unwrap();
    let block2_hash = node.block_hash_by_number(2).await.unwrap();
    let block2 = node.get_block_by_hash(block2_hash).await;

    // This will fail ideally once we update to a polkadot-sdk version that includes a fix for
    // https://github.com/paritytech/polkadot-sdk/issues/10177.
    assert_eq!(U256::from_be_bytes(block2.base_fee_per_gas.to_big_endian()), 5999775);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_next_fee_multiplier_minimum() {
    // 1e18 denomination.
    let new_base_fee = U256::from(50_123);
    let anvil_node_config =
        AnvilNodeConfig::test_config().with_base_fee(Some(new_base_fee.to::<u64>()));
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Currently the gas_price returned from evm is equivalent to the base_fee.
    let gas_price =
        unwrap_response::<U256>(node.eth_rpc(EthRequest::EthGasPrice(())).await.unwrap()).unwrap();
    assert_eq!(gas_price, new_base_fee);

    // We send a regular eth transfer to check the associated effective gas price used by the
    // transaction, after it will be included in a next block. We're interested especially in
    // the tx effective gas price to validate that the base_fee_per_gas set previously is also
    // considered when computing the fees for the tx execution.
    // We could have checked the `base_fee_per_gas` after querying the latest eth block mined
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
    node.wait_for_block_with_timeout(1, Duration::from_millis(400)).await.unwrap();
    tokio::time::sleep(Duration::from_millis(400)).await;
    let transaction_receipt = node.get_transaction_receipt(tx_hash).await;
    let effective_gas_price =
        U256::from_be_bytes(transaction_receipt.effective_gas_price.to_big_endian());
    let gas_used = U256::from_be_bytes(transaction_receipt.gas_used.to_big_endian());
    assert_eq!(effective_gas_price, new_base_fee);
    let alith_final_balance = node.get_balance(alith.address(), None).await;
    let baltathar_final_balance = node.get_balance(baltathar.address(), None).await;
    assert_eq!(
        baltathar_final_balance,
        baltathar_initial_balance + transfer_amount,
        "Baltathar's balance should have changed"
    );
    assert_eq!(
        alith_final_balance,
        alith_initial_balance - transfer_amount - effective_gas_price * gas_used,
        "Alith's balance should have changed"
    );

    let block1_hash = node.block_hash_by_number(1).await.unwrap();
    let block1 = node.get_block_by_hash(block1_hash).await;

    // The anvil-polkadot substrate-runtime is configured similarly to the assethub runtimes in
    // terms of the minimum NextFeeMultiplier value that can be reached. The minimum is the one
    // configured in the runtime, which in our case is the same as for asset-hub-westend. This
    // assert should fail once https://github.com/paritytech/polkadot-sdk/issues/10177 is fixed.
    // The actual value should be the previously set base_fee.
    assert_eq!(U256::from_be_bytes(block1.base_fee_per_gas.to_big_endian()), U256::from(100_000));

    // Mining a second block should update the base fee according to the logic that determines
    // the base_fee in relation to how congested the network is.
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    node.wait_for_block_with_timeout(2, Duration::from_millis(500)).await.unwrap();
    let block2_hash = node.block_hash_by_number(2).await.unwrap();
    let block2 = node.get_block_by_hash(block2_hash).await;

    // However, since the previously set base_fee is lower than the minimum, this should be set
    // right away to the minimum.
    assert_eq!(U256::from_be_bytes(block2.base_fee_per_gas.to_big_endian()), U256::from(100_000));
}
