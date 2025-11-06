use crate::{config::*, test_helpers::TEST_DATA_REVIVE};
use foundry_test_utils::Filter;
use revive_strategy::ReviveRuntimeMode;
use revm::primitives::hardfork::SpecId;
use rstest::rstest;

#[rstest]
#[case::pvm_mode_with_any_etched_evm_code(ReviveRuntimeMode::Pvm)]
#[case::evm_mode_with_any_etched_evm_code(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_etch(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner: forge::MultiContractRunner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new(".*", "EtchTest", ".*/revive/EtchTest.t.sol");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}
