use foundry_test_utils::{forgetest_async, revive::PolkadotNode, util::OutputExt};

forgetest_async!(test_cast_chain_id, |_prj, cmd| {
    if let Ok(_node) = PolkadotNode::start().await {
        let rpc_url = PolkadotNode::http_endpoint();
        cmd.cast_fuse().args(["chain-id", "--rpc-url", rpc_url]).assert_success().stdout_eq(str![
            [r#"
420420420

"#]
        ]);
    }
});

forgetest_async!(test_cast_chain, |_prj, cmd| {
    if let Ok(_node) = PolkadotNode::start().await {
        let rpc_url = PolkadotNode::http_endpoint();
        cmd.cast_fuse().args(["chain", "--rpc-url", rpc_url]).assert_success().stdout_eq(str![[
            r#"
unknown

"#
        ]]);
    }
});

forgetest_async!(test_cast_client, |_prj, cmd| {
    if let Ok(_node) = PolkadotNode::start().await {
        let rpc_url = PolkadotNode::http_endpoint();
        let version = cmd
            .cast_fuse()
            .args(["client", "--rpc-url", rpc_url])
            .assert_success()
            .get_output()
            .stdout_lossy()
            .trim()
            .to_string();

        assert!(!version.is_empty(), "Expected non-empty client version");
    }
});
