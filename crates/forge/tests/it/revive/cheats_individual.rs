//! Individual cheatcode tests for pallet-revive
//! Each test runs a specific cheatcode file for easier debugging

use crate::{config::*, test_helpers::TEST_DATA_REVIVE};
use foundry_test_utils::Filter;
use revive_strategy::ReviveRuntimeMode;
use revm::primitives::hardfork::SpecId;
use rstest::rstest;

macro_rules! revive_cheat_test_with_dir {
    ($test_name:ident, $file_pattern:expr, $dir:expr) => {
        #[rstest]
        #[case::evm(ReviveRuntimeMode::Evm)]
        #[tokio::test(flavor = "multi_thread")]
        async fn $test_name(#[case] runtime_mode: ReviveRuntimeMode) {
            let filter = Filter::new(".*", ".*", &format!(".*/{}/{}.t.sol$", $dir, $file_pattern));

            let runner = TEST_DATA_REVIVE.runner_revive_with(runtime_mode, |config| {
                use foundry_config::{FsPermissions, fs_permissions::PathPermission};
                config.fs_permissions = FsPermissions::new(vec![PathPermission::read_write("./")]);
            });

            TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
        }
    };
}

// Public macro for revive-specific tests (default)
macro_rules! revive_cheat_test {
    ($test_name:ident, $file_pattern:expr) => {
        revive_cheat_test_with_dir!($test_name, $file_pattern, "revive");
    };
}

// Public macro for original cheatcode tests
macro_rules! revive_cheat_test_original {
    ($test_name:ident, $file_pattern:expr) => {
        revive_cheat_test_with_dir!($test_name, $file_pattern, "cheats");
    };
}

revive_cheat_test!(test_warp, "Warp");
revive_cheat_test!(test_chainid, "ChainId");
revive_cheat_test!(test_deal, "Deal");
revive_cheat_test!(test_get_block_timestamp, "GetBlockTimestamp");
revive_cheat_test!(test_get_block_number, "getBlockNumber");
revive_cheat_test_original!(test_expect_emit, "ExpectEmit");
// TOFIX
revive_cheat_test_original!(test_expect_call, "ExpectCall");
// vm.fee() doesn't work correctly in revive mode
// revive_cheat_test!(test_fee, "Fee");
// vm.prevrandao() doesn't work correctly in revive mode
// revive_cheat_test!(test_prevrandao, "Prevrandao");
revive_cheat_test_original!(test_load, "Load");
// Not implemented
// revive_cheat_test_original!(test_access_list, "AccessList");
revive_cheat_test_original!(test_addr, "Addr");
// Not implemented vm.setArbitraryStorage
// revive_cheat_test_original!(test_arbitrary_storage, "ArbitraryStorage");
// SKIP it should not affect pallet-revive execution
// revive_cheat_test_original!(test_assert, "Assert");
// SKIP it should not affect pallet-revive execution
// revive_cheat_test!(test_assume, "Assume");
revive_cheat_test_original!(test_assume_no_revert, "AssumeNoRevert");
// vm.attachBlob vm.broadcast does not work
revive_cheat_test_original!(test_attach_blob, "AttachBlob");
// vm.attachDelegation vm.broadcast does not work
// revive_cheat_test_original!(test_attach_delegation, "AttachDelegation");
// SKIP it should not affect pallet-revive execution
// revive_cheat_test_original!(test_base64, "Base64");
// Compilation error
// revive_cheat_test!(test_blob_base_fee, "BlobBaseFee");
revive_cheat_test_original!(test_blobhashes, "Blobhashes");
// vm.broadcast does not work
// revive_cheat_test_original!(test_broadcast, "Broadcast");
//  vm.broadcastRawTransaction does not work
// revive_cheat_test_original!(test_broadcast_raw_transaction, "BroadcastRawTransaction");
// vm.cloneAccount maybly ised for forks - skip it
//revive_cheat_test_original!(test_clone_account, "CloneAccount");
// not supported in polkadot
// revive_cheat_test_original!(test_cool, "Cool");
// vm.copyStorage vm.setArbitraryStorage not implemented
//revive_cheat_test_original!(test_copy_storage, "CopyStorage");
//  vm.deployCode not implemented
revive_cheat_test_original!(test_deploy_code, "DeployCode");
// SKIP it should not affect pallet-revive execution
// revive_cheat_test_original!(test_derive, "Derive");
// SKIP it should not affect pallet-revive execution
// revive_cheat_test_original!(test_ens_namehash, "EnsNamehash");
// SKIP it should not affect pallet-revive execution
// revive_cheat_test_original!(test_env, "Env");
// EXTCODECOPY compilation issue
revive_cheat_test_original!(test_etch, "Etch");
// revive_cheat_test_original!(test_ffi, "Ffi");
// fork cheatcodes not supported
// revive_cheat_test_original!(test_fork, "Fork");
// fork cheatcodes not supported
// revive_cheat_test_original!(test_fork2, "Fork2");
// revive_cheat_test_original!(test_fs, "Fs");
revive_cheat_test!(test_get_artifact_path, "GetArtifactPath");
revive_cheat_test_original!(test_get_chain, "GetChain");
revive_cheat_test_original!(test_get_code, "GetCode");
revive_cheat_test!(test_get_deployed_code, "GetDeployedCode");
// SKIP it should not affect pallet-revive execution
// revive_cheat_test_original!(test_get_foundry_version, "GetFoundryVersion");
revive_cheat_test_original!(test_get_label, "GetLabel");
revive_cheat_test_original!(test_get_nonce, "GetNonce");
// Implement test to work without fork
revive_cheat_test!(test_get_raw_block_header, "GetRawBlockHeader");
// SKIP it should not affect pallet-revive execution
// revive_cheat_test_original!(test_json, "Json");
// revive_cheat_test_original!(test_label, "Label");
// TODO: check if it is needed
// revive_cheat_test_original!(test_mapping, "Mapping");
// TODO: check if it is needed
// revive_cheat_test_original!(test_mem_safety, "MemSafety");
// SKIP it should not affect pallet-revive execution
// revive_cheat_test_original!(test_parse, "Parse");
// SKIP it should not affect pallet-revive execution
// revive_cheat_test_original!(test_project_root, "ProjectRoot");
// TODO: check if it is needed
// revive_cheat_test_original!(test_prompt, "Prompt");
// SKIP it should not affect pallet-revive execution
// revive_cheat_test_original!(test_random_address, "RandomAddress");
// SKIP it should not affect pallet-revive execution
// revive_cheat_test_original!(test_random_bytes, "RandomBytes");
// SKIP it should not affect pallet-revive execution
// revive_cheat_test_original!(test_random_cheatcodes, "RandomCheatcodes");
// SKIP it should not affect pallet-revive execution
// revive_cheat_test_original!(test_random_uint, "RandomUint");
// TODO: check if it is needed
revive_cheat_test_original!(test_read_callers, "ReadCallers");
revive_cheat_test_original!(test_record_account_accesses, "RecordAccountAccesses");
revive_cheat_test_original!(test_record_debug_trace, "RecordDebugTrace");
revive_cheat_test_original!(test_record_logs, "RecordLogs");
// SKIP it should not affect pallet-revive execution
// revive_cheat_test_original!(test_remember, "Remember");
revive_cheat_test_original!(test_reset_nonce, "ResetNonce");
// SKIP it should not affect pallet-revive execution
// revive_cheat_test_original!(test_rpc_urls, "RpcUrls");
revive_cheat_test_original!(test_seed, "Seed");
revive_cheat_test_original!(test_set_nonce, "SetNonce");
revive_cheat_test_original!(test_set_nonce_unsafe, "SetNonceUnsafe");
revive_cheat_test_original!(test_setup, "Setup");
// revive_cheat_test_original!(test_shuffle, "Shuffle");
// SKIP it should not affect pallet-revive execution
// revive_cheat_test_original!(test_sign, "Sign");
// SKIP it should not affect pallet-revive execution
// revive_cheat_test_original!(test_sign_p256, "SignP256");
revive_cheat_test_original!(test_skip, "Skip");
// revive_cheat_test_original!(test_sleep, "Sleep");
// revive_cheat_test_original!(test_sort, "Sort");
revive_cheat_test_original!(test_state_snapshots, "StateSnapshots");
// Will not work at itt check Gas nd vm.cool
revive_cheat_test_original!(test_storage_slot_state, "StorageSlotState");
// SKIP it should not affect pallet-revive execution
// revive_cheat_test_original!(test_string_utils, "StringUtils");
// revive_cheat_test_original!(test_to_string, "ToString");
// revive_cheat_test_original!(test_toml, "Toml");
revive_cheat_test!(test_chainid2, "Travel");
// revive_cheat_test_original!(test_try_ffi, "TryFfi");
// revive_cheat_test_original!(test_unix_time, "UnixTime");
// revive_cheat_test_original!(test_wallet, "Wallet");
revive_cheat_test_original!(test_dump_state, "dumpState");
// revive_cheat_test_original!(test_load_allocs, "loadAllocs");
revive_cheat_test_original!(test_gas_metering, "GasMetering");
revive_cheat_test!(test_custom_nonce, "Nonce");
revive_cheat_test_original!(test_nonce, "Nonce");
revive_cheat_test_original!(test_expect_create, "ExpectCreate");
revive_cheat_test_original!(test_record_accesses, "RecordAccessesRevive");
revive_cheat_test_original!(test_record_rw, "Record");
revive_cheat_test_original!(test_expect_revert, "ExpectRevert");
revive_cheat_test_original!(test_custom_expect_call, "ExpectCallRevive");
revive_cheat_test!(test_coinbase, "CoinBase");
revive_cheat_test!(test_set_custom_blockhash, "SetBlockhash");
revive_cheat_test_original!(test_set_blockhash, "SetBlockhash");
revive_cheat_test!(test_roll, "Roll");
