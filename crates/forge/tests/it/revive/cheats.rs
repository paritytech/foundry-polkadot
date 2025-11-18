//! Forge tests for cheatcodes on pallet-revive.
//! A copy of the original cheats.rs tests

use crate::{config::*, test_helpers::TEST_DATA_REVIVE};
use foundry_test_utils::Filter;
use revive_strategy::ReviveRuntimeMode;
use revm::primitives::hardfork::SpecId;
use rstest::rstest;

/// Executes all cheat code tests on pallet-revive but not fork cheat codes or tests that
/// require isolation mode or specific seed.
#[rstest]
//#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_revive_cheats_local(#[case] runtime_mode: ReviveRuntimeMode) {
    let mut filter = Filter::new(".*", ".*", ".*/cheats/.*")
        .exclude_paths("Fork")
        .exclude_contracts("(Isolated|WithSeed)");

    // Exclude FFI tests on Windows because no `echo`, and file tests that expect certain file paths
    if cfg!(windows) {
        filter = filter.exclude_tests("(Ffi|File|Line|Root)");
    }

    if cfg!(feature = "isolate-by-default") {
        filter = filter.exclude_contracts("(LastCallGasDefaultTest|MockFunctionTest|WithSeed)");
    }

    let runner = TEST_DATA_REVIVE.runner_revive_with(runtime_mode, |config| {
        use foundry_config::{FsPermissions, fs_permissions::PathPermission};

        config.fs_permissions = FsPermissions::new(vec![PathPermission::read_write("./")]);
    });
    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

/// Executes subset of all cheat code tests in isolation mode on pallet-revive.
#[rstest]
//#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_revive_cheats_local_isolated(#[case] runtime_mode: ReviveRuntimeMode) {
    let filter = Filter::new(".*", ".*(Isolated)", ".*/cheats/.*");

    let runner = TEST_DATA_REVIVE.runner_revive_with(runtime_mode, |config| {
        config.isolate = true;
    });
    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

/// Executes subset of all cheat code tests using a specific seed on pallet-revive.
#[rstest]
//#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_revive_cheats_local_with_seed(#[case] runtime_mode: ReviveRuntimeMode) {
    let filter = Filter::new(".*", ".*(WithSeed)", ".*/cheats/.*");

    let runner = TEST_DATA_REVIVE.runner_revive_with(runtime_mode, |config| {
        config.fuzz.seed = Some(alloy_primitives::U256::from(100));
    });
    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}
