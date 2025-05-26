use foundry_test_utils::{casttest_serial, deploy_contract, revive::PolkadotNode, util::OutputExt};

casttest_serial!(test_cast_receipt, |_prj, cmd| {
    if let Ok(_node) = tokio::runtime::Runtime::new().unwrap().block_on(PolkadotNode::start()) {
        let (url, _deployer_pk, _contract_address, tx_hash) = deploy_contract!(cmd);

        cmd.cast_fuse().args(["receipt", &tx_hash, "--rpc-url", url]).assert_success().stdout_eq(
            str![[r#"

blockHash            0x[..]
blockNumber          [..]
contractAddress      0x[..]
cumulativeGasUsed    [..]
effectiveGasPrice    [..]
from                 0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac
gasUsed              [..]
logs                 []
logsBloom            0x[..]
root                 
status               1 (success)
transactionHash      [..]
transactionIndex     [..]
type                 [..]
blobGasPrice         
blobGasUsed          

"#]],
        );
    }
});

casttest_serial!(test_cast_call, |_prj, cmd| {
    if let Ok(_node) = tokio::runtime::Runtime::new().unwrap().block_on(PolkadotNode::start()) {
        let (url, _deployer_pk, contract_address, _tx_hash) = deploy_contract!(cmd);

        cmd.cast_fuse()
            .args(["call", &contract_address, "--rpc-url", url, "getCount()"])
            .assert_success()
            .stdout_eq(str![[r#"
0x000000000000000000000000000000000000000000000000000000000000002a

"#]]);
    }
});

casttest_serial!(test_cast_mktx, |_prj, cmd| {
    if let Ok(_node) = tokio::runtime::Runtime::new().unwrap().block_on(PolkadotNode::start()) {
        let (url, deployer_pk, contract_address, _tx_hash) = deploy_contract!(cmd);

        cmd.cast_fuse()
            .args([
                "mktx",
                &contract_address,
                "incrementCounter()",
                "--rpc-url",
                url,
                "--private-key",
                &deployer_pk,
            ])
            .assert_success()
            .stdout_eq(str![[r#"
0x[..]

"#]]);
    }
});

casttest_serial!(test_cast_tx, |_prj, cmd| {
    if let Ok(_node) = tokio::runtime::Runtime::new().unwrap().block_on(PolkadotNode::start()) {
        let (url, _deployer_pk, _contract_address, tx_hash) = deploy_contract!(cmd);

        cmd.cast_fuse().args(["tx", "--rpc-url", url, &tx_hash]).assert_success().stdout_eq(str![
            [r#"

blockHash            0x[..]
blockNumber          [..]
from                 0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac
transactionIndex     [..]
effectiveGasPrice    [..]

accessList           [..]
chainId              [..]
gasLimit             [..]
hash                 0x[..]
input                0x[..]
maxFeePerGas         [..]
maxPriorityFeePerGas [..]
nonce                [..]
r                    0x[..]
s                    0x[..]
to                   
type                 [..]
value                [..]
yParity              0
            

"#]
        ]);
    }
});

casttest_serial!(test_cast_estimate, |_prj, cmd| {
    if let Ok(_node) = tokio::runtime::Runtime::new().unwrap().block_on(PolkadotNode::start()) {
        let (url, _deployer_pk, contract_address, _tx_hash) = deploy_contract!(cmd);

        let output = cmd
            .cast_fuse()
            .args(["estimate", "--rpc-url", url, &contract_address, "getCount()"])
            .assert_success()
            .get_output()
            .stdout_lossy();

        let gas_estimate = output.trim().parse::<u64>();
        assert!(gas_estimate.is_ok(), "Expected a numeric gas estimate, got: {output}");
    }
});

casttest_serial!(test_cast_rpc_eth_get_block_by_number, |_prj, cmd| {
    if let Ok(_node) = tokio::runtime::Runtime::new().unwrap().block_on(PolkadotNode::start()) {
        let url = PolkadotNode::http_endpoint();

        let output = cmd
            .cast_fuse()
            .args(["rpc", "eth_getBlockByNumber", "--rpc-url", url, "latest", "false"])
            .assert_success()
            .get_output()
            .stdout_lossy();

        let block_data = serde_json::from_str::<serde_json::Value>(&output)
            .expect("Failed to parse JSON output");
        assert!(block_data.get("hash").is_some(), "Missing 'hash' field");
        assert!(block_data.get("number").is_some(), "Missing 'number' field");
        assert!(block_data.get("parentHash").is_some(), "Missing 'parentHash' field");
        assert!(block_data.get("timestamp").is_some(), "Missing 'timestamp' field");
        assert!(block_data.get("transactions").is_some(), "Missing 'transactions' field");
        assert!(block_data.get("gasUsed").is_some(), "Missing 'gasUsed' field");
        assert!(block_data.get("gasLimit").is_some(), "Missing 'gasLimit' field");
        let block_number_hex = block_data.get("number").unwrap().as_str().unwrap();
        let block_number = u64::from_str_radix(block_number_hex.trim_start_matches("0x"), 16);
        assert!(block_number.is_ok(), "Failed to parse block number as hex: {block_number_hex}");
        assert!(block_data.get("miner").is_some(), "Missing 'miner' field");
    }
});

casttest_serial!(test_cast_logs, |_prj, cmd| {
    if let Ok(_node) = tokio::runtime::Runtime::new().unwrap().block_on(PolkadotNode::start()) {
        let (url, _deployer_pk, _contract_address, _tx_hash) = deploy_contract!(cmd);

        cmd.cast_fuse()
            .args(["logs", "--rpc-url", url, "--from-block", "latest", "--to-block", "latest"])
            .assert_success()
            .stdout_eq(str![[r#"


"#]]);
    }
});
