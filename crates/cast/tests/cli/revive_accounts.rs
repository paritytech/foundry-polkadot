use foundry_test_utils::{deploy_contract, forgetest_async, revive::PolkadotNode, util::OutputExt};

forgetest_async!(test_cast_balance_polkadot_localnode, |_prj, cmd| {
    if let Ok(_node) = PolkadotNode::start().await {
        let url = PolkadotNode::http_endpoint();
        let (account, _) = PolkadotNode::dev_accounts().next().expect("no dev accounts available");
        let account = account.to_string();

        cmd.cast_fuse().args(["balance", "--rpc-url", url, &account]).assert_success().stdout_eq(
            str![[r#"
18446744073709551615000000000000000000

"#]],
        );
    }
});

forgetest_async!(test_cast_nonce_polkadot_localnode, |_prj, cmd| {
    if let Ok(_node) = PolkadotNode::start().await {
        let url = PolkadotNode::http_endpoint();
        let (account, _) = PolkadotNode::dev_accounts().next().unwrap();
        let account = account.to_string();

        cmd.cast_fuse().args(["nonce", "--rpc-url", url, &account]).assert_success().stdout_eq(
            str![[r#"
0

"#]],
        );
    }
});

forgetest_async!(test_cast_code_polkadot_localnode, |_prj, cmd| {
    if let Ok(_node) = PolkadotNode::start().await {
        let (url, _deployer_pk, contract_address, _tx_hash) = deploy_contract!(cmd);

        cmd.cast_fuse()
            .args(["code", "--rpc-url", url, &contract_address])
            .assert_success()
            .stdout_eq(str![[r#"
0x5[..]

"#]]);
    }
});

forgetest_async!(test_cast_codesize_polkadot_localnode, |_prj, cmd| {
    if let Ok(_node) = PolkadotNode::start().await {
        let (url, _deployer_pk, contract_address, _tx_hash) = deploy_contract!(cmd);

        cmd.cast_fuse()
            .args(["codesize", "--rpc-url", url, &contract_address])
            .assert_success()
            .stdout_eq(str![[r#"
5501

"#]]);
    }
});

forgetest_async!(test_cast_storage_polkadot_localnode, |_prj, cmd| {
    if let Ok(_node) = PolkadotNode::start().await {
        let (url, _deployer_pk, contract_address, _tx_hash) = deploy_contract!(cmd);

        cmd.cast_fuse()
            .args(["storage", "--rpc-url", url, &contract_address, "0x0"])
            .assert_success()
            .stdout_eq(str![[r#"
0x000000000000000000000000000000000000000000000000000000000000002a

"#]]);
    }
});
