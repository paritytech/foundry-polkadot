use crate::{config::*, test_helpers::TEST_DATA_REVIVE};
use foundry_test_utils::Filter;
use revive_strategy::ReviveRuntimeMode;
use revm::primitives::hardfork::SpecId;
use rstest::rstest;

#[rstest]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_polkadot_skip(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner: forge::MultiContractRunner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new(".*", "PolkadotSkipTest", ".*/revive/PolkadotSkip.t.sol");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}
