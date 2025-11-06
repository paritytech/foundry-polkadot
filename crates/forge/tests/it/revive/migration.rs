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
// TODO: Add Evm test when pallet-revive will allow for Evm bytecode upload
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
// TODO: Add Evm test when pallet-revive will allow for Evm bytecode upload
#[tokio::test(flavor = "multi_thread")]
async fn test_revive_immutables_migration(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new("testImmutablesMigration", "EvmReviveMigrationTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}
