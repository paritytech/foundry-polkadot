// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "utils/DSTest.sol";
import "utils/Vm.sol";
import "utils/console.sol";

/**
 * @title Minimal reproducer for foundry-polkadot transient storage bug
 * @dev Demonstrates that transient storage is incorrectly cleared between external calls
 */
contract Counter {
    function increment() external {
        assembly {
            let value := tload(0)
            tstore(0, add(value, 1))
        }
    }

    function get() external view returns (uint256 value) {
        assembly {
            value := tload(0)
        }
    }
}

contract TransientStorage is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);
    Counter counter;

    function setUp() public {
        counter = new Counter();
    }

    /**
     * @dev This test FAILS in foundry-polkadot
     * Expected: counter.get() returns 1
     * Actual: counter.get() returns 0
     *
     * The bug: transient storage is cleared between the two external calls
     * According to EIP-1153, transient storage should persist for the entire transaction
     */
    function testTransientStoragePersistence() public {
        // Store value 1 in transient storage
        counter.increment();

        // This should return 1, but returns 0 in foundry-polkadot
        uint256 value = counter.get();

        assertEq(value, 1, "Transient storage should persist across external calls");
    }
}
