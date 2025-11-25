//! Individual cheatcode tests for pallet-revive
//! Each test runs a specific cheatcode file for easier debugging

use crate::{config::*, test_helpers::TEST_DATA_REVIVE};
use foundry_test_utils::Filter;
use revive_strategy::ReviveRuntimeMode;
use revm::primitives::hardfork::SpecId;
use rstest::rstest;

macro_rules! revive_cheat_test {
    ($test_name:ident, $file_pattern:expr) => {
        #[rstest]
        #[case::evm(ReviveRuntimeMode::Evm)]
        #[tokio::test(flavor = "multi_thread")]
        async fn $test_name(#[case] runtime_mode: ReviveRuntimeMode) {
            let filter = Filter::new(".*", ".*", &format!(".*/revive/{}.*", $file_pattern));

            let runner = TEST_DATA_REVIVE.runner_revive_with(runtime_mode, |config| {
                use foundry_config::{FsPermissions, fs_permissions::PathPermission};
                config.fs_permissions = FsPermissions::new(vec![PathPermission::read_write("./")]);
            });

            TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
        }
    };
}

revive_cheat_test!(test_nonce, "Nonce");
revive_cheat_test!(test_coinbase, "CoinBase");
revive_cheat_test!(test_warp, "Warp");
revive_cheat_test!(test_roll, "Roll");
// revive_cheat_test!(test_chainid, "ChainId");
// revive_cheat_test!(test_deal, "Deal");
// revive_cheat_test!(test_nonce, "Nonce");
// revive_cheat_test!(test_get_block_timestamp, "GetBlockTimestamp");
// revive_cheat_test!(test_get_block_number, "getBlockNumber");
// revive_cheat_test!(test_expect_emit, "ExpectEmit");
// revive_cheat_test!(test_expect_revert, "ExpectRevert");
// revive_cheat_test!(test_expect_call, "ExpectCall");
// // Fee test skipped - vm.fee() doesn't work correctly in revive mode
// revive_cheat_test!(test_fee, "Fee");
// // Prevrandao test skipped - vm.prevrandao() doesn't work correctly in revive mode
// revive_cheat_test!(test_prevrandao, "Prevrandao");
// // Load test skipped - vm.load() doesn't work correctly in revive mode
// revive_cheat_test!(test_load, "Load");
// revive_cheat_test!(test_access_list, "AccessList");
// revive_cheat_test!(test_addr, "Addr");
// revive_cheat_test!(test_arbitrary_storage, "ArbitraryStorage");
// revive_cheat_test!(test_assert, "Assert");
// revive_cheat_test!(test_assume, "Assume");
// revive_cheat_test!(test_assume_no_revert, "AssumeNoRevert");
// revive_cheat_test!(test_attach_blob, "AttachBlob");
// revive_cheat_test!(test_attach_delegation, "AttachDelegation");
// revive_cheat_test!(test_base64, "Base64");
// revive_cheat_test!(test_blob_base_fee, "BlobBaseFee");
// revive_cheat_test!(test_blobhashes, "Blobhashes");
// revive_cheat_test!(test_broadcast, "Broadcast");
// revive_cheat_test!(test_broadcast_raw_transaction, "BroadcastRawTransaction");
// revive_cheat_test!(test_clone_account, "CloneAccount");
// revive_cheat_test!(test_cool, "Cool");
// revive_cheat_test!(test_copy_storage, "CopyStorage");
// revive_cheat_test!(test_deploy_code, "DeployCode");
// revive_cheat_test!(test_derive, "Derive");
// revive_cheat_test!(test_ens_namehash, "EnsNamehash");
// revive_cheat_test!(test_env, "Env");
// revive_cheat_test!(test_etch, "Etch");
// revive_cheat_test!(test_expect_create, "ExpectCreate");
// revive_cheat_test!(test_ffi, "Ffi");
// revive_cheat_test!(test_fork, "Fork");
// revive_cheat_test!(test_fork2, "Fork2");
// revive_cheat_test!(test_fs, "Fs");
// revive_cheat_test!(test_get_artifact_path, "GetArtifactPath");
// revive_cheat_test!(test_get_chain, "GetChain");
// revive_cheat_test!(test_get_code, "GetCode");
// revive_cheat_test!(test_get_deployed_code, "GetDeployedCode");
// revive_cheat_test!(test_get_foundry_version, "GetFoundryVersion");
// revive_cheat_test!(test_get_label, "GetLabel");
// revive_cheat_test!(test_get_nonce, "GetNonce");
// revive_cheat_test!(test_get_raw_block_header, "GetRawBlockHeader");
// revive_cheat_test!(test_json, "Json");
// revive_cheat_test!(test_label, "Label");
// revive_cheat_test!(test_mapping, "Mapping");
// revive_cheat_test!(test_mem_safety, "MemSafety");
// revive_cheat_test!(test_parse, "Parse");
// revive_cheat_test!(test_project_root, "ProjectRoot");
// revive_cheat_test!(test_prompt, "Prompt");
// revive_cheat_test!(test_random_address, "RandomAddress");
// revive_cheat_test!(test_random_bytes, "RandomBytes");
// revive_cheat_test!(test_random_cheatcodes, "RandomCheatcodes");
// revive_cheat_test!(test_random_uint, "RandomUint");
// revive_cheat_test!(test_read_callers, "ReadCallers");
// revive_cheat_test!(test_record, "Record");
// revive_cheat_test!(test_record_account_accesses, "RecordAccountAccesses");
// revive_cheat_test!(test_record_debug_trace, "RecordDebugTrace");
// revive_cheat_test!(test_record_logs, "RecordLogs");
// revive_cheat_test!(test_remember, "Remember");
// revive_cheat_test!(test_reset_nonce, "ResetNonce");
// revive_cheat_test!(test_rpc_urls, "RpcUrls");
// revive_cheat_test!(test_seed, "Seed");
// revive_cheat_test!(test_set_blockhash, "SetBlockhash");
// revive_cheat_test!(test_set_nonce, "SetNonce");
// revive_cheat_test!(test_set_nonce_unsafe, "SetNonceUnsafe");
// revive_cheat_test!(test_setup, "Setup");
// revive_cheat_test!(test_shuffle, "Shuffle");
// revive_cheat_test!(test_sign, "Sign");
// revive_cheat_test!(test_sign_p256, "SignP256");
// revive_cheat_test!(test_skip, "Skip");
// revive_cheat_test!(test_sleep, "Sleep");
// revive_cheat_test!(test_sort, "Sort");
// revive_cheat_test!(test_state_snapshots, "StateSnapshots");
// revive_cheat_test!(test_storage_slot_state, "StorageSlotState");
// revive_cheat_test!(test_string_utils, "StringUtils");
// revive_cheat_test!(test_to_string, "ToString");
// revive_cheat_test!(test_toml, "Toml");
// revive_cheat_test!(test_travel, "Travel");
// revive_cheat_test!(test_try_ffi, "TryFfi");
// revive_cheat_test!(test_unix_time, "UnixTime");
// revive_cheat_test!(test_wallet, "Wallet");
// revive_cheat_test!(test_dump_state, "dumpState");
// revive_cheat_test!(test_load_allocs, "loadAllocs");
