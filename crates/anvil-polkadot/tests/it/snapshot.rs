use std::time::Duration;

use crate::{
    abi::Multicall,
    utils::{
        BlockWaitTimeout, TestNode, assert_with_tolerance, get_contract_code, unwrap_response,
    },
};
use alloy_primitives::{Address, Bytes, U256};
use alloy_rpc_types::{TransactionInput, TransactionRequest};
use alloy_serde::WithOtherFields;
use alloy_sol_types::SolCall;
use anvil_core::eth::EthRequest;
use anvil_polkadot::{
    api_server::revive_conversions::{AlloyU256, ReviveAddress},
    config::{AnvilNodeConfig, SubstrateNodeConfig},
};
use polkadot_sdk::pallet_revive::{
    self,
    evm::{Account, Block, ReceiptInfo},
};
use std::collections::HashSet;
use subxt::utils::H256;

async fn assert_block_number_is_best_and_finalized(
    node: &mut TestNode,
    n: u64,
    wait_for_block_provider: Option<Duration>,
) {
    assert_eq!(std::convert::Into::<u64>::into(node.best_block_number().await), n);
    if let Some(duration) = wait_for_block_provider {
        tokio::time::sleep(duration).await;
        let best_block = unwrap_response::<Block>(
            node.eth_rpc(EthRequest::EthGetBlockByNumber(
                alloy_eips::BlockNumberOrTag::Latest,
                false,
            ))
            .await
            .unwrap(),
        )
        .unwrap();
        let n_as_u256 = pallet_revive::U256::from(n);
        assert_eq!(best_block.number, n_as_u256);

        let finalized_block = unwrap_response::<Block>(
            node.eth_rpc(EthRequest::EthGetBlockByNumber(
                alloy_eips::BlockNumberOrTag::Finalized,
                false,
            ))
            .await
            .unwrap(),
        )
        .unwrap();
        assert_eq!(finalized_block.number, n_as_u256);
    }
}

async fn snapshot(node: &mut TestNode, expected_snapshot_id: U256) -> U256 {
    let id = U256::from_str_radix(
        unwrap_response::<String>(node.eth_rpc(EthRequest::EvmSnapshot(())).await.unwrap())
            .unwrap()
            .trim_start_matches("0x"),
        16,
    )
    .unwrap();

    assert_eq!(id, expected_snapshot_id);
    id
}

async fn mine_blocks(
    node: &mut TestNode,
    blocks: u64,
    assert_best_block: u64,
    wait_for_block_provider: Option<Duration>,
) {
    unwrap_response::<()>(
        node.eth_rpc(EthRequest::Mine(Some(U256::from(blocks)), None)).await.unwrap(),
    )
    .unwrap();
    assert_block_number_is_best_and_finalized(node, assert_best_block, wait_for_block_provider)
        .await;
}

async fn revert(
    node: &mut TestNode,
    snapshot_id: U256,
    assert_best_block: u64,
    assert_success: bool,
    wait_for_block_provider: Option<Duration>,
) {
    let reverted =
        unwrap_response::<bool>(node.eth_rpc(EthRequest::EvmRevert(snapshot_id)).await.unwrap())
            .unwrap();
    assert_eq!(reverted, assert_success);
    assert_block_number_is_best_and_finalized(node, assert_best_block, wait_for_block_provider)
        .await;
}

async fn do_transfer(
    node: &mut TestNode,
    from: Address,
    to: Option<Address>,
    amount: U256,
    block_wait_timeout: Option<BlockWaitTimeout>,
) -> (H256, Option<ReceiptInfo>) {
    let tx_hash = if let Some(to) = to {
        let transaction = TransactionRequest::default().value(amount).from(from).to(to);
        node.send_transaction(transaction, None).await.unwrap()
    } else {
        let (_, tx_hash) =
            node.eth_transfer_to_unitialized_random_account(from, amount, None).await;
        tx_hash
    };

    if let Some(BlockWaitTimeout { block_number, timeout }) = block_wait_timeout {
        mine_blocks(node, 1, block_number.into(), Some(timeout)).await;
        return (tx_hash, Some(node.get_transaction_receipt(tx_hash).await));
    }

    (tx_hash, None)
}

async fn multicall_get_timestamp(
    node: &mut TestNode,
    from: Address,
    contract_address: Address,
) -> U256 {
    let get_timestamp = Multicall::getCurrentBlockTimestampCall::new(()).abi_encode();
    let call_tx = TransactionRequest::default()
        .from(from)
        .to(contract_address)
        .input(TransactionInput::both(get_timestamp.into()));
    Multicall::getCurrentBlockTimestampCall::abi_decode_returns(
        &unwrap_response::<Bytes>(
            node.eth_rpc(EthRequest::EthCall(WithOtherFields::new(call_tx), None, None, None))
                .await
                .unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}

fn alith() -> (Address, Account) {
    let alith_account = Account::from(subxt_signer::eth::dev::alith());
    let alith_addr = Address::from(ReviveAddress::new(alith_account.address()));
    (alith_addr, alith_account)
}

fn baltathar() -> (Address, Account) {
    let baltathar_account = Account::from(subxt_signer::eth::dev::baltathar());
    let baltathar_addr = Address::from(ReviveAddress::new(baltathar_account.address()));
    (baltathar_addr, baltathar_account)
}

#[tokio::test(flavor = "multi_thread")]
async fn test_best_block_after_evm_revert() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Assert on initial best block number.
    assert_block_number_is_best_and_finalized(&mut node, 0, None).await;

    // Snapshot at genesis.
    let zero = snapshot(&mut node, U256::ZERO).await;

    // Mine 5 blocks and assert on the new best block.
    mine_blocks(&mut node, 5, 5, Some(Duration::from_millis(500))).await;

    // Snapshot at block number 5.
    let one = snapshot(&mut node, U256::ONE).await;

    // Mine 5 more blocks.
    mine_blocks(&mut node, 5, 10, Some(Duration::from_millis(500))).await;

    // Snapshot again at block number 10.
    let two = snapshot(&mut node, U256::from(2)).await;
    assert_block_number_is_best_and_finalized(&mut node, 10, None).await;

    // Mine 5 more blocks.
    mine_blocks(&mut node, 5, 15, Some(Duration::from_millis(500))).await;

    // Revert to the second snapshot and assert best block number is 10.
    revert(&mut node, two, 10, true, None).await;

    // Check mining works fine after reverting.
    mine_blocks(&mut node, 10, 20, Some(Duration::from_millis(500))).await;

    // Revert immediatelly after a snapshot (same best number is expected after the revert).
    let id = snapshot(&mut node, U256::from(3)).await;
    revert(&mut node, id, 20, true, Some(Duration::from_millis(500))).await;

    // Test the case of revert id -> revert same id.
    revert(&mut node, one, 5, true, Some(Duration::from_millis(500))).await;
    revert(&mut node, one, 5, false, Some(Duration::from_millis(500))).await;

    // Test reverting down to genesis.
    revert(&mut node, zero, 0, true, Some(Duration::from_millis(500))).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_balances_and_txs_index_after_evm_revert() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Assert on initial best block number.
    assert_block_number_is_best_and_finalized(&mut node, 0, None).await;

    // Mine 5 blocks and assert on the new best block.
    mine_blocks(&mut node, 5, 5, Some(Duration::from_millis(500))).await;

    // Snapshot at block number 5.
    let zero = snapshot(&mut node, U256::ZERO).await;

    // Get known accounts initial balances.
    let (alith_addr, alith_account) = alith();
    let (baltathar_addr, baltathar_account) = baltathar();
    let alith_initial_balance = node.get_balance(alith_account.address(), None).await;
    let baltathar_initial_balance = node.get_balance(baltathar_account.address(), None).await;

    // Initialize a random account. Assume its initial balance is 0.
    let transfer_amount = U256::from(16e17);
    let (_, receipt_info) = do_transfer(
        &mut node,
        alith_addr,
        None,
        transfer_amount,
        Some(BlockWaitTimeout { block_number: 6, timeout: Duration::from_millis(500) }),
    )
    .await;
    let receipt_info = receipt_info.unwrap();

    let dest_h160 = receipt_info.to.unwrap();
    let alith_balance_after_tx0 = node.get_balance(alith_account.address(), None).await;
    let dest_balance = node.get_balance(dest_h160, None).await;
    assert_eq!(
        alith_balance_after_tx0,
        alith_initial_balance
            - AlloyU256::from(receipt_info.effective_gas_price * receipt_info.gas_used).inner()
            - transfer_amount,
        "alith's balance should have changed"
    );
    assert_eq!(dest_balance, transfer_amount, "dest's balance should have changed");
    assert_eq!(receipt_info.block_number, pallet_revive::U256::from(6));
    assert_eq!(receipt_info.transaction_index, pallet_revive::U256::one());

    // Make another regular transfer between known accounts.
    let transfer_amount = U256::from(1e17);
    let (_, receipt_info) = do_transfer(
        &mut node,
        baltathar_addr,
        Some(alith_addr),
        transfer_amount,
        Some(BlockWaitTimeout { block_number: 7, timeout: Duration::from_millis(500) }),
    )
    .await;
    let receipt_info = receipt_info.unwrap();

    assert_eq!(receipt_info.block_number, pallet_revive::U256::from(7));
    assert_eq!(receipt_info.transaction_index, pallet_revive::U256::one());
    let alith_final_balance = node.get_balance(alith_account.address(), None).await;
    let baltathar_final_balance = node.get_balance(baltathar_account.address(), None).await;
    assert_eq!(
        baltathar_final_balance,
        baltathar_initial_balance
            - transfer_amount
            - AlloyU256::from(receipt_info.effective_gas_price * receipt_info.gas_used).inner(),
        "Baltathar's balance should have changed"
    );
    assert_eq!(
        alith_final_balance,
        alith_balance_after_tx0 + transfer_amount,
        "Alith's balance should have changed"
    );

    // Revert to a block before the transactions have been mined.
    revert(&mut node, zero, 5, true, Some(Duration::from_millis(500))).await;

    // Assert on accounts balances to be the initial balances.
    let dest_addr = Address::from(dest_h160.to_fixed_bytes());
    let alith_balance = node.get_balance(alith_account.address(), None).await;
    let baltathar_balance = node.get_balance(baltathar_account.address(), None).await;
    let dest_balance = node.get_balance(dest_h160, None).await;
    assert_eq!(alith_balance, alith_initial_balance);
    assert_eq!(baltathar_balance, baltathar_initial_balance);
    assert_eq!(dest_balance, U256::ZERO);
    assert_eq!(node.get_nonce(alith_addr).await, U256::ZERO);
    assert_eq!(node.get_nonce(baltathar_addr).await, U256::ZERO);
    assert_eq!(node.get_nonce(dest_addr).await, U256::ZERO);

    // Remine the 6th block with same txs above.
    let (tx_hash1, _) =
        do_transfer(&mut node, alith_addr, Some(dest_addr), U256::from(16e17), None).await;
    let (tx_hash2, receipt_info2) = do_transfer(
        &mut node,
        baltathar_addr,
        Some(alith_addr),
        U256::from(1e17),
        Some(BlockWaitTimeout { block_number: 6, timeout: Duration::from_millis(500) }),
    )
    .await;
    let receipt_info2 = receipt_info2.unwrap();
    let receipt_info = node.get_transaction_receipt(tx_hash1).await;
    let mut tx_indices =
        HashSet::from([pallet_revive::U256::from(1), pallet_revive::U256::from(2)]);
    assert_eq!(receipt_info.block_number, pallet_revive::U256::from(6));
    assert!(tx_indices.remove(&receipt_info.transaction_index));
    assert_eq!(receipt_info.transaction_hash, tx_hash1);
    assert_eq!(receipt_info2.block_number, pallet_revive::U256::from(6));
    assert!(tx_indices.remove(&receipt_info2.transaction_index));
    assert_eq!(receipt_info2.transaction_hash, tx_hash2);
    assert_eq!(node.get_nonce(alith_addr).await, U256::ONE);
    assert_eq!(node.get_nonce(baltathar_addr).await, U256::ONE);
    assert_eq!(node.get_nonce(dest_addr).await, U256::ZERO);
    assert!(tx_indices.is_empty());

    let txs_in_block = unwrap_response::<U256>(
        node.eth_rpc(EthRequest::EthGetTransactionCountByNumber(
            alloy_eips::BlockNumberOrTag::Latest,
        ))
        .await
        .unwrap(),
    )
    .unwrap();
    assert_eq!(txs_in_block, U256::from(2));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_evm_revert_and_timestamp() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    // Generate the current timestamp and pass it to anvil config.
    let genesis_timestamp = anvil_node_config.get_genesis_timestamp();
    let anvil_node_config = anvil_node_config.with_genesis_timestamp(Some(genesis_timestamp));
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Do a first snapshot for genesis.
    let zero = snapshot(&mut node, U256::ZERO).await;

    // Assert on first best block number.
    mine_blocks(&mut node, 1, 1, None).await;
    let first_timestamp = node.get_decoded_timestamp(None).await;
    assert_with_tolerance(
        first_timestamp.saturating_div(1000),
        genesis_timestamp,
        1,
        "wrong timestamp at first block",
    );

    let second_timestamp = first_timestamp.saturating_add(3000);
    assert_with_tolerance(
        unwrap_response::<u64>(
            node.eth_rpc(EthRequest::EvmSetTime(U256::from(second_timestamp.saturating_div(1000))))
                .await
                .unwrap(),
        )
        .unwrap(),
        3,
        1,
        "Wrong offset 1",
    );

    // Mine 1 blocks and assert on the new best block.
    mine_blocks(&mut node, 1, 2, None).await;
    let second_timestamp = node.get_decoded_timestamp(None).await;
    assert_with_tolerance(
        second_timestamp.saturating_sub(first_timestamp),
        3000,
        300,
        "wrong timestamp at second block",
    );

    // Snapshot at block number 2 and then mine 1 more block.
    let one = snapshot(&mut node, U256::ONE).await;

    // Seconds
    let third_timestamp = second_timestamp.saturating_add(3000);
    assert_with_tolerance(
        unwrap_response::<u64>(
            node.eth_rpc(EthRequest::EvmSetTime(U256::from(third_timestamp.saturating_div(1000))))
                .await
                .unwrap(),
        )
        .unwrap(),
        3,
        1,
        "Wrong offset 2",
    );

    mine_blocks(&mut node, 1, 3, None).await;
    let third_timestamp = node.get_decoded_timestamp(None).await;
    assert_with_tolerance(
        third_timestamp.saturating_sub(second_timestamp),
        3000,
        300,
        "wrong timestamp at third block",
    );

    // Revert to block number 2.
    revert(&mut node, one, 2, true, None).await;
    let seconds_ts_after_revert = node.get_decoded_timestamp(None).await;
    assert_with_tolerance(
        seconds_ts_after_revert.saturating_sub(second_timestamp),
        0,
        5,
        "wrong timestamp at reverted second block",
    );

    // Mine again 1 block and check again the timestamp. We should have the next block timestamp
    // with 1 second later than the second block timestamp.
    tokio::time::sleep(Duration::from_secs(1)).await;
    mine_blocks(&mut node, 1, 3, None).await;
    let remined_third_block_ts = node.get_decoded_timestamp(None).await;
    assert_with_tolerance(
        remined_third_block_ts.saturating_sub(second_timestamp),
        1000,
        300,
        "wrong timestamp at remined third block",
    );

    // Revert to genesis block number.
    revert(&mut node, zero, 0, true, None).await;
    let reverted_genesis_block_ts = node.get_decoded_timestamp(None).await;
    assert_with_tolerance(
        reverted_genesis_block_ts.saturating_div(1000),
        genesis_timestamp,
        0,
        "wrong timestamp at reverted genesis block",
    );

    // Mine 1 block and check the timestamp. We don't check on a specific
    // timestamp, but expect the time has increased a bit since the revert, which set the time back
    // to genesis timestamp.
    mine_blocks(&mut node, 1, 1, None).await;
    assert_eq!(node.best_block_number().await, 1);
    let remined_first_block_ts = node.get_decoded_timestamp(None).await;
    // Here assert that the time is increasing.
    assert!(remined_first_block_ts > genesis_timestamp * 1000);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_rollback() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Assert on initial best block number.
    assert_block_number_is_best_and_finalized(&mut node, 0, None).await;

    // Mine 5 blocks and assert on the new best block.
    mine_blocks(&mut node, 5, 5, Some(Duration::from_millis(500))).await;

    // Rollback 2 blocks.
    unwrap_response::<()>(node.eth_rpc(EthRequest::Rollback(Some(2))).await.unwrap()).unwrap();
    assert_block_number_is_best_and_finalized(&mut node, 3, Some(Duration::from_millis(500))).await;

    // Check mining works fine after reverting.
    mine_blocks(&mut node, 10, 13, Some(Duration::from_millis(500))).await;

    // Rollback 1 block.
    unwrap_response::<()>(node.eth_rpc(EthRequest::Rollback(None)).await.unwrap()).unwrap();
    assert_block_number_is_best_and_finalized(&mut node, 12, Some(Duration::from_millis(500)))
        .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mine_with_txs_in_mempool_before_revert() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Assert on initial best block number.
    assert_block_number_is_best_and_finalized(&mut node, 0, None).await;

    // Mine 5 blocks and assert on the new best block.
    mine_blocks(&mut node, 5, 5, Some(Duration::from_millis(500))).await;

    // Snapshot at block number 5.
    let zero = snapshot(&mut node, U256::ZERO).await;
    mine_blocks(&mut node, 5, 10, None).await;

    // Get known accounts.
    let (alith_addr, _) = alith();
    let (baltathar_addr, _) = baltathar();

    // Initialize a random account.
    let transfer_amount = U256::from(16e17);
    let (dest_addr, _) =
        node.eth_transfer_to_unitialized_random_account(alith_addr, transfer_amount, None).await;

    // Make another regular transfer between known accounts.
    let transfer_amount = U256::from(1e17);
    let transaction =
        TransactionRequest::default().value(transfer_amount).from(baltathar_addr).to(alith_addr);
    let _ = node.send_transaction(transaction, None).await.unwrap();

    // Revert to a block before the transactions have been sent.
    revert(&mut node, zero, 5, true, None).await;
    let one = snapshot(&mut node, U256::ONE).await;

    mine_blocks(&mut node, 1, 6, Some(Duration::from_millis(500))).await;

    let txs_in_block = unwrap_response::<U256>(
        node.eth_rpc(EthRequest::EthGetTransactionCountByNumber(
            alloy_eips::BlockNumberOrTag::Latest,
        ))
        .await
        .unwrap(),
    )
    .unwrap();
    assert_eq!(txs_in_block, U256::from(2));

    // Now make two more txs again with same senders, with different nonces than the actual
    // accounts nonces at block 5.
    let transfer_amount = U256::from(1e15);
    do_transfer(&mut node, baltathar_addr, Some(alith_addr), transfer_amount, None).await;
    do_transfer(&mut node, alith_addr, Some(dest_addr), transfer_amount, None).await;
    revert(&mut node, one, 5, true, None).await;

    let txs_in_block = unwrap_response::<U256>(
        node.eth_rpc(EthRequest::EthGetTransactionCountByNumber(
            alloy_eips::BlockNumberOrTag::Latest,
        ))
        .await
        .unwrap(),
    )
    .unwrap();
    assert_eq!(txs_in_block, U256::ZERO);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_timestmap_in_contract_after_revert() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    // Generate the current timestamp and pass it to anvil config.
    let genesis_timestamp = anvil_node_config.get_genesis_timestamp();
    let anvil_node_config = anvil_node_config.with_genesis_timestamp(Some(genesis_timestamp));
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Deploy multicall contract
    let alith = Account::from(subxt_signer::eth::dev::alith());
    let contract_code = get_contract_code("Multicall");
    let tx_hash = node.deploy_contract(&contract_code.init, alith.address(), None).await;
    mine_blocks(&mut node, 1, 1, Some(Duration::from_millis(500))).await;

    let first_timestamp = node.get_decoded_timestamp(None).await;
    assert_with_tolerance(
        first_timestamp.saturating_div(1000),
        genesis_timestamp,
        1,
        "wrong timestamp at first block",
    );

    // Make a snapshot
    let zero = snapshot(&mut node, U256::ZERO).await;

    // Get contract address.
    let receipt = node.get_transaction_receipt(tx_hash).await;
    assert_eq!(receipt.status, Some(pallet_revive::U256::from(1)));
    let contract_address = receipt.contract_address.unwrap();

    // Get contract timestamp after first block and expect it to be the genesis timestamp
    // mostly.
    let alith_addr = Address::from(alith.address().to_fixed_bytes());
    let contract_address = Address::from(contract_address.to_fixed_bytes());
    let timestamp = multicall_get_timestamp(&mut node, alith_addr, contract_address).await;
    assert_eq!(timestamp, U256::from(first_timestamp.saturating_div(1000)));

    let second_timestamp = first_timestamp.saturating_add(3000);
    assert_with_tolerance(
        unwrap_response::<u64>(
            node.eth_rpc(EthRequest::EvmSetTime(U256::from(second_timestamp.saturating_div(1000))))
                .await
                .unwrap(),
        )
        .unwrap(),
        3,
        1,
        "Wrong offset 1",
    );

    // After setting the time, still expect to get the block 1 timestamp with the contract call.
    let timestamp = multicall_get_timestamp(&mut node, alith_addr, contract_address).await;
    assert_eq!(timestamp, U256::from(first_timestamp.saturating_div(1000)));

    // Mine 1 block again and expect on the set timestamp.
    mine_blocks(&mut node, 1, 2, Some(Duration::from_millis(500))).await;
    let second_timestamp = node.get_decoded_timestamp(None).await;
    assert_with_tolerance(
        second_timestamp.saturating_sub(first_timestamp),
        3000,
        350,
        "wrong timestamp at second block",
    );

    // The contract call should return with same block 2 timestamp.
    let timestamp = multicall_get_timestamp(&mut node, alith_addr, contract_address).await;
    assert_with_tolerance(
        U256::from(second_timestamp.saturating_div(1000)),
        timestamp,
        U256::ZERO,
        "wrong timestamp after mining second block",
    );

    // Now check we got back to timestamp after the first block when reverting.
    revert(&mut node, zero, 1, true, None).await;
    let timestamp = multicall_get_timestamp(&mut node, alith_addr, contract_address).await;
    assert_with_tolerance(
        U256::from(first_timestamp.saturating_div(1000)),
        timestamp,
        U256::ZERO,
        "wrong timestamp after reverting to first block",
    );
}
