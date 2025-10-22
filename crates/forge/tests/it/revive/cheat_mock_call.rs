use crate::{config::*, test_helpers::TEST_DATA_REVIVE};
use foundry_test_utils::Filter;
use revm::primitives::hardfork::SpecId;

#[tokio::test(flavor = "multi_thread")]
async fn test_mock_getters() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testMockGetters", "MockCall", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_clear_mock_reverted_calls() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testClearMockRevertedCalls", "MockCall", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mock_call_empty_account_revert() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testMockCallEmptyAccountRevert", "MockCall", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mock_call_revert() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testMockCallResetsMockCallRevert", "MockCall", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mock_call_reverts_partial_match() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testMockCallRevertPartialMatch", "MockCall", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mock_call_revert_resets_mock_call() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testMockCallRevertResetsMockCall", "MockCall", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mock_call_revert_with_call() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testMockCallRevertWithCall", "MockCall", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mock_call_revert_with_value() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testMockCallRevertWithValue", "MockCall", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mock_call_data_revert() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testMockCalldataRevert", "MockCall", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mock_getters_revert() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testMockGettersRevert", "MockCall", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mock_nested_revert() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testMockNestedRevert", "MockCall", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mock_revert_with_custom_error() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testMockRevertWithCustomError", "MockCall", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_clear_mocked_calls() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testClearMockedCalls", "MockCall", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mock_call_empty_account() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testMockCallEmptyAccount", "MockCall", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mock_call_multiple_partial_match() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testMockCallMultiplePartialMatch", "MockCall", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mock_call_with_value() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testMockCallWithValue", "MockCall", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mock_call_with_value_calldata_precedence() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testMockCallWithValueCalldataPrecedence", "MockCall", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mock_calldata() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testMockCalldata", "MockCall", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mock_nested() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testMockNested", "MockCall", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mock_nested_delegate() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testMockNestedDelegate", "MockCall", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mock_selector() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testMockSelector", "MockCall", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}
