use crate::{config::*, test_helpers::TEST_DATA_REVIVE};
use foundry_test_utils::Filter;
use revive_strategy::ReviveRuntimeMode;
use revm::primitives::hardfork::SpecId;

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_deploy_time_linking() {
    let runner = TEST_DATA_REVIVE.runner_revive(ReviveRuntimeMode::Evm);
    let filter = Filter::new("testLibraryLinking", "LinkingTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_factory_deploy_time_linking() {
    let runner = TEST_DATA_REVIVE.runner_revive(ReviveRuntimeMode::Evm);
    let filter = Filter::new("testFactoryLibraryLinking", "FactoryLinkingTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_revive_deploy_time_linking_pvm() {
    let runner = TEST_DATA_REVIVE.runner_revive(ReviveRuntimeMode::Pvm);
    let filter = Filter::new("testLibraryLinking", "LinkingTest", ".*/revive/.*");
    TestConfig::with_filter(runner, filter).spec_id(SpecId::SHANGHAI).run().await;
}
