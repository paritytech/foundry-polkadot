use crate::{config::*, test_helpers::TEST_DATA_REVIVE};
use foundry_test_utils::Filter;
use revive_strategy::ReviveRuntimeMode;
use revm::primitives::hardfork::SpecId;
use rstest::rstest;

#[rstest]
#[case::pvm_mode_with_existing_etched_pvm_code(
    "testEtchExistingContractPvmCode",
    ReviveRuntimeMode::Pvm
)]
#[case::evm_mode_with_existing_etched_pvm_code(
    "testEtchExistingContractPvmCode",
    ReviveRuntimeMode::Evm
)]
#[case::pvm_mode_with_existing_etched_evm_code(
    "testEtchExistingContractPvmCode",
    ReviveRuntimeMode::Pvm
)]
#[case::evm_mode_with_existing_etched_evm_code(
    "testEtchExistingContractEvmCode",
    ReviveRuntimeMode::Evm
)]
#[case::pvm_mode_with_any_etched_pvm_code("testEtchAnyContractPvmCode", ReviveRuntimeMode::Pvm)]
#[case::evm_mode_with_any_etched_pvm_code("testEtchAnyContractPvmCode", ReviveRuntimeMode::Evm)]
#[case::pvm_mode_with_any_etched_evm_code("testEtchAnyContractEvmCode", ReviveRuntimeMode::Pvm)]
#[case::evm_mode_with_any_etched_evm_code("testEtchAnyContractEvmCode", ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_etch(#[case] test_name: &str, #[case] runtime_mode: ReviveRuntimeMode) {
    let runner: forge::MultiContractRunner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new(test_name, "EtchTest", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}
