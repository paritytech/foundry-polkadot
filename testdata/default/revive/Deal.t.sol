// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "ds-test/test.sol";
import "cheats/Vm.sol";

contract BalanceChecker {
    function getBalance(address target) public view returns (uint256) {
        return target.balance;
    }
}

contract DealTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testDeal(uint128 amount) public {
        BalanceChecker checker = new BalanceChecker();
        address target = address(10);
        assertEq(checker.getBalance(target), 0, "initial balance incorrect");
            // // Give half the amount
        vm.deal(target, amount / 2);
        assertEq(checker.getBalance(target), amount / 2, "half balance is incorrect");
            // // Give the entire amount to check that deal is not additive
        vm.deal(target, amount);
        assertEq(checker.getBalance(target), amount, "deal did not overwrite balance");
    }
}