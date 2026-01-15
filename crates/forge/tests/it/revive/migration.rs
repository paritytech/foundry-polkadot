//! Forge tests for migration between EVM and Revive.

use crate::{config::*, test_helpers::TEST_DATA_REVIVE};
use foundry_test_utils::Filter;
use revive_strategy::ReviveRuntimeMode;
use revm::primitives::hardfork::SpecId;
use rstest::rstest;

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_revive_balance_migration(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new("testBalanceMigration", "EvmReviveMigrationTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_revive_nonce_migration(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new("testNonceMigration", "EvmReviveMigrationTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_revive_bytecode_migration(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter =
        Filter::new("testBytecodeMigrationToEvm", "EvmReviveMigrationTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_revive_bytecode_migration_to_revive(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter =
        Filter::new("testBytecodeMigrationToRevive", "EvmReviveMigrationTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_revive_precision_preservation(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new("testPrecisionPreservation", "EvmReviveMigrationTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_revive_timestamp_migration(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new("testTimestampMigration", "EvmReviveMigrationTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_revive_storage_migration(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new("testStorageMigration", "EvmReviveMigrationTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_revive_immutables_migration(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new("testImmutablesMigration", "EvmReviveMigrationTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_revive_callback_from_revive(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new("testCallbackFromRevive", "EvmReviveMigrationTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_revive_storage_migration_extra(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new(".*", "StoreTestExtra", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_explicit_evm_mode_switch(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new("testExplicitEvmModeSwitch", "EvmReviveMigrationTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_explicit_pvm_mode_switch(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new("testExplicitPvmModeSwitch", "EvmReviveMigrationTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_multiple_mode_switches(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new("testMultipleModeSwitches", "EvmReviveMigrationTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_contract_deployment_in_different_modes(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new(
        "testContractDeploymentInDifferentModes",
        "EvmReviveMigrationTest",
        ".*/revive/.*",
    );
    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}
