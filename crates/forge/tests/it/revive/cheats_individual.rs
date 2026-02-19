//! Individual cheatcode tests for pallet-revive
//! Each test runs a specific cheatcode file for easier debugging

use crate::{config::*, test_helpers::TEST_DATA_REVIVE};
use foundry_test_utils::Filter;
use revive_strategy::ReviveRuntimeMode;
use revm::primitives::hardfork::SpecId;
use rstest::rstest;

macro_rules! revive_cheat_test_with_dir {
    ($test_name:ident, $file_pattern:expr, $dir:expr, $exclude_option:expr) => {
        #[rstest]
        #[case::evm(ReviveRuntimeMode::Evm)]
        #[tokio::test(flavor = "multi_thread")]
        async fn $test_name(#[case] runtime_mode: ReviveRuntimeMode) {
            let mut filter =
                Filter::new(".*", ".*", &format!(".*/{}/{}.t.sol$", $dir, $file_pattern));

            if let Some(exclude_pattern) = $exclude_option {
                filter = filter.exclude_tests(exclude_pattern);
            }

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
        revive_cheat_test_with_dir!($test_name, $file_pattern, "revive", None::<&str>);
    };
    ($test_name:ident, $file_pattern:expr,exclude: $exclude_pattern:expr) => {
        revive_cheat_test_with_dir!($test_name, $file_pattern, "revive", Some($exclude_pattern));
    };
}

// Public macro for original cheatcode tests
macro_rules! revive_cheat_test_original {
    ($test_name:ident, $file_pattern:expr) => {
        revive_cheat_test_with_dir!($test_name, $file_pattern, "cheats", None::<&str>);
    };
    ($test_name:ident, $file_pattern:expr,exclude: $exclude_pattern:expr) => {
        revive_cheat_test_with_dir!($test_name, $file_pattern, "cheats", Some($exclude_pattern));
    };
}

revive_cheat_test!(test_warp, "Warp");
revive_cheat_test!(test_chainid, "ChainId");
revive_cheat_test!(test_deal, "Deal");
revive_cheat_test!(test_get_block_timestamp, "GetBlockTimestamp");
revive_cheat_test!(test_get_block_number, "getBlockNumber");
revive_cheat_test_original!(test_expect_emit, "ExpectEmit");
// FAILS: Requires investigation and fix on the pallet-revive side
// revive_cheat_test_original!(test_expect_call, "ExpectCall");
// FAILS: vm.fee() doesn't work correctly in revive mode
// revive_cheat_test!(test_fee, "Fee");
// FAILS: vm.prevrandao() doesn't work correctly in revive mode
// revive_cheat_test!(test_prevrandao, "Prevrandao");
revive_cheat_test_original!(test_load, "Load", exclude: "testLoadNotAvailableOnPrecompiles");
// FAILS: AccessList Not implemented
// revive_cheat_test_original!(test_access_list, "AccessList");
revive_cheat_test_original!(test_addr, "Addr");
// FAILS: vm.setArbitraryStorage not implemented
// revive_cheat_test_original!(test_arbitrary_storage, "ArbitraryStorage");
revive_cheat_test_original!(test_assert, "Assert");
revive_cheat_test_original!(test_assume, "Assume");
revive_cheat_test_original!(test_assume_no_revert, "AssumeNoRevert");
// FAILS: vm.attachBlob: vm.broadcast does not work
revive_cheat_test_original!(test_attach_blob, "AttachBlob");
// FAILS: vm.attachDelegation: vm.broadcast does not work
// revive_cheat_test_original!(test_attach_delegation, "AttachDelegation");
revive_cheat_test_original!(test_base64, "Base64");
// FAILS: Compilation error
// revive_cheat_test!(test_blob_base_fee, "BlobBaseFee");
revive_cheat_test_original!(test_blobhashes, "Blobhashes");
// FAILS: vm.broadcast does not work
// revive_cheat_test_original!(test_broadcast, "Broadcast");
// FAILS: vm.broadcastRawTransaction does not work
// revive_cheat_test_original!(test_broadcast_raw_transaction, "BroadcastRawTransaction");
// vm.cloneAccount not implemented
// revive_cheat_test_original!(test_clone_account, "CloneAccount");
// FAILS: vm.cool not supported in Polkadot
// revive_cheat_test_original!(test_cool, "Cool");
// FAILS: vm.copyStorage, vm.setArbitraryStorage not implemented
// revive_cheat_test_original!(test_copy_storage, "CopyStorage");
revive_cheat_test_original!(test_deploy_code, "DeployCode");
revive_cheat_test_original!(test_derive, "Derive");
revive_cheat_test_original!(test_ens_namehash, "EnsNamehash");
revive_cheat_test_original!(test_env, "Env");
// FAILS: EXTCODECOPY compilation issue
// revive_cheat_test_original!(test_ffi, "Ffi");
// FAILS: Fork cheatcodes not supported
// revive_cheat_test_original!(test_fork, "Fork");
// FAILS: Fork cheatcodes not supported
// revive_cheat_test_original!(test_fork2, "Fork2");
revive_cheat_test_original!(test_fs, "Fs");
revive_cheat_test!(test_get_artifact_path, "GetArtifactPath");
revive_cheat_test_original!(test_get_chain, "GetChain");
revive_cheat_test_original!(test_get_code, "GetCode");
revive_cheat_test!(test_get_deployed_code, "GetDeployedCode");
revive_cheat_test_original!(test_get_foundry_version, "GetFoundryVersion");
revive_cheat_test_original!(test_get_label, "GetLabel");
revive_cheat_test_original!(test_get_nonce, "GetNonce");
// FAILS: test uses vm.fork
// revive_cheat_test_original!(test_get_raw_block_header, "GetRawBlockHeader");
revive_cheat_test_original!(test_json, "Json");
revive_cheat_test_original!(test_label, "Label");
// FAILS: Mapping recording cheatcodes (startMappingRecording, getMappingLength) don't work
// because SSTORE operations happen in pallet-revive, not REVM, so mapping slots aren't tracked
// revive_cheat_test_original!(test_mapping, "Mapping");
revive_cheat_test_original!(test_mem_safety, "MemSafety");
revive_cheat_test_original!(test_parse, "Parse");
revive_cheat_test_original!(test_project_root, "ProjectRoot");
revive_cheat_test_original!(test_prompt, "Prompt");
revive_cheat_test_original!(test_random_address, "RandomAddress");
revive_cheat_test_original!(test_random_bytes, "RandomBytes");
revive_cheat_test_original!(test_random_cheatcodes, "RandomCheatcodes");
revive_cheat_test_original!(test_random_uint, "RandomUint");
revive_cheat_test_original!(test_read_callers, "ReadCallers");
// FAILS: State diff recording (startStateDiffRecording) doesn't capture all account accesses
// (EXTCODESIZE, EXTCODEHASH, etc.) since these opcodes execute in pallet-revive, not REVM
// revive_cheat_test_original!(test_record_account_accesses, "RecordAccountAccesses");
// FAILS: Debug trace recording doesn't capture call depth correctly when execution happens in
// pallet-revive revive_cheat_test_original!(test_record_debug_trace, "RecordDebugTrace");
revive_cheat_test_original!(test_record_logs, "RecordLogs");
revive_cheat_test_original!(test_remember, "Remember");
revive_cheat_test_original!(test_reset_nonce, "ResetNonce");
revive_cheat_test_original!(test_rpc_urls, "RpcUrls");
revive_cheat_test_original!(test_seed, "Seed");
revive_cheat_test_original!(test_set_nonce, "SetNonce");
revive_cheat_test_original!(test_set_nonce_unsafe, "SetNonceUnsafe");
revive_cheat_test_original!(test_setup, "Setup");
revive_cheat_test_original!(test_shuffle, "Shuffle");
revive_cheat_test_original!(test_sign, "Sign");
revive_cheat_test_original!(test_sign_p256, "SignP256");
revive_cheat_test_original!(test_skip, "Skip");
revive_cheat_test_original!(test_sleep, "Sleep");
revive_cheat_test_original!(test_sort, "Sort");
revive_cheat_test_original!(test_state_snapshots, "StateSnapshots");
// FAILS: Uses gas checks and vm.cool which are not supported in pallet-revive
// revive_cheat_test_original!(test_storage_slot_state, "StorageSlotState");
revive_cheat_test_original!(test_string_utils, "StringUtils");
revive_cheat_test_original!(test_to_string, "ToString");
revive_cheat_test_original!(test_toml, "Toml");
revive_cheat_test!(test_chainid2, "Travel");
// FAILS: TryFfi.t.sol uses EXTCODECOPY which has compilation issues with pallet-revive
// revive_cheat_test_original!(test_try_ffi, "TryFfi");
revive_cheat_test_original!(test_unix_time, "UnixTime");
revive_cheat_test_original!(test_wallet, "Wallet");
// FAILS: In Polkadot mode, vm.dumpState dumps all persistent accounts (14) instead of just the
// explicitly created ones (expected 1). Test asserts account count which differs in pallet-revive.
// revive_cheat_test_original!(test_dump_state, "dumpState");
// FAILS: vm.loadAllocs() combined with vm.revertToState() causes panic in
// storage_rollback_transaction() because vm.loadAllocs creates external accounts that get migrated
// to pallet-revive, and the snapshot/revert mechanism doesn't properly handle rolling back
// cross-runtime state changes revive_cheat_test_original!(test_load_allocs, "loadAllocs");
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
revive_cheat_test_original!(test_etch, "Etch");
