use crate::{config::*, test_helpers::TEST_DATA_REVIVE};
use foundry_test_utils::Filter;
use revm::primitives::hardfork::SpecId;

#[tokio::test(flavor = "multi_thread")]
async fn test_mock_calls() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testMockCalls", "MockCalls", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mock_calls_last_should_persist() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testMockCallsLastShouldPersist", "MockCalls", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mock_calls_with_value() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testMockCallsWithValue", "MockCalls", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}
