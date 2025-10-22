use crate::{config::*, test_helpers::TEST_DATA_REVIVE};
use foundry_test_utils::Filter;
use revm::primitives::hardfork::SpecId;

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_prank0_after_prank1() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testPrank0AfterPrank1", "Prank", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_prank1_after_prank0() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testPrank1AfterPrank0", "Prank", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_prank_constructor_origin() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testPrankConstructorOrigin", "Prank", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_prank_constructor_sender() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testPrankConstructorSender", "Prank", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_prank_delegate_call_prank2() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testPrankDelegateCallPrank2", "Prank", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_prank_delegate_call_prank3() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testPrankDelegateCallPrank3", "Prank", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_prank_delegate_call_start_prank2() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testPrankDelegateCallStartPrank2", "Prank", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_prank_delegate_call_start_prank3() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testPrankDelegateCallStartPrank3", "Prank", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_prank_origin() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testPrankOrigin", "Prank", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_prank_sender() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testPrankSender", "Prank", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_prank_start_stop() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testPrankStartStop", "Prank", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_prank_start_stop_constructor() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testPrankStartStopConstructor", "Prank", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_revert_if_overwrite_unused_prank() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testRevertIfOverwriteUnusedPrank", "Prank", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_revert_if_overwrite_unused_prank_after_successful_prank() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new(
        "testRevertIfOverwriteUnusedPrankAfterSuccessfulPrank",
        "Prank",
        ".*/revive/.*",
    );

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_prank_delegate_call_to_eoa() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testRevertIfPrankDelegateCalltoEOA", "Prank", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_prank_start_prank0_after_prank1() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testStartPrank0AfterPrank1", "Prank", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_prank_start_prank0_after_start_prank1() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testStartPrank0AfterStartPrank1", "Prank", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_prank_start_prank1_after_start_prank0() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testStartPrank1AfterStartPrank0", "Prank", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_prank_tx_origin_in_nested_prank() {
    let runner = TEST_DATA_REVIVE.runner_revive();
    let filter = Filter::new("testTxOriginInNestedPrank", "Prank", ".*/revive/.*");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}

// TODO: Enable once etch support is enabled.

// #[tokio::test(flavor = "multi_thread")]
// async fn test_revive_delegate_prank() {
//     let runner = TEST_DATA_REVIVE.runner_revive();
//     let filter = Filter::new("testDelegatePrank", "Prank", ".*/revive/.*");

//     TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
// }
