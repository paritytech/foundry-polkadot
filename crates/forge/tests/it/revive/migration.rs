//! Forge tests for migration between EVM and Revive.

use crate::{config::*, test_helpers::TEST_DATA_REVIVE};
use foundry_test_utils::Filter;
use revive_strategy::ReviveRuntimeMode;
use revm::primitives::hardfork::SpecId;

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_balance_migration() {
    let runtime_mode = ReviveRuntimeMode::Pvm;
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new("testBalanceMigration", "EvmReviveMigrationTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_nonce_migration() {
    let runtime_mode = ReviveRuntimeMode::Pvm;
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new("testNonceMigration", "EvmReviveMigrationTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

// Enable it after new pallet-revive is being used
// #[tokio::test(flavor = "multi_thread")]
// async fn test_revive_precision_preservation() {
//     let runtime_mode = ReviveRuntimeMode::Pvm;
//     let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
//     let filter = Filter::new("testPrecisionPreservation", "EvmReviveMigrationTest",
// ".*/revive/.*");     TestConfig::with_filter(runner,
// filter).spec_id(SpecId::SHANGHAI).run().await; }

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_pvm_bytecode_migration() {
    let runtime_mode = ReviveRuntimeMode::Pvm;
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter =
        Filter::new("testBytecodeMigrationToEvm", "EvmReviveMigrationTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_evm_bytecode_migration() {
    let runtime_mode = ReviveRuntimeMode::Evm;
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter =
        Filter::new("testBytecodeMigrationToEvm", "EvmReviveMigrationTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_evm_to_revive_pvm_bytecode_migration() {
    let runtime_mode = ReviveRuntimeMode::Pvm;
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter =
        Filter::new("testBytecodeMigrationToRevive", "EvmReviveMigrationTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

// TODO: Enable when pallet-revive's EVM mode supports uploading EVM bytecode
// Currently getting CodeRejected errors even though pallet-revive uses REVM
// #[tokio::test(flavor = "multi_thread")]
// async fn test_evm_to_revive_evm_bytecode_migration() {
//     let runtime_mode = ReviveRuntimeMode::Evm;
//     let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
//     let filter =
//         Filter::new("testBytecodeMigrationToRevive", "EvmReviveMigrationTest", ".*/revive/.*");
//     TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
// }

// #[tokio::test(flavor = "multi_thread")]
// async fn test_revive_timestamp_migration() {
//     let runtime_mode = ReviveRuntimeMode::Pvm;
//     let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
//     let filter = Filter::new("testTimestampMigration", "EvmReviveMigrationTest", ".*/revive/.*");
//     TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
// }
