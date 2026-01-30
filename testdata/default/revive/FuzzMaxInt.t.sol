// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "ds-test/test.sol";

contract FuzzMaxIntTest is DSTest {
    // Test that unsigned integers are limited to max_fuzz_int.
    // With max_fuzz_int = 255, all uint256 values should be <= 255.
    function testFuzzUint256Limited(uint256 val) public {
        assertTrue(val <= 255, "uint256 should be limited to max_fuzz_int (255)");
    }

    // Test that signed integers are limited to [-(max+1), max].
    // With max_fuzz_int = 255, int256 should be in range [-256, 255].
    function testFuzzInt256Limited(int256 val) public {
        assertTrue(val >= -256, "int256 should be >= -(max_fuzz_int+1) = -256");
        assertTrue(val <= 255, "int256 should be <= max_fuzz_int = 255");
    }
}

contract FuzzNoLimitTest is DSTest {
    // This test should fail without max_fuzz_int as fuzzer will generate values > 255.
    function testFuzzUint256Unlimited(uint256 val) public {
        assertTrue(val <= 255, "Expected to fail: fuzzer should generate values > 255");
    }

    // This test should fail without max_fuzz_int as fuzzer will generate values outside [-256, 255].
    function testFuzzInt256Unlimited(int256 val) public {
        bool inRange = val >= -256 && val <= 255;
        assertTrue(inRange, "Expected to fail: fuzzer should generate values outside [-256, 255]");
    }
}
