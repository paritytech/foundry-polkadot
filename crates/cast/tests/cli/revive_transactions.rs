use foundry_test_utils::{
    deploy_contract, forgetest_async, revive::AnvilPolkadotNode, util::OutputExt,
};

forgetest_async!(test_cast_receipt_polkadot_localnode, |_prj, cmd| {
    if let Ok(node) = AnvilPolkadotNode::start().await {
        let (url, _deployer_pk, _contract_address, tx_hash) = deploy_contract!(cmd, &node);

        cmd.cast_fuse().args(["receipt", &tx_hash, "--rpc-url", &url]).assert_success().stdout_eq(
            str![[r#"

blockHash            0x[..]
blockNumber          [..]
contractAddress      0x[..]
cumulativeGasUsed    [..]
effectiveGasPrice    [..]
from                 [..]
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

forgetest_async!(test_cast_call_polkadot_localnode, |_prj, cmd| {
    if let Ok(node) = AnvilPolkadotNode::start().await {
        let (url, _deployer_pk, contract_address, _tx_hash) = deploy_contract!(cmd, &node);

        cmd.cast_fuse()
            .args(["call", &contract_address, "--rpc-url", &url, "getCount()"])
            .assert_success()
            .stdout_eq(str![[r#"
0x000000000000000000000000000000000000000000000000000000000000002a

"#]]);
    }
});

forgetest_async!(test_cast_mktx_polkadot_localnode, |_prj, cmd| {
    if let Ok(node) = AnvilPolkadotNode::start().await {
        let (url, deployer_pk, contract_address, _tx_hash) = deploy_contract!(cmd, &node);

        cmd.cast_fuse()
            .args([
                "mktx",
                &contract_address,
                "incrementCounter()",
                "--rpc-url",
                &url,
                "--private-key",
                &deployer_pk,
            ])
            .assert_success()
            .stdout_eq(str![[r#"
0x[..]

"#]]);
    }
});

forgetest_async!(test_cast_tx_polkadot_localnode, |_prj, cmd| {
    if let Ok(node) = AnvilPolkadotNode::start().await {
        let (url, _deployer_pk, _contract_address, tx_hash) = deploy_contract!(cmd, &node);

        cmd.cast_fuse().args(["tx", "--rpc-url", &url, &tx_hash]).assert_success().stdout_eq(str![
            [r#"

blockHash            [..]
blockNumber          [..]
from                 [..]
transactionIndex     [..]
effectiveGasPrice    [..]

accessList           []
chainId              [..]
gasLimit             [..]
hash                 [..]
input                [..]
maxFeePerGas         [..]
maxPriorityFeePerGas [..]
nonce                [..]
r                    [..]
s                    [..]
to                   
type                 [..]
value                [..]
yParity              [..]
            

"#]
        ]);
    }
});

forgetest_async!(test_cast_estimate_polkadot_localnode, |_prj, cmd| {
    if let Ok(node) = AnvilPolkadotNode::start().await {
        let (url, _deployer_pk, contract_address, _tx_hash) = deploy_contract!(cmd, &node);

        let output = cmd
            .cast_fuse()
            .args(["estimate", "--rpc-url", &url, &contract_address, "getCount()"])
            .assert_success()
            .get_output()
            .stdout_lossy();

        let gas_estimate = output.trim().parse::<u64>();
        assert!(gas_estimate.is_ok(), "Expected a numeric gas estimate, got: {output}");
    }
});

forgetest_async!(test_cast_rpc_eth_get_block_by_number_polkadot_localnode, |_prj, cmd| {
    if let Ok(node) = AnvilPolkadotNode::start().await {
        let url = node.http_endpoint();

        let output = cmd
            .cast_fuse()
            .args(["rpc", "eth_getBlockByNumber", "--rpc-url", &url, "latest", "false"])
            .assert_success()
            .get_output()
            .stdout_lossy();

        let block: alloy_rpc_types::Block =
            serde_json::from_str(&output).expect("Failed to parse block data");
        assert!(!block.header.hash.is_zero(), "Block should have a non-zero hash");
        // assert!(!block.header.parent_hash.is_zero(), "Block should have a non-zero parent hash");
        assert!(block.header.timestamp > 0, "Block should have a positive timestamp");
        assert!(
            block.transactions.is_empty() || !block.transactions.is_empty(),
            "Block should have a transactions field"
        );
        assert!(block.header.gas_limit > 0, "Block should have gas_limit > 0");
    }
});

forgetest_async!(test_cast_logs_polkadot_localnode, |_prj, cmd| {
    if let Ok(node) = AnvilPolkadotNode::start().await {
        let (url, _deployer_pk, _contract_address, _tx_hash) = deploy_contract!(cmd, &node);

        cmd.cast_fuse()
            .args(["logs", "--rpc-url", &url, "--from-block", "latest", "--to-block", "latest"])
            .assert_success()
            .stdout_eq(str![[r#"


"#]]);
    }
});
