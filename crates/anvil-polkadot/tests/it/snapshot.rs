use std::time::Duration;

use crate::utils::{EXISTENTIAL_DEPOSIT, TestNode, assert_with_tolerance, unwrap_response};
use alloy_primitives::{Address, U256};
use alloy_rpc_types::TransactionRequest;
use anvil_core::eth::EthRequest;
use anvil_polkadot::{
    api_server::revive_conversions::{AlloyU256, ReviveAddress},
    config::{AnvilNodeConfig, SubstrateNodeConfig},
};
use polkadot_sdk::pallet_revive::{self, evm::Account};
use subxt::utils::H160;

#[tokio::test(flavor = "multi_thread")]
// TODO: expect on block number as returned by block provider.
async fn test_best_after_evm_revert() {
    let anvil_node_config = AnvilNodeConfig::test_config().with_no_mining(true);
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Assert on initial best block number.
    assert_eq!(node.best_block_number().await, 0);

    // Mine 5 blocks and assert on the new best block.
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(5)), None)).await.unwrap())
        .unwrap();
    assert_eq!(node.best_block_number().await, 5);

    // Snapshot at block number 5.
    let id = unwrap_response::<String>(node.eth_rpc(EthRequest::EvmSnapshot(())).await.unwrap())
        .unwrap();
    assert_eq!(id, "0x1".to_string());

    // Mine 5 more blocks.
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(5)), None)).await.unwrap())
        .unwrap();
    assert_eq!(node.best_block_number().await, 10);

    // Snapshot again at block number 10.
    let id = unwrap_response::<String>(node.eth_rpc(EthRequest::EvmSnapshot(())).await.unwrap())
        .unwrap();
    assert_eq!(id, "0x2".to_string());
    assert_eq!(node.best_block_number().await, 10);

    // Mine 5 more blocks.
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(5)), None)).await.unwrap())
        .unwrap();
    assert_eq!(node.best_block_number().await, 15);

    // Revert to the second snapshot and assert best block number is 10.
    let snapshot_id = U256::from_str_radix(id.trim_start_matches("0x"), 16).unwrap();
    assert_eq!(snapshot_id, U256::from(2));
    let reverted =
        unwrap_response::<u64>(node.eth_rpc(EthRequest::EvmRevert(snapshot_id)).await.unwrap())
            .unwrap();
    assert_eq!(reverted, 5);
    assert_eq!(node.best_block_number().await, 10);

    // Check mining works fine after reverting.
    unwrap_response::<()>(
        node.eth_rpc(EthRequest::Mine(Some(U256::from(10)), None)).await.unwrap(),
    )
    .unwrap();
    assert_eq!(node.best_block_number().await, 20);

    // Test the case of revert -> mine -> snapshot -> revert (the block number remains the same
    // when snapshot was done).
    let id = unwrap_response::<String>(node.eth_rpc(EthRequest::EvmSnapshot(())).await.unwrap())
        .unwrap();
    assert_eq!(id, "0x3".to_string());
    assert_eq!(node.best_block_number().await, 20);

    let snapshot_id = U256::from_str_radix(id.trim_start_matches("0x"), 16).unwrap();
    assert_eq!(snapshot_id, U256::from(3));
    let reverted =
        unwrap_response::<u64>(node.eth_rpc(EthRequest::EvmRevert(snapshot_id)).await.unwrap())
            .unwrap();
    assert_eq!(reverted, 0);
    assert_eq!(node.best_block_number().await, 20);

    // Test the case of revert id -> revert same id.
    let reverted =
        unwrap_response::<u64>(node.eth_rpc(EthRequest::EvmRevert(U256::ONE)).await.unwrap())
            .unwrap();
    assert_eq!(reverted, 15);
    assert_eq!(node.best_block_number().await, 5);

    let reverted =
        unwrap_response::<u64>(node.eth_rpc(EthRequest::EvmRevert(U256::ONE)).await.unwrap())
            .unwrap();
    assert_eq!(reverted, 0);

    // Test reverting down to genesis.
    // The snapshot at genesis block is automatically created
    // at node startup.
    let reverted =
        unwrap_response::<u64>(node.eth_rpc(EthRequest::EvmRevert(U256::ZERO)).await.unwrap())
            .unwrap();
    assert_eq!(reverted, 5);
    assert_eq!(node.best_block_number().await, 0);
}

#[tokio::test(flavor = "multi_thread")]
// TODO: expect on block number as returned by block provider.
async fn test_balances_and_txs_index_after_evm_revert() {
    let anvil_node_config = AnvilNodeConfig::test_config().with_no_mining(true);
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Assert on initial best block number.
    assert_eq!(node.best_block_number().await, 0);

    // Mine 5 blocks and assert on the new best block.
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(5)), None)).await.unwrap())
        .unwrap();
    assert_eq!(node.best_block_number().await, 5);

    // Snapshot at block number 5.
    let id = unwrap_response::<String>(node.eth_rpc(EthRequest::EvmSnapshot(())).await.unwrap())
        .unwrap();
    assert_eq!(id, "0x1".to_string());

    // Get known accounts initial balances.
    let alith_account = Account::from(subxt_signer::eth::dev::alith());
    let alith_addr = Address::from(ReviveAddress::new(alith_account.address()));
    let baltathar_account = Account::from(subxt_signer::eth::dev::baltathar());
    let baltathar_addr = Address::from(ReviveAddress::new(baltathar_account.address()));
    let alith_initial_balance = node.get_balance(alith_account.address(), None).await;
    let baltathar_initial_balance = node.get_balance(baltathar_account.address(), None).await;

    // Initialize a random account. Assume its initial balance is 0.
    let transfer_amount = U256::from(16e17);
    let (dest_addr, tx_hash) =
        node.eth_transfer_to_unitialized_random_account(alith_addr, transfer_amount, None).await;
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(1)), None)).await.unwrap())
        .unwrap();
    assert_eq!(node.best_block_number().await, 6);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let receipt_info = node.get_transaction_receipt(tx_hash).await;

    // Assert on balances after first transfer.
    let alith_balance_after_tx0 = node.get_balance(alith_account.address(), None).await;
    let dest_balance = node.get_balance(H160::from_slice(dest_addr.as_slice()), None).await;
    assert_eq!(
        alith_balance_after_tx0,
        alith_initial_balance
            - AlloyU256::from(receipt_info.effective_gas_price * receipt_info.gas_used).inner()
            - transfer_amount
            - U256::from(EXISTENTIAL_DEPOSIT),
        "alith's balance should have changed"
    );
    assert_eq!(dest_balance, transfer_amount, "dest's balance should have changed");

    let transaction_receipt = node.get_transaction_receipt(tx_hash).await;
    assert_eq!(transaction_receipt.block_number, pallet_revive::U256::from(6));
    assert_eq!(transaction_receipt.transaction_index, pallet_revive::U256::one());
    assert_eq!(transaction_receipt.transaction_hash, tx_hash);

    // Make another regular transfer between known accounts.
    let transfer_amount = U256::from(1e17);
    let transaction =
        TransactionRequest::default().value(transfer_amount).from(baltathar_addr).to(alith_addr);
    let tx_hash = node.send_transaction(transaction, None).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(1)), None)).await.unwrap())
        .unwrap();
    assert_eq!(node.best_block_number().await, 7);

    tokio::time::sleep(Duration::from_millis(500)).await;
    let transaction_receipt = node.get_transaction_receipt(tx_hash).await;

    assert_eq!(transaction_receipt.block_number, pallet_revive::U256::from(7));
    assert_eq!(transaction_receipt.transaction_index, pallet_revive::U256::one());
    assert_eq!(transaction_receipt.transaction_hash, tx_hash);

    let alith_final_balance = node.get_balance(alith_account.address(), None).await;
    let baltathar_final_balance = node.get_balance(baltathar_account.address(), None).await;
    assert_eq!(
        baltathar_final_balance,
        baltathar_initial_balance
            - transfer_amount
            - AlloyU256::from(
                transaction_receipt.effective_gas_price * transaction_receipt.gas_used
            )
            .inner(),
        "Baltathar's balance should have changed"
    );
    assert_eq!(
        alith_final_balance,
        alith_balance_after_tx0 + transfer_amount,
        "Alith's balance should have changed"
    );

    // Revert to a block before the transactions have been mined.
    let snapshot_id = U256::from_str_radix(id.trim_start_matches("0x"), 16).unwrap();
    assert_eq!(snapshot_id, U256::from(1));
    let reverted =
        unwrap_response::<u64>(node.eth_rpc(EthRequest::EvmRevert(snapshot_id)).await.unwrap())
            .unwrap();
    assert_eq!(reverted, 2);
    assert_eq!(node.best_block_number().await, 5);

    // Assert on accounts balances to be the initial balances.
    let alith_balance = node.get_balance(alith_account.address(), None).await;
    let baltathar_balance = node.get_balance(baltathar_account.address(), None).await;
    let dest_balance = node.get_balance(H160::from_slice(dest_addr.as_slice()), None).await;
    assert_eq!(alith_balance, alith_initial_balance);
    assert_eq!(baltathar_balance, baltathar_initial_balance);
    assert_eq!(dest_balance, U256::ZERO);
    assert_eq!(
        node.nonce(5, alith_account.address()).await.unwrap(),
        pallet_revive::evm::U256::zero()
    );
    assert_eq!(
        node.nonce(5, baltathar_account.address()).await.unwrap(),
        pallet_revive::evm::U256::zero()
    );
    assert_eq!(
        node.nonce(5, H160::from_slice(dest_addr.as_slice())).await.unwrap(),
        pallet_revive::evm::U256::zero()
    );

    // Remine the 6th block with same txs above.
    let transaction =
        TransactionRequest::default().value(U256::from(16e17)).from(alith_addr).to(dest_addr);
    let tx_hash1 = node.send_transaction(transaction, None).await.unwrap();
    let transaction =
        TransactionRequest::default().value(U256::from(1e17)).from(baltathar_addr).to(alith_addr);
    let tx_hash2 = node.send_transaction(transaction, None).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(1)), None)).await.unwrap())
        .unwrap();
    assert_eq!(node.best_block_number().await, 6);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let receipt_info = node.get_transaction_receipt(tx_hash1).await;
    assert_eq!(receipt_info.block_number, pallet_revive::U256::from(6));
    assert_eq!(receipt_info.transaction_index, pallet_revive::U256::one());
    assert_eq!(receipt_info.transaction_hash, tx_hash1);
    let receipt_info = node.get_transaction_receipt(tx_hash2).await;
    assert_eq!(receipt_info.block_number, pallet_revive::U256::from(6));
    assert_eq!(receipt_info.transaction_index, pallet_revive::U256::from(2));
    assert_eq!(receipt_info.transaction_hash, tx_hash2);
    assert_eq!(
        node.nonce(6, alith_account.address()).await.unwrap(),
        pallet_revive::evm::U256::one()
    );
    assert_eq!(
        node.nonce(6, baltathar_account.address()).await.unwrap(),
        pallet_revive::evm::U256::one()
    );
    assert_eq!(
        node.nonce(6, H160::from_slice(dest_addr.as_slice())).await.unwrap(),
        pallet_revive::evm::U256::zero()
    );
}

#[tokio::test(flavor = "multi_thread")]
// TODO: add a test where we call a contract that queries the timestamp
// at a certain block before and after a revert, while mining blocks
// TODO: expect on block number as returned by block provider.
async fn test_evm_revert_and_timestamp() {
    let anvil_node_config = AnvilNodeConfig::test_config().with_no_mining(true);
    // Generate the current timestamp and pass it to anvil config.
    let genesis_timestamp = anvil_node_config.get_genesis_timestamp();
    let anvil_node_config = anvil_node_config.with_genesis_timestamp(Some(genesis_timestamp));
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Assert on first best block number.
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(1)), None)).await.unwrap())
        .unwrap();
    assert_eq!(node.best_block_number().await, 1);
    assert_with_tolerance(
        node.get_decoded_timestamp(None)
            .await
            .saturating_div(1000)
            .saturating_sub(genesis_timestamp),
        2,
        1,
        "wrong timestamp at first block",
    );

    let first_timestamp = node.get_decoded_timestamp(None).await;
    let second_timestamp = first_timestamp.saturating_div(1000).saturating_add(3600);
    assert_with_tolerance(
        unwrap_response::<u64>(
            node.eth_rpc(EthRequest::EvmSetTime(U256::from(second_timestamp))).await.unwrap(),
        )
        .unwrap(),
        3600,
        1,
        "Wrong offset 1",
    );

    // Mine 1 blocks and assert on the new best block.
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(1)), None)).await.unwrap())
        .unwrap();
    assert_eq!(node.best_block_number().await, 2);
    let second_timestamp = node.get_decoded_timestamp(None).await;
    assert_with_tolerance(
        second_timestamp.saturating_sub(first_timestamp).saturating_div(1000),
        3600,
        1,
        "wrong timestamp at second block",
    );

    // Snapshot at block number 2 and then mine 1 more block.
    let id = unwrap_response::<String>(node.eth_rpc(EthRequest::EvmSnapshot(())).await.unwrap())
        .unwrap();
    assert_eq!(id, "0x1".to_string());

    // Seconds
    let third_timestamp = second_timestamp.saturating_div(1000).saturating_add(3600);
    assert_with_tolerance(
        unwrap_response::<u64>(
            node.eth_rpc(EthRequest::EvmSetTime(U256::from(third_timestamp))).await.unwrap(),
        )
        .unwrap(),
        3600,
        1,
        "Wrong offset 2",
    );

    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(1)), None)).await.unwrap())
        .unwrap();
    assert_eq!(node.best_block_number().await, 3);
    let third_timestamp = node.get_decoded_timestamp(None).await;
    assert_with_tolerance(
        third_timestamp.saturating_sub(second_timestamp).saturating_div(1000),
        3600,
        1,
        "wrong timestamp at third block",
    );

    // Revert to block number 2.
    let snapshot_id = U256::from_str_radix(id.trim_start_matches("0x"), 16).unwrap();
    assert_eq!(snapshot_id, U256::from(1));
    let reverted =
        unwrap_response::<u64>(node.eth_rpc(EthRequest::EvmRevert(snapshot_id)).await.unwrap())
            .unwrap();
    assert_eq!(reverted, 1);
    assert_eq!(node.best_block_number().await, 2);
    assert_with_tolerance(
        node.get_decoded_timestamp(None).await.saturating_sub(second_timestamp),
        0,
        1,
        "wrong timestamp at reverted second block",
    );

    // Mine again 1 block and check again the timestamp. We should have the same timestamp as for
    // block number 2, since we haven't set a new time for the time manager.
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(1)), None)).await.unwrap())
        .unwrap();
    assert_eq!(node.best_block_number().await, 3);
    let last_timestamp = node.get_decoded_timestamp(None).await;
    assert_with_tolerance(
        last_timestamp.saturating_sub(second_timestamp),
        0,
        1,
        "wrong timestamp at remined third block",
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_rollback() {
    let anvil_node_config = AnvilNodeConfig::test_config().with_no_mining(true);
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Assert on initial best block number.
    assert_eq!(node.best_block_number().await, 0);

    // Mine 5 blocks and assert on the new best block.
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(5)), None)).await.unwrap())
        .unwrap();
    assert_eq!(node.best_block_number().await, 5);

    // Rollback 2 blocks.
    let reverted =
        unwrap_response::<u64>(node.eth_rpc(EthRequest::Rollback(Some(2))).await.unwrap()).unwrap();
    assert_eq!(reverted, 2);
    assert_eq!(node.best_block_number().await, 3);

    // Check mining works fine after reverting.
    unwrap_response::<()>(
        node.eth_rpc(EthRequest::Mine(Some(U256::from(10)), None)).await.unwrap(),
    )
    .unwrap();
    assert_eq!(node.best_block_number().await, 13);

    // Rollback 1 blocks.
    let reverted =
        unwrap_response::<u64>(node.eth_rpc(EthRequest::Rollback(None)).await.unwrap()).unwrap();
    assert_eq!(reverted, 1);
    assert_eq!(node.best_block_number().await, 12);
}
