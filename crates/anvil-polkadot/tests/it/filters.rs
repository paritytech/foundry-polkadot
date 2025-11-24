use crate::{
    abi::SimpleStorage::{self as SimpleStorage},
    revert::{revert, snapshot},
    utils::{TestNode, get_contract_code, unwrap_response},
};
use alloy_eips::BlockNumberOrTag;
use alloy_primitives::{Address, B256, U256};
use alloy_rpc_types::{Filter, TransactionInput, TransactionRequest};
use alloy_sol_types::{SolCall, SolEvent};
use anvil_core::eth::EthRequest;
use anvil_polkadot::{
    api_server::{filters::Filters, revive_conversions::ReviveAddress},
    config::{AnvilNodeConfig, SubstrateNodeConfig},
};
use polkadot_sdk::{
    pallet_revive::evm::{Account, Log},
    sp_core::keccak_256,
};
use std::collections::HashSet;
use subxt::utils::H256;

#[tokio::test(flavor = "multi_thread")]
async fn test_block_filter_receives_new_blocks() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Create a block filter
    let id =
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthNewBlockFilter(())).await.unwrap())
            .unwrap();

    // Mine a new block
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    // Get block hash
    let block_hash = node.block_hash_by_number(1).await.unwrap();

    let block_hashes_notified = unwrap_response::<Vec<H256>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(id.clone())).await.unwrap(),
    )
    .unwrap();

    // Check that hash for block number 1 is in between hashes we were notified about
    assert!(block_hashes_notified.contains(&block_hash));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_filter_returns_empty_when_no_new_blocks() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Create a block filter
    let id =
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthNewBlockFilter(())).await.unwrap())
            .unwrap();

    let block_hashes_notified = unwrap_response::<Vec<H256>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(id.clone())).await.unwrap(),
    )
    .unwrap();
    assert_eq!(block_hashes_notified.len(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_filter_only_returns_new_blocks_since_last_poll() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Create a block filter
    let id =
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthNewBlockFilter(())).await.unwrap())
            .unwrap();

    // Mine a new block
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    let block_hashes_notified = unwrap_response::<Vec<H256>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(id.clone())).await.unwrap(),
    )
    .unwrap();
    assert!(!block_hashes_notified.is_empty());
    assert!(block_hashes_notified.contains(&node.block_hash_by_number(1).await.unwrap()));
    // Mine some more blocks
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(5)), None)).await.unwrap())
        .unwrap();
    let block_hashes_notified = unwrap_response::<Vec<H256>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(id.clone())).await.unwrap(),
    )
    .unwrap();

    for (index, hash) in block_hashes_notified.iter().enumerate() {
        assert_eq!(*hash, node.block_hash_by_number(index as u32 + 2).await.unwrap());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_uninstall_filter() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Create a block filter
    let id =
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthNewBlockFilter(())).await.unwrap())
            .unwrap();
    // Check that we can successfully rmeove the filter
    assert!(
        unwrap_response::<bool>(
            node.eth_rpc(EthRequest::EthUninstallFilter(id.clone())).await.unwrap()
        )
        .unwrap()
    );
    // Check that we can not remove the filter a second time
    assert!(
        !unwrap_response::<bool>(
            node.eth_rpc(EthRequest::EthUninstallFilter(id.clone())).await.unwrap()
        )
        .unwrap()
    );

    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(5)), None)).await.unwrap())
        .unwrap();
    // Try to poll a filter that does not exist
    assert_eq!(
        unwrap_response::<Vec<H256>>(
            node.eth_rpc(EthRequest::EthGetFilterChanges(id.clone())).await.unwrap(),
        )
        .unwrap()
        .len(),
        0
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_multiple_filters_receive_same_blocks() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Create two block filters
    let id1 =
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthNewBlockFilter(())).await.unwrap())
            .unwrap();
    let id2 =
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthNewBlockFilter(())).await.unwrap())
            .unwrap();

    // Mine a few blocks
    unwrap_response::<()>(
        node.eth_rpc(EthRequest::Mine(Some(U256::from(10)), None)).await.unwrap(),
    )
    .unwrap();
    let block_hashes_notified1 = unwrap_response::<Vec<H256>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(id1.clone())).await.unwrap(),
    )
    .unwrap();
    let block_hashes_notified2 = unwrap_response::<Vec<H256>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(id2.clone())).await.unwrap(),
    )
    .unwrap();
    assert!(!block_hashes_notified1.is_empty());
    assert_eq!(block_hashes_notified1, block_hashes_notified2);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_filter_after_snapshot() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    // Create a block filter
    let id =
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthNewBlockFilter(())).await.unwrap())
            .unwrap();
    // Mine two blocks
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(2)), None)).await.unwrap())
        .unwrap();
    // Create a snapshot
    let zero = snapshot(&mut node, U256::ZERO).await;
    // Mine two more blocks (bn3 and bn4)
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(2)), None)).await.unwrap())
        .unwrap();
    // Get the hashes of the mined blocks
    let block_hashes_notified = unwrap_response::<Vec<H256>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(id.clone())).await.unwrap(),
    )
    .unwrap();

    // Check that the received hashes are equal with the hashes of blocks 1..4
    for (index, hash) in block_hashes_notified.iter().enumerate() {
        assert_eq!(*hash, node.block_hash_by_number(index as u32).await.unwrap());
    }
    // Go back to the block number 2.
    revert(&mut node, zero, 2, true).await;
    // Mine blocks [3,4,5,6,7]
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(5)), None)).await.unwrap())
        .unwrap();
    // Get the hashes of the mined blocks
    let block_hashes_notified = unwrap_response::<Vec<H256>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(id.clone())).await.unwrap(),
    )
    .unwrap();
    // Check that the received hashes are equal with the hashes of blocks 3..7
    for (index, hash) in block_hashes_notified.iter().enumerate() {
        assert_eq!(*hash, node.block_hash_by_number(index as u32 + 3).await.unwrap());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_filter_is_evicted() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new_inner(
        anvil_node_config.clone(),
        substrate_node_config,
        Filters::with_keepalive(std::time::Duration::from_secs(2)),
    )
    .await
    .unwrap();

    // Create a block filter
    let id =
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthNewBlockFilter(())).await.unwrap())
            .unwrap();
    // Mine a new block
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    // Get the hashes of the mined blocks
    let block_hashes_notified = unwrap_response::<Vec<H256>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(id.clone())).await.unwrap(),
    )
    .unwrap();
    assert!(!block_hashes_notified.is_empty());
    // Wait for the filter to expire
    tokio::time::sleep(std::time::Duration::from_secs(4)).await;
    // Mine five blocks
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(5)), None)).await.unwrap())
        .unwrap();
    let block_hashes_notified = unwrap_response::<Vec<H256>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(id.clone())).await.unwrap(),
    )
    .unwrap();
    assert_eq!(block_hashes_notified.len(), 0);
}

// ======= Logs filter

/// Checks that the values from the log are the ones we expect.
fn assert_decoded_simple_storage_data(log: &Log, old: U256, new: U256, changer: Address) {
    let alloy_topics: Vec<_> =
        log.topics.iter().map(|t| alloy_primitives::B256::from_slice(t.as_bytes())).collect();

    let decoded =
        SimpleStorage::ValueChanged::decode_raw_log(alloy_topics, &log.data.as_ref().unwrap().0)
            .unwrap();

    assert_eq!(decoded.oldValue, old);
    assert_eq!(decoded.newValue, new);
    assert_eq!(decoded.changer, changer);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_logs_filter_receives_new_logs() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_address = alith.address();

    // Deploy contract
    let contract_code = get_contract_code("SimpleStorage");
    let tx_hash = node.deploy_contract(&contract_code.init, alith.address()).await;
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    let receipt = node.get_transaction_receipt(tx_hash).await;
    let contract_address = receipt.contract_address.unwrap();

    // Create a new logs filter for the contract_address
    let filter_id = unwrap_response::<String>(
        node.eth_rpc(EthRequest::EthNewFilter(
            Filter::new().address(Address::from(ReviveAddress::new(contract_address))),
        ))
        .await
        .unwrap(),
    )
    .unwrap();
    // Check that before interacting with the contract there are no logs
    let logs = unwrap_response::<Vec<Log>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(filter_id.clone())).await.unwrap(),
    )
    .unwrap();
    assert!(logs.is_empty());

    // Emit an event by calling setValue
    let set_value_data = SimpleStorage::setValueCall::new((U256::from(511),)).abi_encode();
    let call_tx = TransactionRequest::default()
        .from(Address::from(ReviveAddress::new(alith_address)))
        .to(Address::from(ReviveAddress::new(contract_address)))
        .input(TransactionInput::both(set_value_data.into()))
        .nonce(1);
    node.send_transaction(call_tx).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    let logs = unwrap_response::<Vec<Log>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(filter_id.clone())).await.unwrap(),
    )
    .unwrap();

    // Check the number of logs
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].address, contract_address);
    // Check that our topic is the ValueChanged event.
    let event_hash = keccak_256(b"ValueChanged(address,uint256,uint256)");
    assert_eq!(logs[0].topics[0], H256::from(event_hash));

    assert_decoded_simple_storage_data(
        &logs[0],
        U256::from(0),
        U256::from(511),
        Address::from(ReviveAddress::new(Account::from(subxt_signer::eth::dev::alith()).address())),
    );

    // Emit the second event
    let set_value_data = SimpleStorage::setValueCall::new((U256::from(200),)).abi_encode();
    let call_tx = TransactionRequest::default()
        .from(Address::from(ReviveAddress::new(alith_address)))
        .to(Address::from(ReviveAddress::new(contract_address)))
        .input(TransactionInput::both(set_value_data.into()))
        .nonce(2);
    node.send_transaction(call_tx).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    // Get second batch - should only contain new logs
    let logs2 = unwrap_response::<Vec<Log>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(filter_id.clone())).await.unwrap(),
    )
    .unwrap();
    assert_eq!(logs2.len(), 1);
    assert_decoded_simple_storage_data(
        &logs2[0],
        U256::from(511),
        U256::from(200),
        Address::from(ReviveAddress::new(Account::from(subxt_signer::eth::dev::alith()).address())),
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_logs_filter_returns_historic_logs_on_creation() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_address = alith.address();

    // Deploy contract
    let contract_code = get_contract_code("SimpleStorage");
    let tx_hash = node.deploy_contract(&contract_code.init, alith.address()).await;
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    let receipt = node.get_transaction_receipt(tx_hash).await;
    let contract_address = receipt.contract_address.unwrap();

    // Emit events before creating the filter
    for i in 0..3 {
        let set_value_data = SimpleStorage::setValueCall::new((U256::from(511 + i),)).abi_encode();
        let call_tx = TransactionRequest::default()
            .from(Address::from(ReviveAddress::new(alith_address)))
            .to(Address::from(ReviveAddress::new(contract_address)))
            .input(TransactionInput::both(set_value_data.into()))
            .nonce(i + 1);
        node.send_transaction(call_tx).await.unwrap();
    }
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    // Create filter with fromBlock
    let from_block_filter_id = unwrap_response::<String>(
        node.eth_rpc(EthRequest::EthNewFilter(
            Filter::new()
                .address(Address::from(ReviveAddress::new(contract_address)))
                .from_block(0),
        ))
        .await
        .unwrap(),
    )
    .unwrap();

    // First poll should return historic logs + deployment log
    let from_block_logs = unwrap_response::<Vec<Log>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(from_block_filter_id.clone())).await.unwrap(),
    )
    .unwrap();
    for (i, log) in from_block_logs.iter().enumerate().skip(1) {
        let old_value = if i == 1 { 0 } else { 511 + (i - 2) };
        assert_decoded_simple_storage_data(
            log,
            U256::from(old_value),
            U256::from(511 + (i - 1)),
            Address::from(ReviveAddress::new(
                Account::from(subxt_signer::eth::dev::alith()).address(),
            )),
        );
    }

    let set_value_data = SimpleStorage::setValueCall::new((U256::from(514),)).abi_encode();
    let call_tx = TransactionRequest::default()
        .from(Address::from(ReviveAddress::new(alith_address)))
        .to(Address::from(ReviveAddress::new(contract_address)))
        .input(TransactionInput::both(set_value_data.into()))
        .nonce(4);
    node.send_transaction(call_tx).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    let from_block_logs = unwrap_response::<Vec<Log>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(from_block_filter_id.clone())).await.unwrap(),
    )
    .unwrap();
    assert_eq!(from_block_logs.len(), 1);
    assert_decoded_simple_storage_data(
        &from_block_logs[0],
        U256::from(513),
        U256::from(514),
        Address::from(ReviveAddress::new(Account::from(subxt_signer::eth::dev::alith()).address())),
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_logs_filter_with_block_range() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_address = alith.address();

    // Deploy contract
    let contract_code = get_contract_code("SimpleStorage");
    let tx_hash = node.deploy_contract(&contract_code.init, alith.address()).await;
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    let receipt = node.get_transaction_receipt(tx_hash).await;
    let contract_address = receipt.contract_address.unwrap();

    // Create filter with block range 0-1
    let range_filter_id = unwrap_response::<String>(
        node.eth_rpc(EthRequest::EthNewFilter(
            Filter::new()
                .address(Address::from(ReviveAddress::new(contract_address)))
                .from_block(0)
                .to_block(1),
        ))
        .await
        .unwrap(),
    )
    .unwrap();
    // Emit event in block 2 (outside the range).
    let set_value_data = SimpleStorage::setValueCall::new((U256::from(100),)).abi_encode();
    let call_tx = TransactionRequest::default()
        .from(Address::from(ReviveAddress::new(alith_address)))
        .to(Address::from(ReviveAddress::new(contract_address)))
        .input(TransactionInput::both(set_value_data.into()))
        .nonce(1);
    node.send_transaction(call_tx).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    let logs = unwrap_response::<Vec<Log>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(range_filter_id.clone())).await.unwrap(),
    )
    .unwrap();
    assert_eq!(logs.len(), 1); // deployment, no ValueChanged event.

    // Emit event in block 3 (outside range)
    let set_value_data = SimpleStorage::setValueCall::new((U256::from(200),)).abi_encode();
    let call_tx = TransactionRequest::default()
        .from(Address::from(ReviveAddress::new(alith_address)))
        .to(Address::from(ReviveAddress::new(contract_address)))
        .input(TransactionInput::both(set_value_data.into()))
        .nonce(2);
    node.send_transaction(call_tx).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    // Should not return logs from block 3 (outside toBlock range)
    let logs = unwrap_response::<Vec<Log>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(range_filter_id.clone())).await.unwrap(),
    )
    .unwrap();
    assert_eq!(logs.len(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_logs_filter_with_topics() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_address = alith.address();

    // Deploy contract
    let contract_code = get_contract_code("SimpleStorage");
    let tx_hash = node.deploy_contract(&contract_code.init, alith.address()).await;
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    let receipt = node.get_transaction_receipt(tx_hash).await;
    let contract_address = receipt.contract_address.unwrap();

    // Create a filter that is interested in the ValueChanged events only
    let value_changed_hash = B256::from(keccak_256(b"ValueChanged(address,uint256,uint256)"));

    let filter_id = unwrap_response::<String>(
        node.eth_rpc(EthRequest::EthNewFilter(
            Filter::new()
                .address(Address::from(ReviveAddress::new(contract_address)))
                .event_signature(value_changed_hash),
        ))
        .await
        .unwrap(),
    )
    .unwrap();

    // Call setData
    let set_data_data = SimpleStorage::setDataCall::new(("hello".to_string(),)).abi_encode();
    let call_tx = TransactionRequest::default()
        .from(Address::from(ReviveAddress::new(alith_address)))
        .to(Address::from(ReviveAddress::new(contract_address)))
        .input(TransactionInput::both(set_data_data.into()));
    node.send_transaction(call_tx).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    let logs = unwrap_response::<Vec<Log>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(filter_id.clone())).await.unwrap(),
    )
    .unwrap();

    // Should not find any events because there was no eventChanged emitted.
    assert_eq!(logs.len(), 0);

    let set_value_data = SimpleStorage::setValueCall::new((U256::from(100),)).abi_encode();
    let call_tx = TransactionRequest::default()
        .from(Address::from(ReviveAddress::new(alith_address)))
        .to(Address::from(ReviveAddress::new(contract_address)))
        .input(TransactionInput::both(set_value_data.into()))
        .nonce(2);
    node.send_transaction(call_tx).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    let logs = unwrap_response::<Vec<Log>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(filter_id.clone())).await.unwrap(),
    )
    .unwrap();

    // Should get ValueChanged event
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].topics[0], H256::from(value_changed_hash.as_ref()));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_logs_filter_multiple_events_in_same_transaction() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_address = alith.address();

    // Deploy contract
    let contract_code = get_contract_code("SimpleStorage");
    let tx_hash = node.deploy_contract(&contract_code.init, alith.address()).await;
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    let receipt = node.get_transaction_receipt(tx_hash).await;
    let contract_address = receipt.contract_address.unwrap();

    let filter_id = unwrap_response::<String>(
        node.eth_rpc(EthRequest::EthNewFilter(
            Filter::new().address(Address::from(ReviveAddress::new(contract_address))),
        ))
        .await
        .unwrap(),
    )
    .unwrap();

    let emit_both_data =
        SimpleStorage::emitBothCall::new((U256::from(100), "Cappybara".to_string())).abi_encode();

    let call_tx = TransactionRequest::default()
        .from(Address::from(ReviveAddress::new(alith_address)))
        .to(Address::from(ReviveAddress::new(contract_address)))
        .input(TransactionInput::both(emit_both_data.into()))
        .nonce(1);
    let tx_hash = node.send_transaction(call_tx).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    let logs = unwrap_response::<Vec<Log>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(filter_id.clone())).await.unwrap(),
    )
    .unwrap();

    // Should have 2 events from the same transaction
    assert_eq!(logs.len(), 2);
    // Both logs should be from the same transaction
    assert_eq!(logs[0].transaction_hash, tx_hash);
    assert_eq!(logs[1].transaction_hash, tx_hash);

    // Verify event types
    let value_changed_hash = H256::from(keccak_256(b"ValueChanged(address,uint256,uint256)"));
    let data_updated_hash = H256::from(keccak_256(b"DataUpdated(address,string)"));

    assert_eq!(logs[0].topics[0], value_changed_hash);
    assert_eq!(logs[1].topics[0], data_updated_hash);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_multiple_logs_filters_receive_same_logs() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_address = alith.address();

    // Deploy contract
    let contract_code = get_contract_code("SimpleStorage");
    let tx_hash = node.deploy_contract(&contract_code.init, alith.address()).await;
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    let receipt = node.get_transaction_receipt(tx_hash).await;
    let contract_address = receipt.contract_address.unwrap();

    let filter_id_1 = unwrap_response::<String>(
        node.eth_rpc(EthRequest::EthNewFilter(
            Filter::new().address(Address::from(ReviveAddress::new(contract_address))),
        ))
        .await
        .unwrap(),
    )
    .unwrap();
    let filter_id_2 = unwrap_response::<String>(
        node.eth_rpc(EthRequest::EthNewFilter(
            Filter::new().address(Address::from(ReviveAddress::new(contract_address))),
        ))
        .await
        .unwrap(),
    )
    .unwrap();

    // Emit events
    for i in 0..3 {
        let set_value_data = SimpleStorage::setValueCall::new((U256::from(100 + i),)).abi_encode();
        let call_tx = TransactionRequest::default()
            .from(Address::from(ReviveAddress::new(alith_address)))
            .to(Address::from(ReviveAddress::new(contract_address)))
            .input(TransactionInput::both(set_value_data.into()))
            .nonce(i + 1);
        node.send_transaction(call_tx).await.unwrap();
    }
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    // Both filters should receive the same logs
    let logs1 = unwrap_response::<Vec<Log>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(filter_id_1.clone())).await.unwrap(),
    )
    .unwrap();

    let logs2 = unwrap_response::<Vec<Log>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(filter_id_2.clone())).await.unwrap(),
    )
    .unwrap();
    assert_eq!(logs1, logs2);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_logs_filter_after_snapshot_and_revert() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_address = alith.address();

    // Deploy contract
    let contract_code = get_contract_code("SimpleStorage");
    let tx_hash = node.deploy_contract(&contract_code.init, alith.address()).await;
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    let receipt = node.get_transaction_receipt(tx_hash).await;
    let contract_address = receipt.contract_address.unwrap();

    let filter_id = unwrap_response::<String>(
        node.eth_rpc(EthRequest::EthNewFilter(
            Filter::new().address(Address::from(ReviveAddress::new(contract_address))),
        ))
        .await
        .unwrap(),
    )
    .unwrap();

    // Emit event in block 2
    let set_value_data = SimpleStorage::setValueCall::new((U256::from(100),)).abi_encode();
    let call_tx = TransactionRequest::default()
        .from(Address::from(ReviveAddress::new(alith_address)))
        .to(Address::from(ReviveAddress::new(contract_address)))
        .input(TransactionInput::both(set_value_data.into()))
        .nonce(1);
    node.send_transaction(call_tx).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    // Take snapshot
    let snapshot_id = snapshot(&mut node, U256::ZERO).await;

    // Emit event in block 3
    let set_value_data = SimpleStorage::setValueCall::new((U256::from(200),)).abi_encode();
    let call_tx = TransactionRequest::default()
        .from(Address::from(ReviveAddress::new(alith_address)))
        .to(Address::from(ReviveAddress::new(contract_address)))
        .input(TransactionInput::both(set_value_data.into()))
        .nonce(2);
    node.send_transaction(call_tx).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    let logs_before_revert = unwrap_response::<Vec<Log>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(filter_id.clone())).await.unwrap(),
    )
    .unwrap();
    assert_eq!(logs_before_revert.len(), 2);

    // Revert to snapshot
    revert(&mut node, snapshot_id, 2, true).await;

    // Emit different event in new block 3 (after revert)
    let set_value_data = SimpleStorage::setValueCall::new((U256::from(300),)).abi_encode();
    let call_tx = TransactionRequest::default()
        .from(Address::from(ReviveAddress::new(alith_address)))
        .to(Address::from(ReviveAddress::new(contract_address)))
        .input(TransactionInput::both(set_value_data.into()))
        .nonce(2);
    node.send_transaction(call_tx).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    let logs_after_revert = unwrap_response::<Vec<Log>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(filter_id.clone())).await.unwrap(),
    )
    .unwrap();

    // Should get the new log from the alternate timeline
    assert_eq!(logs_after_revert.len(), 1);

    assert_decoded_simple_storage_data(
        &logs_after_revert[0],
        U256::from(100),
        U256::from(300),
        Address::from(ReviveAddress::new(Account::from(subxt_signer::eth::dev::alith()).address())),
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_logs_filter_with_multiple_addresses() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_address = alith.address();

    // Deploy first contract
    let contract_code = get_contract_code("SimpleStorage");
    let tx_hash1 = node.deploy_contract(&contract_code.init, alith.address()).await;
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    let receipt1 = node.get_transaction_receipt(tx_hash1).await;
    let contract1 = receipt1.contract_address.unwrap();

    // Deploy second contract
    let tx_hash2 = node.deploy_contract(&contract_code.init, alith.address()).await;
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    let receipt2 = node.get_transaction_receipt(tx_hash2).await;
    let contract2 = receipt2.contract_address.unwrap();

    // Create filter for both contracts
    let filter_id = unwrap_response::<String>(
        node.eth_rpc(EthRequest::EthNewFilter(Filter::new().address(vec![
            Address::from(ReviveAddress::new(contract1)),
            Address::from(ReviveAddress::new(contract2)),
        ])))
        .await
        .unwrap(),
    )
    .unwrap();

    // Emit event from first contract
    let set_value_data = SimpleStorage::setValueCall::new((U256::from(100),)).abi_encode();
    let call_tx = TransactionRequest::default()
        .from(Address::from(ReviveAddress::new(alith_address)))
        .to(Address::from(ReviveAddress::new(contract1)))
        .input(TransactionInput::both(set_value_data.into()))
        .nonce(2);
    node.send_transaction(call_tx).await.unwrap();

    // Emit event from second contract
    let set_value_data = SimpleStorage::setValueCall::new((U256::from(200),)).abi_encode();
    let call_tx = TransactionRequest::default()
        .from(Address::from(ReviveAddress::new(alith_address)))
        .to(Address::from(ReviveAddress::new(contract2)))
        .input(TransactionInput::both(set_value_data.into()))
        .nonce(3);
    node.send_transaction(call_tx).await.unwrap();

    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    let logs = unwrap_response::<Vec<Log>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(filter_id.clone())).await.unwrap(),
    )
    .unwrap();

    // Should have events from both contracts
    assert_eq!(logs.len(), 2);

    // Verify both contract addresses appear
    let addresses: HashSet<_> = logs.iter().map(|l| l.address).collect();
    assert!(addresses.contains(&contract1));
    assert!(addresses.contains(&contract2));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_logs_filter_future_block_number() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_address = alith.address();

    // Deploy contract
    let contract_code = get_contract_code("SimpleStorage");
    let tx_hash = node.deploy_contract(&contract_code.init, alith.address()).await;
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    let receipt = node.get_transaction_receipt(tx_hash).await;
    let contract_address = receipt.contract_address.unwrap();

    let current_block = node.best_block_number().await;
    // Create filter with future block number
    let filter_id = unwrap_response::<String>(
        node.eth_rpc(EthRequest::EthNewFilter(
            Filter::new()
                .address(Address::from(ReviveAddress::new(contract_address)))
                .from_block(BlockNumberOrTag::Number((current_block + 1000) as u64)),
        ))
        .await
        .unwrap(),
    )
    .unwrap();

    // Emit event in current timeline
    let set_value_data = SimpleStorage::setValueCall::new((U256::from(100),)).abi_encode();
    let call_tx = TransactionRequest::default()
        .from(Address::from(ReviveAddress::new(alith_address)))
        .to(Address::from(ReviveAddress::new(contract_address)))
        .input(TransactionInput::both(set_value_data.into()))
        .nonce(1);
    node.send_transaction(call_tx).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    let logs = unwrap_response::<Vec<Log>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(filter_id.clone())).await.unwrap(),
    )
    .unwrap();

    assert_eq!(logs.len(), 0);

    unwrap_response::<()>(
        node.eth_rpc(EthRequest::Mine(Some(U256::from(1001)), None)).await.unwrap(),
    )
    .unwrap();
    let set_value_data = SimpleStorage::setValueCall::new((U256::from(511),)).abi_encode();
    let call_tx = TransactionRequest::default()
        .from(Address::from(ReviveAddress::new(alith_address)))
        .to(Address::from(ReviveAddress::new(contract_address)))
        .input(TransactionInput::both(set_value_data.into()))
        .nonce(2);
    node.send_transaction(call_tx).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    let logs = unwrap_response::<Vec<Log>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(filter_id.clone())).await.unwrap(),
    )
    .unwrap();
    assert_eq!(logs.len(), 1);
    assert_decoded_simple_storage_data(
        &logs[0],
        U256::from(100),
        U256::from(511),
        Address::from(ReviveAddress::new(Account::from(subxt_signer::eth::dev::alith()).address())),
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_filter_logs_returns_all_matching_logs() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_address = alith.address();

    // Deploy contract
    let contract_code = get_contract_code("SimpleStorage");
    let tx_hash = node.deploy_contract(&contract_code.init, alith.address()).await;
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    let receipt = node.get_transaction_receipt(tx_hash).await;
    let contract_address = receipt.contract_address.unwrap();

    let filter_id = unwrap_response::<String>(
        node.eth_rpc(EthRequest::EthNewFilter(
            Filter::new()
                .address(Address::from(ReviveAddress::new(contract_address)))
                .from_block(1),
        ))
        .await
        .unwrap(),
    )
    .unwrap();

    // Emit multiple events
    for i in 0..3 {
        let set_value_data = SimpleStorage::setValueCall::new((U256::from(100 + i),)).abi_encode();
        let call_tx = TransactionRequest::default()
            .from(Address::from(ReviveAddress::new(alith_address)))
            .to(Address::from(ReviveAddress::new(contract_address)))
            .input(TransactionInput::both(set_value_data.into()))
            .nonce(i + 1);
        node.send_transaction(call_tx).await.unwrap();
        unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    }

    // Get filter changes (clears the filter)
    let logs_from_changes = unwrap_response::<Vec<Log>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(filter_id.clone())).await.unwrap(),
    )
    .unwrap();
    assert_eq!(logs_from_changes.len(), 4);

    let set_value_data = SimpleStorage::setValueCall::new((U256::from(511),)).abi_encode();
    let call_tx = TransactionRequest::default()
        .from(Address::from(ReviveAddress::new(alith_address)))
        .to(Address::from(ReviveAddress::new(contract_address)))
        .input(TransactionInput::both(set_value_data.into()))
        .nonce(4);
    node.send_transaction(call_tx).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    // eth_getFilterLogs should still return ALL logs matching the filter, not just new ones
    let all_logs = unwrap_response::<Vec<Log>>(
        node.eth_rpc(EthRequest::EthGetFilterLogs(filter_id.clone())).await.unwrap(),
    )
    .unwrap();

    let logs_from_changes = unwrap_response::<Vec<Log>>(
        node.eth_rpc(EthRequest::EthGetFilterChanges(filter_id.clone())).await.unwrap(),
    )
    .unwrap();
    assert_eq!(logs_from_changes.len(), 1);

    assert_eq!(all_logs.len(), 5);
}
