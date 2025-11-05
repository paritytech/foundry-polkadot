use crate::{config::*, test_helpers::TEST_DATA_REVIVE};
use foundry_test_utils::Filter;
use revive_strategy::ReviveRuntimeMode;
use revm::primitives::hardfork::SpecId;
use rstest::rstest;

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_mockx_function(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new("test_mockx_function", "MockFunction", ".*/revive/MockFunction.t.sol");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_mock_function_concrete_args(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new(
        "test_mock_function_concrete_args",
        "MockFunction",
        ".*/revive/MockFunction.t.sol",
    );

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_mock_function_all_args(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter =
        Filter::new("test_mock_function_all_args", "MockFunction", ".*/revive/MockFunction.t.sol");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}
