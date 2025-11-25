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
            let filter = Filter::new(".*", ".*", &format!(".*/cheats/{}.*", $file_pattern));

            let runner = TEST_DATA_REVIVE.runner_revive_with(runtime_mode, |config| {
                use foundry_config::{FsPermissions, fs_permissions::PathPermission};
                config.fs_permissions = FsPermissions::new(vec![PathPermission::read_write("./")]);
            });

            TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
        }
    };
}

revive_cheat_test!(test_nonce, "Nonce");
