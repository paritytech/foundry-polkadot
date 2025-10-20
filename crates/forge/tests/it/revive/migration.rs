//! Forge tests for migration between EVM and Revive.

use crate::{config::*, test_helpers::TEST_DATA_REVIVE};
use foundry_test_utils::Filter;
use revive_strategy::ReviveRuntimeMode;
use revm::primitives::hardfork::SpecId;

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_balance_migration_pvm() {
    let runtime_mode = ReviveRuntimeMode::Pvm;
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new("testBalanceMigration", "EvmReviveMigrationTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_balance_migration_evm() {
    let runtime_mode = ReviveRuntimeMode::Evm;
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new("testBalanceMigration", "EvmReviveMigrationTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_nonce_migration_pvm() {
    let runtime_mode = ReviveRuntimeMode::Pvm;
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new("testNonceMigration", "EvmReviveMigrationTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_nonce_migration_evm() {
    let runtime_mode = ReviveRuntimeMode::Evm;
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new("testNonceMigration", "EvmReviveMigrationTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

// Enable it after new pallet-revive is being used
// #[tokio::test(flavor = "multi_thread")]
// async fn test_revive_precision_preservation_pvm() {
//     let runtime_mode = ReviveRuntimeMode::Pvm;
//     let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
//     let filter = Filter::new("testPrecisionPreservation", "EvmReviveMigrationTest",
// ".*/revive/.*");     TestConfig::with_filter(runner,
// filter).spec_id(SpecId::SHANGHAI).run().await; }

// #[tokio::test(flavor = "multi_thread")]
// async fn test_revive_precision_preservation_evm() {
//     let runtime_mode = ReviveRuntimeMode::Evm;
//     let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
//     let filter = Filter::new("testPrecisionPreservation", "EvmReviveMigrationTest",
// ".*/revive/.*");     TestConfig::with_filter(runner,
// filter).spec_id(SpecId::SHANGHAI).run().await; }

// Bytecode migration tests - currently only PVM mode is fully supported
// EVM mode bytecode migration is a work in progress as pallet-revive's EVM mode
// has strict bytecode validation that may reject some EVM bytecode
#[tokio::test(flavor = "multi_thread")]
async fn test_revive_bytecode_migration() {
    let runtime_mode = ReviveRuntimeMode::Pvm;
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter =
        Filter::new("testBytecodeMigrationToEvm", "EvmReviveMigrationTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_evm_bytecode_migration() {
    let runtime_mode = ReviveRuntimeMode::Pvm;
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter =
        Filter::new("testBytecodeMigrationToRevive", "EvmReviveMigrationTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

// #[tokio::test(flavor = "multi_thread")]
// async fn test_revive_timestamp_migration() {
//     let runtime_mode = ReviveRuntimeMode::Pvm;
//     let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
//     let filter = Filter::new("testTimestampMigration", "EvmReviveMigrationTest", ".*/revive/.*");
//     TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
// }
