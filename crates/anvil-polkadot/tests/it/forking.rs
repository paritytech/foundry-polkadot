use std::time::Duration;

use crate::{
    abi::SimpleStorage,
    utils::{TestNode, get_contract_code, simplestorage_get_value, unwrap_response},
};
use alloy_primitives::{Address, Bytes, U256};
use alloy_rpc_types::{TransactionInput, TransactionRequest};
use alloy_sol_types::SolCall;
use anvil_core::eth::EthRequest;
use anvil_polkadot::{
    api_server::revive_conversions::ReviveAddress,
    config::{AnvilNodeConfig, ForkChoice, SubstrateNodeConfig},
};
use polkadot_sdk::pallet_revive::evm::Account;

/// Westend Asset Hub zombienet local URL for forking tests
/// This URL should point to a running zombienet instance,
/// so running the tests depending on it should be preceded
/// by setting up a zombienet network with a running RPC.
const WESTEND_ASSET_HUB_URL: &str = "http://127.0.0.1:63982";

/// Tests that forking preserves state from the source chain and allows local modifications
#[tokio::test(flavor = "multi_thread")]
async fn test_fork_preserves_state_and_allows_modifications() {
    // Step 1: Create the "source" node
    let source_config = AnvilNodeConfig::test_config();
    let source_substrate_config = SubstrateNodeConfig::new(&source_config);
    let mut source_node =
        TestNode::new(source_config.clone(), source_substrate_config).await.unwrap();

    let source_substrate_rpc_port = source_node.substrate_rpc_port();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let baltathar = Account::from(subxt_signer::eth::dev::baltathar());
    let alith_address = ReviveAddress::new(alith.address());
    let baltathar_address = ReviveAddress::new(baltathar.address());

    // Step 2: Perform a transaction on the source node
    let initial_alith_balance = source_node.get_balance(alith.address(), None).await;
    let initial_baltathar_balance = source_node.get_balance(baltathar.address(), None).await;

    let transfer_amount = U256::from_str_radix("5000000000000000000", 10).unwrap(); // 5 ether
    let transaction = TransactionRequest::default()
        .value(transfer_amount)
        .from(Address::from(alith_address))
        .to(Address::from(baltathar_address));

    source_node.send_transaction(transaction).await.unwrap();
    unwrap_response::<()>(source_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap())
        .unwrap();

    // Verify the transfer happened on source node
    let source_alith_balance = source_node.get_balance(alith.address(), None).await;
    let source_baltathar_balance = source_node.get_balance(baltathar.address(), None).await;

    assert!(
        source_alith_balance < initial_alith_balance, // This is to not check the gas fee
        "Alith balance should decrease on source node"
    );
    assert_eq!(
        source_baltathar_balance,
        initial_baltathar_balance + transfer_amount,
        "Baltathar should receive 5 ether on source node"
    );

    // Step 3: Create a forked node pointing to the source node's Substrate RPC
    let source_rpc_url = format!("http://127.0.0.1:{source_substrate_rpc_port}");

    let fork_config =
        AnvilNodeConfig::test_config().with_port(0).with_eth_rpc_url(Some(source_rpc_url));

    let fork_substrate_config = SubstrateNodeConfig::new(&fork_config);
    let mut fork_node = TestNode::new(fork_config.clone(), fork_substrate_config).await.unwrap();

    // Step 4: Verify the forked node has the same balances as the source node at fork point
    let fork_alith_balance = fork_node.get_balance(alith.address(), None).await;
    let fork_baltathar_balance = fork_node.get_balance(baltathar.address(), None).await;

    assert_eq!(
        fork_alith_balance, source_alith_balance,
        "Forked node should have same Alith balance as source"
    );
    assert_eq!(
        fork_baltathar_balance, source_baltathar_balance,
        "Forked node should have same Baltathar balance as source"
    );

    // Step 5: Perform a transaction on the forked node (send 1 ether)
    let fork_transfer_amount = U256::from_str_radix("1000000000000000000", 10).unwrap(); // 1 ether
    let fork_transaction = TransactionRequest::default()
        .value(fork_transfer_amount)
        .from(Address::from(alith_address))
        .to(Address::from(baltathar_address));

    fork_node.send_transaction(fork_transaction).await.unwrap();
    unwrap_response::<()>(fork_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    // Step 6: Verify the transaction affected only the forked node
    let fork_alith_balance_after = fork_node.get_balance(alith.address(), None).await;
    let fork_baltathar_balance_after = fork_node.get_balance(baltathar.address(), None).await;

    assert!(
        fork_alith_balance_after < fork_alith_balance,
        "Alith balance should decrease on forked node"
    );
    assert_eq!(
        fork_baltathar_balance_after,
        fork_baltathar_balance + fork_transfer_amount,
        "Baltathar should receive 1 ether on forked node"
    );

    // Step 7: Verify the source node was NOT affected by the forked node's transaction
    let source_alith_balance_final = source_node.get_balance(alith.address(), None).await;
    let source_baltathar_balance_final = source_node.get_balance(baltathar.address(), None).await;

    assert_eq!(
        source_alith_balance_final, source_alith_balance,
        "Source node Alith balance should remain unchanged"
    );
    assert_eq!(
        source_baltathar_balance_final, source_baltathar_balance,
        "Source node Baltathar balance should remain unchanged"
    );
}

/// Tests that forking creates a new chain starting from the latest finalized block of the source
/// chain
#[tokio::test(flavor = "multi_thread")]
async fn test_fork_from_latest_finalized_block() {
    const BLOCKS_TO_MINE: u32 = 5;
    // Step 1: Create the "source" node
    let source_config = AnvilNodeConfig::test_config();
    let source_substrate_config = SubstrateNodeConfig::new(&source_config);
    let mut source_node =
        TestNode::new(source_config.clone(), source_substrate_config).await.unwrap();

    let source_substrate_rpc_port = source_node.substrate_rpc_port();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let baltathar = Account::from(subxt_signer::eth::dev::baltathar());
    let alith_address = ReviveAddress::new(alith.address());
    let baltathar_address = ReviveAddress::new(baltathar.address());

    // Step 2: Mine several blocks on the source node to create history
    let transfer_amount = U256::from_str_radix("1000000000000000000", 10).unwrap(); // 1 ether

    // Mine blocks with transfers
    for i in 1..=BLOCKS_TO_MINE {
        let transaction = TransactionRequest::default()
            .value(transfer_amount)
            .from(Address::from(alith_address))
            .to(Address::from(baltathar_address));
        source_node.send_transaction(transaction).await.unwrap();
        unwrap_response::<()>(source_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap())
            .unwrap();
        source_node.wait_for_block_with_timeout(i, Duration::from_secs(1)).await.unwrap();
    }

    // Get the current block number from source (this will be the fork point)
    let source_best_block = source_node.best_block_number().await;
    let source_best_block_hash =
        source_node.eth_block_hash_by_number(source_best_block).await.unwrap();

    // Verify the source node has mined the correct number of blocks
    assert_eq!(source_best_block, BLOCKS_TO_MINE, "Source node should have mined 5 blocks");

    // Step 3: Create a forked node from the latest finalized block of the source chain
    let source_rpc_url = format!("http://127.0.0.1:{source_substrate_rpc_port}");

    let fork_config =
        AnvilNodeConfig::test_config().with_port(0).with_eth_rpc_url(Some(source_rpc_url));

    let fork_substrate_config = SubstrateNodeConfig::new(&fork_config);
    let mut fork_node = TestNode::new(fork_config.clone(), fork_substrate_config).await.unwrap();

    fork_node.wait_for_block_with_timeout(source_best_block, Duration::from_secs(1)).await.unwrap();

    // Step 4: Verify the forked node starts from the same block as source best block
    let fork_initial_block_number = fork_node.best_block_number().await;
    assert_eq!(
        fork_initial_block_number, source_best_block,
        "Forked node should start from source's latest finalized block"
    );

    // Step 5: Verify the genesis block hash of the fork matches the source
    let fork_genesis_hash =
        fork_node.eth_block_hash_by_number(fork_initial_block_number).await.unwrap();
    assert_eq!(
        fork_genesis_hash, source_best_block_hash,
        "Forked node's initial block hash should match source block hash"
    );

    // Step 6: Mine a new block on the forked node
    let fork_transfer_amount = U256::from_str_radix("2000000000000000000", 10).unwrap(); // 2 ether
    let fork_transaction = TransactionRequest::default()
        .value(fork_transfer_amount)
        .from(Address::from(alith_address))
        .to(Address::from(baltathar_address));
    fork_node.send_transaction(fork_transaction).await.unwrap();
    unwrap_response::<()>(fork_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    // Wait for the next block to be produced
    let expected_block = fork_initial_block_number + 1;
    fork_node.wait_for_block_with_timeout(expected_block, Duration::from_secs(1)).await.unwrap();

    // Step 7: Verify the forked node advanced by 1 block
    let fork_current_block = fork_node.best_block_number().await;
    assert_eq!(
        fork_current_block,
        fork_initial_block_number + 1,
        "Forked node should have advanced by 1 block after mining"
    );

    // Step 8: Verify that the source node is still at the same block (unchanged by fork)
    let source_current_block = source_node.best_block_number().await;
    assert_eq!(
        source_current_block, source_best_block,
        "Source node should remain at the same block"
    );

    // Step 9: Verify the block hashes diverged after the fork point
    let fork_new_block_hash = fork_node.eth_block_hash_by_number(fork_current_block).await.unwrap();

    // Mine one more block on source to create a divergence
    let source_transfer_amount = U256::from_str_radix("3000000000000000000", 10).unwrap(); // 3 ether
    let source_transaction = TransactionRequest::default()
        .value(source_transfer_amount)
        .from(Address::from(alith_address))
        .to(Address::from(baltathar_address));
    source_node.send_transaction(source_transaction).await.unwrap();
    unwrap_response::<()>(source_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap())
        .unwrap();

    let source_new_block = source_node.best_block_number().await;
    let source_new_block_hash =
        source_node.eth_block_hash_by_number(source_new_block).await.unwrap();

    // Both chains should be at the same height now
    assert_eq!(source_new_block, fork_current_block, "Both chains should be at the same height");

    // But the hashes should be different (chains diverged)
    assert_ne!(
        fork_new_block_hash, source_new_block_hash,
        "Block hashes should be different (chains diverged after fork point)"
    );
}

/// Tests forking from a specific block number using fork_choice configuration
#[tokio::test(flavor = "multi_thread")]
async fn test_fork_from_specific_block_number() {
    // Step 1: Create the "source" node
    let source_config = AnvilNodeConfig::test_config();
    let source_substrate_config = SubstrateNodeConfig::new(&source_config);
    let mut source_node =
        TestNode::new(source_config.clone(), source_substrate_config).await.unwrap();

    let source_substrate_rpc_port = source_node.substrate_rpc_port();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let baltathar = Account::from(subxt_signer::eth::dev::baltathar());
    let alith_address = ReviveAddress::new(alith.address());
    let baltathar_address = ReviveAddress::new(baltathar.address());

    // Step 2: Mine 5 blocks on the source node
    let transfer_amount = U256::from_str_radix("1000000000000000000", 10).unwrap(); // 1 ether

    for _ in 0..5 {
        let transaction = TransactionRequest::default()
            .value(transfer_amount)
            .from(Address::from(alith_address))
            .to(Address::from(baltathar_address));
        source_node.send_transaction(transaction).await.unwrap();
        unwrap_response::<()>(source_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap())
            .unwrap();
    }

    // Verify source node is at block 5
    let source_block_5 = source_node.best_block_number().await;
    assert_eq!(source_block_5, 5, "Source node should be at block 5");

    // Get balances at block 3
    let baltathar_balance_at_block_3 =
        source_node.get_balance(baltathar.address(), Some(3.into())).await;
    let alith_balance_at_block_3 = source_node.get_balance(alith.address(), Some(3.into())).await;

    // Step 3: Create a forked node from block 3
    let source_rpc_url = format!("http://127.0.0.1:{source_substrate_rpc_port}");

    let fork_config = AnvilNodeConfig::test_config()
        .with_port(0)
        .with_eth_rpc_url(Some(source_rpc_url))
        .with_fork_block_number(Some(3u64));

    let fork_substrate_config = SubstrateNodeConfig::new(&fork_config);
    let mut fork_node = TestNode::new(fork_config.clone(), fork_substrate_config).await.unwrap();

    // Step 4: Verify the forked node starts from block 3
    let fork_initial_block = fork_node.best_block_number().await;
    assert_eq!(fork_initial_block, 3, "Forked node should start from block 3");

    // Step 5: Verify balances match block 3 state
    let fork_baltathar_balance = fork_node.get_balance(baltathar.address(), None).await;
    let fork_alith_balance = fork_node.get_balance(alith.address(), None).await;

    assert_eq!(
        fork_baltathar_balance, baltathar_balance_at_block_3,
        "Baltathar balance should match block 3 state"
    );
    assert_eq!(
        fork_alith_balance, alith_balance_at_block_3,
        "Alith balance should match block 3 state"
    );

    // Step 6: Mine a new block on the forked node
    let fork_transaction = TransactionRequest::default()
        .value(transfer_amount)
        .from(Address::from(alith_address))
        .to(Address::from(baltathar_address));
    fork_node.send_transaction(fork_transaction).await.unwrap();
    unwrap_response::<()>(fork_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    // Step 7: Verify fork advanced to block 4
    let fork_new_block = fork_node.best_block_number().await;
    assert_eq!(fork_new_block, 4, "Forked node should be at block 4");

    // Step 8: Verify the balances changed on fork but not on source
    let fork_baltathar_balance_after = fork_node.get_balance(baltathar.address(), None).await;
    assert_eq!(
        fork_baltathar_balance_after,
        fork_baltathar_balance + transfer_amount,
        "Baltathar balance should have increased on fork"
    );

    // Source should still be at block 5
    let source_final_block = source_node.best_block_number().await;
    assert_eq!(source_final_block, 5, "Source node should still be at block 5");
}

/// Tests forking from a negative block number (offset from latest)
#[tokio::test(flavor = "multi_thread")]
async fn test_fork_from_negative_block_number() {
    // Step 1: Create the "source" node
    let source_config = AnvilNodeConfig::test_config();
    let source_substrate_config = SubstrateNodeConfig::new(&source_config);
    let mut source_node =
        TestNode::new(source_config.clone(), source_substrate_config).await.unwrap();

    let source_substrate_rpc_port = source_node.substrate_rpc_port();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let baltathar = Account::from(subxt_signer::eth::dev::baltathar());
    let alith_address = ReviveAddress::new(alith.address());
    let baltathar_address = ReviveAddress::new(baltathar.address());

    // Step 2: Mine 5 blocks on the source node
    let transfer_amount = U256::from_str_radix("1000000000000000000", 10).unwrap(); // 1 ether

    for _ in 0..5 {
        let transaction = TransactionRequest::default()
            .value(transfer_amount)
            .from(Address::from(alith_address))
            .to(Address::from(baltathar_address));
        source_node.send_transaction(transaction).await.unwrap();
        unwrap_response::<()>(source_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap())
            .unwrap();
    }

    // Verify source node is at block 5
    let source_block_5 = source_node.best_block_number().await;
    assert_eq!(source_block_5, 5, "Source node should be at block 5");

    // Get balances at block 3 (which is 5 - 2 = block 3)
    let baltathar_balance_at_block_3 =
        source_node.get_balance(baltathar.address(), Some(3.into())).await;
    let alith_balance_at_block_3 = source_node.get_balance(alith.address(), Some(3.into())).await;

    // Step 3: Create a forked node from block -2
    let source_rpc_url = format!("http://127.0.0.1:{source_substrate_rpc_port}");

    let fork_config = AnvilNodeConfig::test_config()
        .with_port(0)
        .with_eth_rpc_url(Some(source_rpc_url))
        .with_fork_choice(Some(ForkChoice::Block(-2)));

    let fork_substrate_config = SubstrateNodeConfig::new(&fork_config);
    let mut fork_node = TestNode::new(fork_config.clone(), fork_substrate_config).await.unwrap();

    // Step 4: Verify the forked node starts from block 3 (5 - 2)
    let fork_initial_block = fork_node.best_block_number().await;
    assert_eq!(
        fork_initial_block, 3,
        "Forked node should start from block 3 (latest 5 - offset 2)"
    );

    // Step 5: Verify balances match block 3 state
    let fork_baltathar_balance = fork_node.get_balance(baltathar.address(), None).await;
    let fork_alith_balance = fork_node.get_balance(alith.address(), None).await;

    assert_eq!(
        fork_baltathar_balance, baltathar_balance_at_block_3,
        "Baltathar balance should match block 3 state"
    );
    assert_eq!(
        fork_alith_balance, alith_balance_at_block_3,
        "Alith balance should match block 3 state"
    );

    // Step 6: Mine a new block on the forked node
    let fork_transaction = TransactionRequest::default()
        .value(transfer_amount)
        .from(Address::from(alith_address))
        .to(Address::from(baltathar_address));
    fork_node.send_transaction(fork_transaction).await.unwrap();
    unwrap_response::<()>(fork_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    // Step 7: Verify fork advanced to block 4
    let fork_new_block = fork_node.best_block_number().await;
    assert_eq!(fork_new_block, 4, "Forked node should be at block 4");
}

/// Tests that forking preserves contract state from source chain and that multiple contract
/// instances maintain independent storage
#[tokio::test(flavor = "multi_thread")]
async fn test_fork_with_contract_deployment() {
    // Step 1: Create the "source" node
    let source_config = AnvilNodeConfig::test_config().with_port(0);
    let source_substrate_config = SubstrateNodeConfig::new(&source_config);
    let mut source_node =
        TestNode::new(source_config.clone(), source_substrate_config).await.unwrap();

    let source_substrate_rpc_port = source_node.substrate_rpc_port();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_address = ReviveAddress::new(alith.address());

    // Step 2: Deploy 3 instances of SimpleStorage contract on source node
    let contract_code = get_contract_code("SimpleStorage");
    let contract1_tx = source_node.deploy_contract(&contract_code.init, alith.address()).await;
    unwrap_response::<()>(source_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap())
        .unwrap();

    let contract2_tx = source_node.deploy_contract(&contract_code.init, alith.address()).await;
    unwrap_response::<()>(source_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap())
        .unwrap();

    let contract3_tx = source_node.deploy_contract(&contract_code.init, alith.address()).await;
    unwrap_response::<()>(source_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap())
        .unwrap();

    let receipt1 = source_node.get_transaction_receipt(contract1_tx).await;
    let contract1_address = receipt1.contract_address.unwrap();
    let receipt2 = source_node.get_transaction_receipt(contract2_tx).await;
    let contract2_address = receipt2.contract_address.unwrap();
    let receipt3 = source_node.get_transaction_receipt(contract3_tx).await;
    let contract3_address = receipt3.contract_address.unwrap();

    // Verify all contracts are deployed successfully
    assert_eq!(receipt1.status, Some(polkadot_sdk::pallet_revive::U256::from(1)));
    assert_eq!(receipt2.status, Some(polkadot_sdk::pallet_revive::U256::from(1)));
    assert_eq!(receipt3.status, Some(polkadot_sdk::pallet_revive::U256::from(1)));

    // Step 3: Set different values for each contract (Contract 1: 100, Contract 2: 200, Contract 3:
    // 300)
    let set_value1 = SimpleStorage::setValueCall::new((U256::from(100),)).abi_encode();
    let call_tx1 = TransactionRequest::default()
        .from(Address::from(alith_address))
        .to(Address::from(ReviveAddress::new(contract1_address)))
        .input(TransactionInput::both(Bytes::from(set_value1)));
    source_node.send_transaction(call_tx1).await.unwrap();
    unwrap_response::<()>(source_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap())
        .unwrap();

    let set_value2 = SimpleStorage::setValueCall::new((U256::from(200),)).abi_encode();
    let call_tx2 = TransactionRequest::default()
        .from(Address::from(alith_address))
        .to(Address::from(ReviveAddress::new(contract2_address)))
        .input(TransactionInput::both(Bytes::from(set_value2)));
    source_node.send_transaction(call_tx2).await.unwrap();
    unwrap_response::<()>(source_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap())
        .unwrap();

    let set_value3 = SimpleStorage::setValueCall::new((U256::from(300),)).abi_encode();
    let call_tx3 = TransactionRequest::default()
        .from(Address::from(alith_address))
        .to(Address::from(ReviveAddress::new(contract3_address)))
        .input(TransactionInput::both(Bytes::from(set_value3)));
    source_node.send_transaction(call_tx3).await.unwrap();
    unwrap_response::<()>(source_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap())
        .unwrap();

    // Step 4: Verify each contract maintains its own independent storage on source node
    let value1 =
        simplestorage_get_value(&mut source_node, contract1_address, Address::from(alith_address))
            .await;
    let value2 =
        simplestorage_get_value(&mut source_node, contract2_address, Address::from(alith_address))
            .await;
    let value3 =
        simplestorage_get_value(&mut source_node, contract3_address, Address::from(alith_address))
            .await;

    assert_eq!(value1, U256::from(100), "Contract 1 should have value 100");
    assert_eq!(value2, U256::from(200), "Contract 2 should have value 200");
    assert_eq!(value3, U256::from(300), "Contract 3 should have value 300");

    // Update contract 2's value to 999 and verify others are unaffected
    let update_value2 = SimpleStorage::setValueCall::new((U256::from(999),)).abi_encode();
    let update_tx2 = TransactionRequest::default()
        .from(Address::from(alith_address))
        .to(Address::from(ReviveAddress::new(contract2_address)))
        .input(TransactionInput::both(Bytes::from(update_value2)));
    source_node.send_transaction(update_tx2).await.unwrap();
    unwrap_response::<()>(source_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap())
        .unwrap();

    // Verify only contract 2 changed on source node
    let value1_after =
        simplestorage_get_value(&mut source_node, contract1_address, Address::from(alith_address))
            .await;
    let value2_after =
        simplestorage_get_value(&mut source_node, contract2_address, Address::from(alith_address))
            .await;
    let value3_after =
        simplestorage_get_value(&mut source_node, contract3_address, Address::from(alith_address))
            .await;

    assert_eq!(value1_after, U256::from(100), "Contract 1 value should remain 100");
    assert_eq!(value2_after, U256::from(999), "Contract 2 value should be updated to 999");
    assert_eq!(value3_after, U256::from(300), "Contract 3 value should remain 300");

    // Step 5: Fork from the source node
    let source_rpc_url = format!("http://127.0.0.1:{source_substrate_rpc_port}");
    let fork_config =
        AnvilNodeConfig::test_config().with_port(0).with_eth_rpc_url(Some(source_rpc_url));

    let fork_substrate_config = SubstrateNodeConfig::new(&fork_config);
    let mut fork_node = TestNode::new(fork_config.clone(), fork_substrate_config).await.unwrap();

    // Step 6: Verify all contracts exist in forked node
    let fork_code1 = unwrap_response::<Bytes>(
        fork_node
            .eth_rpc(EthRequest::EthGetCodeAt(
                Address::from(ReviveAddress::new(contract1_address)),
                None,
            ))
            .await
            .unwrap(),
    )
    .unwrap();
    let fork_code2 = unwrap_response::<Bytes>(
        fork_node
            .eth_rpc(EthRequest::EthGetCodeAt(
                Address::from(ReviveAddress::new(contract2_address)),
                None,
            ))
            .await
            .unwrap(),
    )
    .unwrap();
    let fork_code3 = unwrap_response::<Bytes>(
        fork_node
            .eth_rpc(EthRequest::EthGetCodeAt(
                Address::from(ReviveAddress::new(contract3_address)),
                None,
            ))
            .await
            .unwrap(),
    )
    .unwrap();

    assert!(!fork_code1.is_empty(), "Contract 1 code should exist in fork");
    assert!(!fork_code2.is_empty(), "Contract 2 code should exist in fork");
    assert!(!fork_code3.is_empty(), "Contract 3 code should exist in fork");

    // Step 7: Verify all contract storage is preserved in fork with correct values
    let fork_value1 =
        simplestorage_get_value(&mut fork_node, contract1_address, Address::from(alith_address))
            .await;
    let fork_value2 =
        simplestorage_get_value(&mut fork_node, contract2_address, Address::from(alith_address))
            .await;
    let fork_value3 =
        simplestorage_get_value(&mut fork_node, contract3_address, Address::from(alith_address))
            .await;

    assert_eq!(
        fork_value1, value1_after,
        "Fork should have the same contract 1 state (100) as source"
    );
    assert_eq!(
        fork_value2, value2_after,
        "Fork should have the same contract 2 state (999) as source"
    );
    assert_eq!(
        fork_value3, value3_after,
        "Fork should have the same contract 3 state (300) as source"
    );
}

// =============================================================================
// Tests forking from external Westend Asset Hub zombienet
// These tests require a running zombienet instance at WESTEND_ASSET_HUB_URL
// =============================================================================

/// Helper to create a fork config pointing to Westend Asset Hub zombienet
fn westend_fork_config() -> AnvilNodeConfig {
    AnvilNodeConfig::test_config()
        .with_port(0)
        .with_eth_rpc_url(Some(WESTEND_ASSET_HUB_URL.to_string()))
}

/// Tests that we can fork from Westend Asset Hub and get balance of addresses
#[tokio::test(flavor = "multi_thread")]
async fn test_fork_eth_get_balance_from_westend() {
    let fork_config = westend_fork_config();
    let fork_substrate_config = SubstrateNodeConfig::new(&fork_config);
    let mut fork_node = TestNode::new(fork_config.clone(), fork_substrate_config).await.unwrap();

    // Get dev account
    let alith = Account::from(subxt_signer::eth::dev::alith());

    // Get balance from forked state
    let alith_balance = fork_node.get_balance(alith.address(), None).await;

    // Mine a block and verify we can still get balances
    unwrap_response::<()>(fork_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    let alith_balance_after_mine = fork_node.get_balance(alith.address(), None).await;

    // Balance should be the same (no transactions were made)
    assert_eq!(
        alith_balance, alith_balance_after_mine,
        "Balance should not change after mining empty block"
    );
}

/// Tests that we can get code of contracts from the forked Westend Asset Hub state
#[tokio::test(flavor = "multi_thread")]
async fn test_fork_eth_get_code_from_westend() {
    let fork_config = westend_fork_config();
    let fork_substrate_config = SubstrateNodeConfig::new(&fork_config);
    let mut fork_node = TestNode::new(fork_config.clone(), fork_substrate_config).await.unwrap();

    // Test getting code of random addresses (should be empty for EOAs)
    for _ in 0..5 {
        let random_addr = Address::random();
        let code = unwrap_response::<Bytes>(
            fork_node.eth_rpc(EthRequest::EthGetCodeAt(random_addr, None)).await.unwrap(),
        )
        .unwrap();
        assert!(code.is_empty(), "Random address should have no code");
    }

    // Mine a block and verify we can still get code
    unwrap_response::<()>(fork_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    // Set balance for alith to deploy contract (may not have balance in the forked chain)
    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_address = Address::from(ReviveAddress::new(alith.address()));
    let initial_balance = U256::from(1e20 as u128); // 100 ether
    unwrap_response::<()>(
        fork_node.eth_rpc(EthRequest::SetBalance(alith_address, initial_balance)).await.unwrap(),
    )
    .unwrap();

    // Deploy a contract on the fork
    let contract_code = get_contract_code("SimpleStorage");
    let tx_hash = fork_node.deploy_contract(&contract_code.init, alith.address()).await;
    unwrap_response::<()>(fork_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    let receipt = fork_node.get_transaction_receipt(tx_hash).await;
    let contract_address = receipt.contract_address.unwrap();

    // Get code of deployed contract
    let deployed_code = unwrap_response::<Bytes>(
        fork_node
            .eth_rpc(EthRequest::EthGetCodeAt(
                Address::from(ReviveAddress::new(contract_address)),
                None,
            ))
            .await
            .unwrap(),
    )
    .unwrap();

    assert!(!deployed_code.is_empty(), "Deployed contract should have code");
}

/// Tests that we can get nonce (transaction count) from the forked Westend Asset Hub state
#[tokio::test(flavor = "multi_thread")]
async fn test_fork_eth_get_nonce_from_westend() {
    let fork_config = westend_fork_config();
    let fork_substrate_config = SubstrateNodeConfig::new(&fork_config);
    let mut fork_node = TestNode::new(fork_config.clone(), fork_substrate_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_address = Address::from(ReviveAddress::new(alith.address()));

    // Set balance for alith (may not have balance in the forked chain)
    let initial_balance = U256::from(1e20 as u128); // 100 ether
    unwrap_response::<()>(
        fork_node.eth_rpc(EthRequest::SetBalance(alith_address, initial_balance)).await.unwrap(),
    )
    .unwrap();

    // Get initial nonce from forked state
    let initial_nonce = fork_node.get_nonce(alith_address).await;

    // Send a transaction to increase nonce
    let baltathar = Account::from(subxt_signer::eth::dev::baltathar());
    let baltathar_address = ReviveAddress::new(baltathar.address());
    let transfer_amount = U256::from(1e18 as u128); // 1 ether

    let transaction = TransactionRequest::default()
        .value(transfer_amount)
        .from(alith_address)
        .to(Address::from(baltathar_address));

    fork_node.send_transaction(transaction).await.unwrap();
    unwrap_response::<()>(fork_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    // Nonce should have increased by 1
    let nonce_after_tx = fork_node.get_nonce(alith_address).await;
    assert_eq!(
        nonce_after_tx,
        initial_nonce + U256::from(1),
        "Nonce should increase by 1 after sending transaction"
    );

    // Random addresses should have nonce 0
    for _ in 0..3 {
        let random_addr = Address::random();
        let nonce = unwrap_response::<U256>(
            fork_node.eth_rpc(EthRequest::EthGetTransactionCount(random_addr, None)).await.unwrap(),
        )
        .unwrap();
        assert_eq!(nonce, U256::ZERO, "Random address should have nonce 0");
    }
}

/// Tests state snapshotting and reverting on a forked Westend Asset Hub node
#[tokio::test(flavor = "multi_thread")]
async fn test_fork_state_snapshotting_from_westend() {
    let fork_config = westend_fork_config();
    let fork_substrate_config = SubstrateNodeConfig::new(&fork_config);
    let mut fork_node = TestNode::new(fork_config.clone(), fork_substrate_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let baltathar = Account::from(subxt_signer::eth::dev::baltathar());
    let alith_address = Address::from(ReviveAddress::new(alith.address()));
    let baltathar_address = Address::from(ReviveAddress::new(baltathar.address()));

    // Set initial balances for dev accounts (they may not have balance in the forked chain)
    let set_balance = U256::from(1e20 as u128); // 100 ether
    unwrap_response::<()>(
        fork_node.eth_rpc(EthRequest::SetBalance(alith_address, set_balance)).await.unwrap(),
    )
    .unwrap();
    unwrap_response::<()>(
        fork_node.eth_rpc(EthRequest::SetBalance(baltathar_address, set_balance)).await.unwrap(),
    )
    .unwrap();

    // Get initial state
    let initial_alith_balance = fork_node.get_balance(alith.address(), None).await;
    let initial_baltathar_balance = fork_node.get_balance(baltathar.address(), None).await;
    let initial_alith_nonce = fork_node.get_nonce(alith_address).await;

    // Create a snapshot
    let snapshot_id = U256::from_str_radix(
        unwrap_response::<String>(fork_node.eth_rpc(EthRequest::EvmSnapshot(())).await.unwrap())
            .unwrap()
            .trim_start_matches("0x"),
        16,
    )
    .unwrap();

    // Perform a transaction that modifies state
    let transfer_amount = U256::from(5e18 as u128); // 5 ether
    let transaction = TransactionRequest::default()
        .value(transfer_amount)
        .from(alith_address)
        .to(baltathar_address);

    fork_node.send_transaction(transaction).await.unwrap();
    unwrap_response::<()>(fork_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    // Verify state changed
    let alith_balance_after = fork_node.get_balance(alith.address(), None).await;
    let baltathar_balance_after = fork_node.get_balance(baltathar.address(), None).await;
    let nonce_after = fork_node.get_nonce(alith_address).await;

    assert!(
        alith_balance_after < initial_alith_balance,
        "Alith balance should decrease after transfer"
    );
    assert_eq!(
        baltathar_balance_after,
        initial_baltathar_balance + transfer_amount,
        "Baltathar should receive transfer amount"
    );
    assert_eq!(nonce_after, initial_alith_nonce + U256::from(1), "Nonce should increase");

    // Revert to snapshot
    let reverted = unwrap_response::<bool>(
        fork_node.eth_rpc(EthRequest::EvmRevert(snapshot_id)).await.unwrap(),
    )
    .unwrap();
    assert!(reverted, "Revert should succeed");

    // Wait for state to settle
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify state is back to snapshot point
    let alith_balance_reverted = fork_node.get_balance(alith.address(), None).await;
    let baltathar_balance_reverted = fork_node.get_balance(baltathar.address(), None).await;
    let nonce_reverted = fork_node.get_nonce(alith_address).await;

    assert_eq!(
        alith_balance_reverted, initial_alith_balance,
        "Alith balance should be restored after revert"
    );
    assert_eq!(
        baltathar_balance_reverted, initial_baltathar_balance,
        "Baltathar balance should be restored after revert"
    );
    assert_eq!(nonce_reverted, initial_alith_nonce, "Nonce should be restored after revert");
}

/// Tests sending transactions on a forked Westend Asset Hub node
#[tokio::test(flavor = "multi_thread")]
async fn test_fork_can_send_tx_from_westend() {
    let fork_config = westend_fork_config();
    let fork_substrate_config = SubstrateNodeConfig::new(&fork_config);
    let mut fork_node = TestNode::new(fork_config.clone(), fork_substrate_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let baltathar = Account::from(subxt_signer::eth::dev::baltathar());
    let alith_address = Address::from(ReviveAddress::new(alith.address()));
    let baltathar_address = Address::from(ReviveAddress::new(baltathar.address()));

    // Set initial balances for dev accounts (they may not have balance in the forked chain)
    let initial_balance = U256::from(1e20 as u128); // 100 ether
    unwrap_response::<()>(
        fork_node.eth_rpc(EthRequest::SetBalance(alith_address, initial_balance)).await.unwrap(),
    )
    .unwrap();
    unwrap_response::<()>(
        fork_node
            .eth_rpc(EthRequest::SetBalance(baltathar_address, initial_balance))
            .await
            .unwrap(),
    )
    .unwrap();

    // Get balances after setting
    let initial_alith_balance = fork_node.get_balance(alith.address(), None).await;
    let initial_baltathar_balance = fork_node.get_balance(baltathar.address(), None).await;

    assert_eq!(initial_alith_balance, initial_balance, "Alith balance should be set");
    assert_eq!(initial_baltathar_balance, initial_balance, "Baltathar balance should be set");

    // Send a simple ETH transfer
    let transfer_amount = U256::from(1e18 as u128); // 1 ether
    let transaction = TransactionRequest::default()
        .value(transfer_amount)
        .from(alith_address)
        .to(baltathar_address);

    let tx_hash = fork_node.send_transaction(transaction).await.unwrap();

    // Mine the transaction
    unwrap_response::<()>(fork_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    // Get receipt and verify transaction succeeded
    let receipt = fork_node.get_transaction_receipt(tx_hash).await;
    assert_eq!(
        receipt.status,
        Some(polkadot_sdk::pallet_revive::U256::from(1)),
        "Transaction should succeed"
    );

    // Verify balances changed
    let final_alith_balance = fork_node.get_balance(alith.address(), None).await;
    let final_baltathar_balance = fork_node.get_balance(baltathar.address(), None).await;

    assert!(
        final_alith_balance < initial_alith_balance,
        "Alith balance should decrease (transfer + gas)"
    );
    assert_eq!(
        final_baltathar_balance,
        initial_baltathar_balance + transfer_amount,
        "Baltathar should receive exact transfer amount"
    );

    // Send another transaction to verify chain continues working
    let second_transfer = U256::from(5e17 as u128); // 0.5 ether
    let transaction2 = TransactionRequest::default()
        .value(second_transfer)
        .from(baltathar_address)
        .to(alith_address);

    let tx_hash2 = fork_node.send_transaction(transaction2).await.unwrap();
    unwrap_response::<()>(fork_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    let receipt2 = fork_node.get_transaction_receipt(tx_hash2).await;
    assert_eq!(
        receipt2.status,
        Some(polkadot_sdk::pallet_revive::U256::from(1)),
        "Second transaction should succeed"
    );

    // Verify block number increased
    let _final_block = fork_node.best_block_number().await;
}

/// Tests that local state changes don't affect the remote fork state
#[tokio::test(flavor = "multi_thread")]
async fn test_fork_separate_states_from_westend() {
    let fork_config = westend_fork_config();
    let fork_substrate_config = SubstrateNodeConfig::new(&fork_config);
    let mut fork_node = TestNode::new(fork_config.clone(), fork_substrate_config).await.unwrap();

    let random_addr = Address::random();

    // Get initial balance (should be 0 for random address)
    let initial_balance =
        fork_node.get_balance(subxt::utils::H160::from(random_addr.0.0), None).await;
    assert_eq!(initial_balance, U256::ZERO, "Random address should have zero balance initially");

    // Set a new balance locally
    let new_balance = U256::from(1337u64);
    unwrap_response::<()>(
        fork_node.eth_rpc(EthRequest::SetBalance(random_addr, new_balance)).await.unwrap(),
    )
    .unwrap();

    // Verify local balance changed
    let local_balance =
        fork_node.get_balance(subxt::utils::H160::from(random_addr.0.0), None).await;
    assert_eq!(local_balance, new_balance, "Local balance should be updated");

    // The remote state should not be affected (we can't directly check this,
    // but we verify that local changes work independently)
}

/// Tests deploying a contract on a forked chain
#[tokio::test(flavor = "multi_thread")]
async fn test_fork_can_deploy_contract_from_westend() {
    let fork_config = westend_fork_config();
    let fork_substrate_config = SubstrateNodeConfig::new(&fork_config);
    let mut fork_node = TestNode::new(fork_config.clone(), fork_substrate_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let alith_address = Address::from(ReviveAddress::new(alith.address()));

    // Set balance for deployment
    let initial_balance = U256::from(1e20 as u128); // 100 ether
    unwrap_response::<()>(
        fork_node.eth_rpc(EthRequest::SetBalance(alith_address, initial_balance)).await.unwrap(),
    )
    .unwrap();

    // Deploy SimpleStorage contract
    let contract_code = get_contract_code("SimpleStorage");
    let tx_hash = fork_node.deploy_contract(&contract_code.init, alith.address()).await;
    unwrap_response::<()>(fork_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    let receipt = fork_node.get_transaction_receipt(tx_hash).await;
    assert_eq!(
        receipt.status,
        Some(polkadot_sdk::pallet_revive::U256::from(1)),
        "Contract deployment should succeed"
    );

    let contract_address = receipt.contract_address.expect("Contract address should exist");

    // Verify contract has code
    let code = unwrap_response::<Bytes>(
        fork_node
            .eth_rpc(EthRequest::EthGetCodeAt(
                Address::from(ReviveAddress::new(contract_address)),
                None,
            ))
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(!code.is_empty(), "Deployed contract should have code");

    // Deploy another contract to verify chain continues working
    let tx_hash2 = fork_node.deploy_contract(&contract_code.init, alith.address()).await;
    unwrap_response::<()>(fork_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    let receipt2 = fork_node.get_transaction_receipt(tx_hash2).await;
    assert_eq!(
        receipt2.status,
        Some(polkadot_sdk::pallet_revive::U256::from(1)),
        "Second contract deployment should succeed"
    );

    let contract_address2 =
        receipt2.contract_address.expect("Second contract address should exist");
    assert_ne!(contract_address, contract_address2, "Contract addresses should be different");
}

/// Tests impersonating an account on a forked chain
#[tokio::test(flavor = "multi_thread")]
async fn test_fork_impersonate_account_from_westend() {
    let fork_config = westend_fork_config();
    let fork_substrate_config = SubstrateNodeConfig::new(&fork_config);
    let mut fork_node = TestNode::new(fork_config.clone(), fork_substrate_config).await.unwrap();

    // Create a random address to impersonate
    let impersonated_addr = Address::random();
    let recipient_addr = Address::random();

    // Set balance for the impersonated account
    let balance = U256::from(1e20 as u128); // 100 ether
    unwrap_response::<()>(
        fork_node.eth_rpc(EthRequest::SetBalance(impersonated_addr, balance)).await.unwrap(),
    )
    .unwrap();

    // Enable impersonation
    unwrap_response::<()>(
        fork_node.eth_rpc(EthRequest::ImpersonateAccount(impersonated_addr)).await.unwrap(),
    )
    .unwrap();

    // Send transaction from impersonated account
    let transfer_amount = U256::from(1e18 as u128); // 1 ether
    let transaction = TransactionRequest::default()
        .value(transfer_amount)
        .from(impersonated_addr)
        .to(recipient_addr);

    let tx_hash = fork_node.send_transaction(transaction).await.unwrap();
    unwrap_response::<()>(fork_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    let receipt = fork_node.get_transaction_receipt(tx_hash).await;
    assert_eq!(
        receipt.status,
        Some(polkadot_sdk::pallet_revive::U256::from(1)),
        "Impersonated transaction should succeed"
    );

    // Verify recipient received the funds
    let recipient_balance =
        fork_node.get_balance(subxt::utils::H160::from(recipient_addr.0.0), None).await;
    assert_eq!(recipient_balance, transfer_amount, "Recipient should receive transfer");

    // Stop impersonation
    unwrap_response::<()>(
        fork_node.eth_rpc(EthRequest::StopImpersonatingAccount(impersonated_addr)).await.unwrap(),
    )
    .unwrap();
}

/// Tests setting balance and code on a forked chain
#[tokio::test(flavor = "multi_thread")]
async fn test_fork_set_balance_and_code_from_westend() {
    let fork_config = westend_fork_config();
    let fork_substrate_config = SubstrateNodeConfig::new(&fork_config);
    let mut fork_node = TestNode::new(fork_config.clone(), fork_substrate_config).await.unwrap();

    let test_addr = Address::random();

    // Initially should have no balance and no code
    let initial_balance =
        fork_node.get_balance(subxt::utils::H160::from(test_addr.0.0), None).await;
    let initial_code = unwrap_response::<Bytes>(
        fork_node.eth_rpc(EthRequest::EthGetCodeAt(test_addr, None)).await.unwrap(),
    )
    .unwrap();

    assert_eq!(initial_balance, U256::ZERO);
    assert!(initial_code.is_empty());

    // Set balance
    let new_balance = U256::from(12345678u64);
    unwrap_response::<()>(
        fork_node.eth_rpc(EthRequest::SetBalance(test_addr, new_balance)).await.unwrap(),
    )
    .unwrap();

    let updated_balance =
        fork_node.get_balance(subxt::utils::H160::from(test_addr.0.0), None).await;
    assert_eq!(updated_balance, new_balance, "Balance should be updated");

    // Set code
    let new_code = Bytes::from(vec![0x60, 0x60, 0x60, 0x40]); // Simple bytecode
    unwrap_response::<()>(
        fork_node.eth_rpc(EthRequest::SetCode(test_addr, new_code.clone())).await.unwrap(),
    )
    .unwrap();

    let updated_code = unwrap_response::<Bytes>(
        fork_node.eth_rpc(EthRequest::EthGetCodeAt(test_addr, None)).await.unwrap(),
    )
    .unwrap();
    assert_eq!(updated_code, new_code, "Code should be updated");

    // Set code to empty (clear code)
    unwrap_response::<()>(
        fork_node.eth_rpc(EthRequest::SetCode(test_addr, Bytes::new())).await.unwrap(),
    )
    .unwrap();

    let cleared_code = unwrap_response::<Bytes>(
        fork_node.eth_rpc(EthRequest::EthGetCodeAt(test_addr, None)).await.unwrap(),
    )
    .unwrap();
    assert!(cleared_code.is_empty(), "Code should be cleared");
}

/// Tests that block number increases correctly after mining on a fork
#[tokio::test(flavor = "multi_thread")]
async fn test_fork_block_number_after_mine_from_westend() {
    let fork_config = westend_fork_config();
    let fork_substrate_config = SubstrateNodeConfig::new(&fork_config);
    let mut fork_node = TestNode::new(fork_config.clone(), fork_substrate_config).await.unwrap();

    let initial_block = fork_node.best_block_number().await;

    // Mine a block
    unwrap_response::<()>(fork_node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();

    let block_after_mine1 = fork_node.best_block_number().await;
    assert_eq!(
        block_after_mine1,
        initial_block + 1,
        "Block number should increase by 1 after mining"
    );

    // Mine multiple blocks
    unwrap_response::<()>(
        fork_node.eth_rpc(EthRequest::Mine(Some(U256::from(5)), None)).await.unwrap(),
    )
    .unwrap();

    let block_after_mine5 = fork_node.best_block_number().await;
    assert_eq!(
        block_after_mine5,
        block_after_mine1 + 5,
        "Block number should increase by 5 after mining 5 blocks"
    );
}
