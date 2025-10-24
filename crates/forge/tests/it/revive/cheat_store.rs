use crate::{config::*, test_helpers::TEST_DATA_REVIVE};
use foundry_test_utils::Filter;
use revm::primitives::hardfork::SpecId;

#[tokio::test(flavor = "multi_thread")]
async fn test_store_works() {
    let runner: forge::MultiContractRunner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testStoreWorks", "Store", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_store_fuzzed() {
    let runner: forge::MultiContractRunner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testStoreFuzzed", "Store", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_store_not_available_on_precompiles() {
    let runner: forge::MultiContractRunner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testStoreNotAvailableOnPrecompiles", "Store", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}
