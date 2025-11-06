use crate::{config::*, test_helpers::TEST_DATA_REVIVE};
use foundry_test_utils::Filter;
use revive_strategy::ReviveRuntimeMode;
use revm::primitives::hardfork::SpecId;
use rstest::rstest;

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_tx_gas_price_works(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner: forge::MultiContractRunner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new("testTxGasPriceWorks", "TxGasPrice", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_tx_gas_price_works_with_zero(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner: forge::MultiContractRunner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new("testTxGasPriceWorksWithZero", "TxGasPrice", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_tx_gas_price_works_with_large_value(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner: forge::MultiContractRunner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new("testTxGasPriceWorksWithLargeValue", "TxGasPrice", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_tx_gas_price_works_in_both_modes(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner: forge::MultiContractRunner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new("testTxGasPriceWorksInBothModes", "TxGasPrice", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_tx_gas_price_preserved_in_pvm_contract(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner: forge::MultiContractRunner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new("testTxGasPricePreservedInPvmContract", "TxGasPrice", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}
