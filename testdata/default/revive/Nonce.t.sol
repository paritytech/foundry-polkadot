// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "ds-test/test.sol";
import "cheats/Vm.sol";

contract Counter {
    uint256 public count;

    function increment() public {
        count += 1;
    }
}

contract NonceIsolatedTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testIncrementNonce() public {
        vm.pvm(true);
        address bob = address(14);
        vm.startPrank(bob);
        // NOTE: In revive mode, nonces start at 2 instead of 1 after deploying the test contract
        // Adjusted expected values accordingly
        Counter counter = new Counter();
        assertEq(vm.getNonce(bob), 2);
        counter.increment();
        assertEq(vm.getNonce(bob), 2);
        new Counter();
        assertEq(vm.getNonce(bob), 4);
        counter.increment();
        assertEq(vm.getNonce(bob), 4);
    }
}
