// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "ds-test/test.sol";

library MathLib {
    function add(uint256 a, uint256 b) public pure returns (uint256) {
        return a + b;
    }
    function mul(uint256 a, uint256 b) public pure returns (uint256) {
        return a * b;
    }
}

contract LibConsumer {
    function compute(uint256 a, uint256 b) public pure returns (uint256) {
        return MathLib.add(a, MathLib.mul(a, b));
    }
}

contract LinkingTest is DSTest {
    LibConsumer consumer;

    function setUp() public {
        consumer = new LibConsumer();
    }

    function testLibraryLinking() public {
        assertEq(consumer.compute(3, 4), 15); // 3 + (3*4) = 15
    }
}
