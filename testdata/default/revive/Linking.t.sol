// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "utils/Test.sol";

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

contract FactoryUser {
    LibConsumer public consumer;

    constructor() {
        consumer = new LibConsumer();
    }

    function compute(uint256 a, uint256 b) public view returns (uint256) {
        return consumer.compute(a, b);
    }
}

contract FactoryLinkingTest is DSTest {
    FactoryUser factory;

    function setUp() public {
        factory = new FactoryUser();
    }

    function testFactoryLibraryLinking() public {
        assertEq(factory.compute(3, 4), 15); // 3 + (3*4) = 15
    }
}
