use foundry_test_utils::{forgetest_async, revive::AnvilPolkadotNode};

forgetest_async!(test_cast_block_number_polkadot_localnode, |_prj, cmd| {
    if let Ok(_node) = AnvilPolkadotNode::start().await {
        let url = AnvilPolkadotNode::http_endpoint();
        cmd.cast_fuse().args(["block-number", "--rpc-url", url]).assert_success().stdout_eq(str![
            [r#"
0

"#]
        ]);
    }
});

forgetest_async!(test_cast_gas_price_polkadot_localnode, |_prj, cmd| {
    if let Ok(_node) = AnvilPolkadotNode::start().await {
        let url = AnvilPolkadotNode::http_endpoint();

        cmd.cast_fuse().args(["gas-price", "--rpc-url", url]).assert_success().stdout_eq(str![[
            r#"
1000000

"#
        ]]);
    }
});

forgetest_async!(test_cast_basefee_polkadot_localnode, |_prj, cmd| {
    if let Ok(_node) = AnvilPolkadotNode::start().await {
        let url = AnvilPolkadotNode::http_endpoint();

        cmd.cast_fuse().args(["basefee", "--rpc-url", url]).assert_success().stdout_eq(str![[r#"
1000000

"#]]);
    }
});

forgetest_async!(test_cast_block_polkadot_localnode, |_prj, cmd| {
    if let Ok(_node) = AnvilPolkadotNode::start().await {
        let url = AnvilPolkadotNode::http_endpoint();

        cmd.cast_fuse().args(["block", "latest", "--rpc-url", url]).assert_success().stdout_eq(
            str![[r#"


baseFeePerGas        [..]
difficulty           0
extraData            0x
gasLimit             [..]
gasUsed              0
hash                 0x[..]
logsBloom            0x[..]
miner                0x[..]
mixHash              0x[..]
nonce                0x[..]
number               [..]
parentHash           0x[..]
parentBeaconRoot     
transactionsRoot     0x[..]
receiptsRoot         0x[..]
sha3Uncles           0x[..]
size                 0
stateRoot            0x[..]
timestamp            [..]
withdrawalsRoot      [..]
totalDifficulty      
blobGasUsed          [..]
excessBlobGas        [..]
requestsHash         
transactions:        []

"#]],
        );
    }
});

forgetest_async!(test_cast_age_polkadot_localnode, |_prj, cmd| {
    if let Ok(_node) = AnvilPolkadotNode::start().await {
        let url = AnvilPolkadotNode::http_endpoint();

        cmd.cast_fuse().args(["age", "latest", "--rpc-url", url]).assert_success().stdout_eq(str![
            [r#"
[..]

"#]
        ]);
    }
});
