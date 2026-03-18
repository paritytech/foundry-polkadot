use crate::{config::*, test_helpers::TEST_DATA_REVIVE};
use foundry_test_utils::Filter;
use revive_strategy::ReviveRuntimeMode;
use revm::primitives::hardfork::SpecId;
use rstest::rstest;

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_snapshot_state(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner: forge::MultiContractRunner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new(".*", "StateSnapshotTest", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_snapshot_constructor_contract(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner: forge::MultiContractRunner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new(".*", "SnapshotConstructorContractTest", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[rstest]
#[case::evm(ReviveRuntimeMode::Evm)]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_snapshot_across_mode_switch(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner: forge::MultiContractRunner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new(".*", "SnapshotAcrossModeSwitchTest", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}
