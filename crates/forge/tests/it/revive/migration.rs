//! Forge tests for cheatcodes.

use crate::{config::*, test_helpers::TEST_DATA_REVIVE};
use foundry_test_utils::Filter;
use revm::primitives::hardfork::SpecId;

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_basic_migration() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testBasicEvmToPvmMigration", "EvmReviveMigrationTest", ".*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_balance_modification() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testBalanceModificationInPvm", "EvmReviveMigrationTest", ".*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_nonce_migration() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testNonceMigration", "EvmReviveMigrationTest", ".*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_multiple_accounts() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testMultipleAccountMigration", "EvmReviveMigrationTest", ".*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_precision_preservation() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testPrecisionPreservation", "EvmReviveMigrationTest", ".*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_round_trip() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testRoundTripMigration", "EvmReviveMigrationTest", ".*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_dust_conversion() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testDustConversionPrecision", "EvmReviveMigrationTest", ".*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_sub_wei_precision() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testSubWeiPrecision", "EvmReviveMigrationTest", ".*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_bytecode_migration() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testBytecodeMigration", "EvmReviveMigrationTest", ".*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_all_migration_tests() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new(".*", "EvmReviveMigrationTest", ".*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}
