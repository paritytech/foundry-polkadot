use crate::{
    abi::SimpleStorage,
    utils::{
        TestNode, assert_with_tolerance, get_contract_code, multicall_get_coinbase, to_hex_string,
        unwrap_response,
    },
};
use alloy_genesis::{Genesis, GenesisAccount};
use alloy_primitives::{Address, B256, Bytes, U256};
use alloy_rpc_types::{BlockId, TransactionInput, TransactionRequest};
use alloy_sol_types::SolCall;
use anvil_core::eth::EthRequest;
use anvil_polkadot::{
    api_server::revive_conversions::ReviveAddress,
    config::{AnvilNodeConfig, SubstrateNodeConfig},
};
use polkadot_sdk::pallet_revive::{self, evm::Account};
use std::{collections::BTreeMap, path::PathBuf, time::Duration};
use subxt::utils::H160;

#[tokio::test(flavor = "multi_thread")]
async fn test_genesis_params() {
    let genesis_block_number: u32 = 1000;
    let anvil_genesis_timestamp: u64 = 42;
    let chain_id: u64 = 4242;
    let anvil_node_config = AnvilNodeConfig::test_config()
        .with_genesis_block_number(Some(genesis_block_number))
        .with_genesis_timestamp(Some(anvil_genesis_timestamp))
        .with_chain_id(Some(chain_id));
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config, substrate_node_config).await.unwrap();

    // Check that block number, timestamp, and chain id are set correctly at genesis
    assert_eq!(node.best_block_number().await, genesis_block_number);
    let genesis_hash = node.block_hash_by_number(genesis_block_number).await.unwrap();
    // Anvil genesis timestamp is in seconds, while Substrate timestamp is in milliseconds.
    let genesis_timestamp = anvil_genesis_timestamp.checked_mul(1000).unwrap();
    let actual_genesis_timestamp = node.get_decoded_timestamp(Some(genesis_hash)).await;
    assert_eq!(actual_genesis_timestamp, genesis_timestamp);
    let current_chain_id_hex =
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthChainId(())).await.unwrap()).unwrap();
    assert_eq!(current_chain_id_hex, to_hex_string(chain_id));

    // Manually mine two blocks and force the timestamp to be increasing with 1 second each time.
    unwrap_response::<()>(
        node.eth_rpc(EthRequest::Mine(Some(U256::from(2)), Some(U256::from(1)))).await.unwrap(),
    )
    .unwrap();

    let latest_block_number = node.best_block_number().await;
    assert_eq!(latest_block_number, genesis_block_number + 2);
    let hash2 = node.block_hash_by_number(genesis_block_number + 2).await.unwrap();
    let timestamp2 = node.get_decoded_timestamp(Some(hash2)).await;
    assert_with_tolerance(
        timestamp2.saturating_sub(genesis_timestamp),
        2000,
        500,
        "Timestamp is not increasing as expected from genesis.",
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_genesis_alloc() {
    // Create test EOA addresses
    let test_eoa_bytes_1 = [0x01; 20];
    let test_eoa_bytes_2 = [0x02; 20];
    let test_eoa_1 = H160::from_slice(&test_eoa_bytes_1);
    let test_eoa_2 = H160::from_slice(&test_eoa_bytes_2);
    let test_eoa_balance_1: u128 = 1_000_000_000_000_000_000; // 1 DOT
    let test_eoa_balance_2: u128 = 2_000_000_000_000_000_000; // 2 DOT

    // Create test contract address
    let test_contract_bytes = [0x03; 20];
    let test_contract_address = H160::from_slice(&test_contract_bytes);
    let test_contract_balance: u128 = 5_000_000_000_000_000_000; // 5 DOT

    // Get the SimpleStorage contract code
    let contract_code = get_contract_code("SimpleStorage");
    let runtime_bytecode = contract_code.runtime.clone().unwrap();

    // Set up initial storage for the contract
    let mut genesis_storage = BTreeMap::new();
    // Set storage slot 0 to value 511 (as in state_injector tests)
    genesis_storage.insert(B256::from(U256::from(0)), B256::from(U256::from(511)));

    // Create genesis alloc with both EOA and contract accounts
    let mut alloc = BTreeMap::new();

    // Add EOA accounts
    alloc.insert(
        Address::from(test_eoa_bytes_1),
        GenesisAccount {
            balance: U256::from(test_eoa_balance_1),
            nonce: None,
            code: None,
            storage: None,
            private_key: None,
        },
    );
    alloc.insert(
        Address::from(test_eoa_bytes_2),
        GenesisAccount {
            balance: U256::from(test_eoa_balance_2),
            nonce: Some(10),
            code: None,
            storage: None,
            private_key: None,
        },
    );

    // Add contract account
    alloc.insert(
        Address::from(test_contract_bytes),
        GenesisAccount {
            balance: U256::from(test_contract_balance),
            nonce: Some(1), // Contract accounts typically start with nonce 1
            code: Some(Bytes::from(runtime_bytecode.clone())),
            storage: Some(genesis_storage),
            private_key: None,
        },
    );

    let genesis = Genesis { alloc, ..Default::default() };

    // Create anvil node config with custom genesis
    let anvil_node_config = AnvilNodeConfig::test_config().with_genesis(Some(genesis));
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config, substrate_node_config).await.unwrap();

    // Test all account balances
    let eoa_balance_1 = node.get_balance(test_eoa_1, None).await;
    let eoa_balance_2 = node.get_balance(test_eoa_2, None).await;
    let contract_balance = node.get_balance(test_contract_address, None).await;

    assert_eq!(
        eoa_balance_1,
        U256::from(test_eoa_balance_1),
        "First EOA should have correct balance"
    );
    assert_eq!(
        eoa_balance_2,
        U256::from(test_eoa_balance_2),
        "Second EOA should have correct balance"
    );
    assert_eq!(
        contract_balance,
        U256::from(test_contract_balance),
        "Genesis contract account should have correct balance"
    );

    // Test all account nonces
    let eoa_nonce_1 = node.get_nonce(Address::from(test_eoa_bytes_1)).await;
    let eoa_nonce_2 = node.get_nonce(Address::from(test_eoa_bytes_2)).await;
    let contract_nonce = node.get_nonce(Address::from(test_contract_bytes)).await;

    assert_eq!(eoa_nonce_1, U256::from(0), "First EOA should have nonce 0");
    assert_eq!(eoa_nonce_2, U256::from(10), "Second EOA should have nonce 10");
    assert_eq!(contract_nonce, U256::from(1), "Genesis contract should have nonce 1");

    // Test all account code (EOA should be empty, contract should have code)
    let eoa_code_1 = unwrap_response::<Bytes>(
        node.eth_rpc(EthRequest::EthGetCodeAt(
            Address::from(test_eoa_bytes_1),
            Some(BlockId::number(0)),
        ))
        .await
        .unwrap(),
    )
    .unwrap();
    let eoa_code_2 = unwrap_response::<Bytes>(
        node.eth_rpc(EthRequest::EthGetCodeAt(
            Address::from(test_eoa_bytes_2),
            Some(BlockId::number(0)),
        ))
        .await
        .unwrap(),
    )
    .unwrap();
    let contract_code_result = unwrap_response::<Bytes>(
        node.eth_rpc(EthRequest::EthGetCodeAt(
            Address::from(test_contract_bytes),
            Some(BlockId::number(0)),
        ))
        .await
        .unwrap(),
    )
    .unwrap();

    assert!(eoa_code_1.is_empty(), "First EOA should have no code");
    assert!(eoa_code_2.is_empty(), "Second EOA should have no code");
    assert!(!contract_code_result.is_empty(), "Genesis contract should have code");
    assert_eq!(
        contract_code_result.len(),
        runtime_bytecode.len(),
        "Genesis contract code length should match"
    );
    assert_eq!(contract_code_result, runtime_bytecode, "Genesis contract code should match");

    // Test contract storage
    let stored_value = node.get_storage_at(U256::from(0), test_contract_address).await;
    assert_eq!(stored_value, 511, "Storage slot 0 of genesis contract should contain value 511");

    // Test contract functionality by calling getValue()
    let tx = TransactionRequest::default()
        .from(Address::from(test_eoa_bytes_1))
        .to(Address::from(test_contract_bytes))
        .input(TransactionInput::both(SimpleStorage::getValueCall.abi_encode().into()));

    let value = unwrap_response::<Bytes>(
        node.eth_rpc(EthRequest::EthCall(tx.into(), None, None, None)).await.unwrap(),
    )
    .unwrap();

    let value = SimpleStorage::getValueCall::abi_decode_returns(&value.0).unwrap();
    assert_eq!(value, U256::from(511), "Contract getValue() should return 511");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_coinbase_genesis() {
    let genesis_coinbase = Address::random();
    let mut anvil_node_config = AnvilNodeConfig::test_config();
    anvil_node_config = anvil_node_config
        .with_genesis(Some(Genesis { coinbase: genesis_coinbase, ..Default::default() }));
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::SetAutomine(true)).await.unwrap()).unwrap();

    // Deploy multicall contract
    let alith = Account::from(subxt_signer::eth::dev::alith());
    let contract_code = get_contract_code("Multicall");
    let tx_hash = node.deploy_contract(&contract_code.init, alith.address(), Some(1)).await;
    tokio::time::sleep(Duration::from_millis(400)).await;

    // Get contract address.
    let receipt = node.get_transaction_receipt(tx_hash).await;
    assert_eq!(receipt.status, Some(pallet_revive::U256::from(1)));
    let contract_address = Address::from(receipt.contract_address.unwrap().to_fixed_bytes());

    // Make a get coinbase contract call.
    let alith_addr = Address::from(alith.address().to_fixed_bytes());
    let coinbase = multicall_get_coinbase(&mut node, alith_addr, contract_address).await;
    assert_eq!(coinbase, genesis_coinbase);
    assert_eq!(
        unwrap_response::<Address>(node.eth_rpc(EthRequest::EthCoinbase(())).await.unwrap())
            .unwrap(),
        genesis_coinbase,
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_genesis_json() {
    // Load genesis.json file from test-data directory
    let genesis_json_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data").join("genesis.json");
    let genesis_file = std::fs::File::open(&genesis_json_path)
        .unwrap_or_else(|_| panic!("Failed to open genesis.json at {genesis_json_path:?}"));
    let genesis: Genesis = serde_json::from_reader(genesis_file)
        .unwrap_or_else(|e| panic!("Failed to parse genesis.json: {e}"));

    // Expected values from genesis.json
    let expected_chain_id = genesis.config.chain_id;
    let expected_timestamp = genesis.timestamp;
    let expected_block_number = genesis.number.unwrap_or_default();
    let expected_coinbase = genesis.coinbase;
    let alloc_accounts = genesis.alloc.iter();

    // Create node config with genesis from file
    let anvil_node_config = AnvilNodeConfig::test_config().with_genesis(Some(genesis.clone()));
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config, substrate_node_config).await.unwrap();

    // Test chain ID
    let chain_id_hex =
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthChainId(())).await.unwrap()).unwrap();
    assert_eq!(
        chain_id_hex,
        to_hex_string(expected_chain_id),
        "Chain ID should match the one in genesis.json"
    );

    // Test block number
    let genesis_block_number = node.best_block_number().await;
    assert_eq!(
        genesis_block_number as u64, expected_block_number,
        "Genesis block number should match the one in genesis.json"
    );

    // Test timestamp
    let genesis_hash = node.block_hash_by_number(genesis_block_number).await.unwrap();
    // Anvil genesis timestamp is in seconds, while Substrate timestamp is in milliseconds
    let expected_timestamp_ms = expected_timestamp.checked_mul(1000).unwrap();
    let actual_timestamp = node.get_decoded_timestamp(Some(genesis_hash)).await;
    assert_eq!(
        actual_timestamp, expected_timestamp_ms,
        "Genesis timestamp should match the one in genesis.json"
    );

    // Test coinbase
    let coinbase =
        unwrap_response::<Address>(node.eth_rpc(EthRequest::EthCoinbase(())).await.unwrap())
            .unwrap();
    assert_eq!(coinbase, expected_coinbase, "Coinbase should match the one in genesis.json");

    // Scan through all accounts in the genesis alloc and test their balances, nonces, codes, and
    // storage.
    for (&account_addr, account_info) in alloc_accounts {
        let account_balance_actual =
            node.get_balance(H160::from_slice(account_addr.as_slice()), None).await;
        let expected_balance = account_info.balance;
        assert_eq!(
            account_balance_actual, expected_balance,
            "Account balance should match the one in genesis.json"
        );
        let account_nonce_actual = node.get_nonce(account_addr).await;
        let expected_nonce = account_info.nonce.unwrap_or_default();
        assert_eq!(
            account_nonce_actual, expected_nonce,
            "Account nonce should match the one in genesis.json"
        );
        if account_info.code.is_none() {
            let code_actual = unwrap_response::<Bytes>(
                node.eth_rpc(EthRequest::EthGetCodeAt(
                    account_addr,
                    Some(BlockId::number(genesis_block_number.into())),
                ))
                .await
                .unwrap(),
            )
            .unwrap();
            assert!(
                code_actual.is_empty(),
                "Genesis account should have no code as in genesis.json"
            );
        } else {
            let code_actual = unwrap_response::<Bytes>(
                node.eth_rpc(EthRequest::EthGetCodeAt(
                    account_addr,
                    Some(BlockId::number(genesis_block_number.into())),
                ))
                .await
                .unwrap(),
            )
            .unwrap();
            assert!(
                !code_actual.is_empty(),
                "Genesis account should have non-empty code as in genesis.json"
            );
            assert_eq!(
                code_actual,
                account_info.code.clone().unwrap(),
                "Genesis account code should match the one in genesis.json"
            );
            for (storage_key, storage_value) in &account_info.storage.clone().unwrap_or_default() {
                let storage_value_actual = node
                    .get_storage_at(
                        U256::from_be_bytes(storage_key.0),
                        ReviveAddress::from(account_addr).inner(),
                    )
                    .await;
                let expected_storage_value = U256::from_be_bytes(storage_value.0);
                assert_eq!(
                    storage_value_actual, expected_storage_value,
                    "Genesis account storage value should match the one in genesis.json"
                );
            }
        }
    }
}
