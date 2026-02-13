//! Tests for max_fuzz_int configuration limiting fuzz values.

use crate::test_helpers::TEST_DATA_REVIVE;
use alloy_primitives::U256;
use forge::result::{SuiteResult, TestStatus};
use foundry_test_utils::Filter;

/// Test that max_fuzz_int properly limits unsigned integers to [0, max].
/// With max_fuzz_int = 255, all uint256 values should be <= 255.
#[tokio::test(flavor = "multi_thread")]
async fn test_fuzz_max_int_uint_limited() {
    let max_value = U256::from(u8::MAX);

    let filter =
        Filter::new("testFuzzUint256Limited", "FuzzMaxIntTest", ".*/revive/FuzzMaxInt.t.sol");
    let mut runner = TEST_DATA_REVIVE.runner_with(|config| {
        config.fuzz.max_fuzz_int = Some(max_value);
        config.fuzz.runs = 1000;
    });
    let results = runner.test_collect(&filter).unwrap();

    for (_, SuiteResult { test_results, .. }) in results {
        for (test_name, result) in test_results {
            assert_eq!(
                result.status,
                TestStatus::Success,
                "Test {} should pass with max_fuzz_int limiting uint256 values to [0, 255].\nReason: {:?}",
                test_name,
                result.reason
            );
        }
    }
}

/// Test that max_fuzz_int properly limits signed integers to [-(max+1), max].
/// With max_fuzz_int = 255, signed integers should be in range [-256, 255].
#[tokio::test(flavor = "multi_thread")]
async fn test_fuzz_max_int_int_limited() {
    let max_value = U256::from(u8::MAX);

    let filter =
        Filter::new("testFuzzInt256Limited", "FuzzMaxIntTest", ".*/revive/FuzzMaxInt.t.sol");
    let mut runner = TEST_DATA_REVIVE.runner_with(|config| {
        config.fuzz.max_fuzz_int = Some(max_value);
        config.fuzz.runs = 1000;
    });
    let results = runner.test_collect(&filter).unwrap();

    for (_, SuiteResult { test_results, .. }) in results {
        for (test_name, result) in test_results {
            assert_eq!(
                result.status,
                TestStatus::Success,
                "Test {} should pass with max_fuzz_int limiting int256 values to [-256, 255].\nReason: {:?}",
                test_name,
                result.reason
            );
        }
    }
}

/// Test that without max_fuzz_int, uint256 values can exceed 255.
#[tokio::test(flavor = "multi_thread")]
async fn test_fuzz_no_limit_uint_exceeds() {
    let filter =
        Filter::new("testFuzzUint256Unlimited", "FuzzNoLimitTest", ".*/revive/FuzzMaxInt.t.sol");
    let mut runner = TEST_DATA_REVIVE.runner_with(|config| {
        config.fuzz.max_fuzz_int = None; // No limit
        config.fuzz.runs = 1000;
        config.fuzz.seed = Some(U256::from(42u32)); // Fixed seed for reproducibility
    });
    let results = runner.test_collect(&filter).unwrap();

    for (_, SuiteResult { test_results, .. }) in results {
        for (test_name, result) in test_results {
            assert_eq!(
                result.status,
                TestStatus::Failure,
                "Test {} should FAIL without max_fuzz_int as fuzzer generates values > 255.\nReason: {:?}",
                test_name,
                result.reason
            );
        }
    }
}

/// Test that without max_fuzz_int, int256 values can exceed [-256, 255] range.
#[tokio::test(flavor = "multi_thread")]
async fn test_fuzz_no_limit_int_exceeds() {
    let filter =
        Filter::new("testFuzzInt256Unlimited", "FuzzNoLimitTest", ".*/revive/FuzzMaxInt.t.sol");
    let mut runner = TEST_DATA_REVIVE.runner_with(|config| {
        config.fuzz.max_fuzz_int = None; // No limit
        config.fuzz.runs = 1000;
        config.fuzz.seed = Some(U256::from(42u32)); // Fixed seed for reproducibility
    });
    let results = runner.test_collect(&filter).unwrap();

    for (_, SuiteResult { test_results, .. }) in results {
        for (test_name, result) in test_results {
            assert_eq!(
                result.status,
                TestStatus::Failure,
                "Test {} should FAIL without max_fuzz_int as fuzzer generates values outside [-256, 255].\nReason: {:?}",
                test_name,
                result.reason
            );
        }
    }
}
