// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "utils/DSTest.sol";
import "utils/Vm.sol";

// NOTE: The vm.fee() cheatcode appears to not work correctly in revive mode
// It returns a fixed value (1000000) instead of the set value
// Skipping these tests until the cheatcode is fixed

contract BlockBaseFee {
    function baseFee() public view returns (uint256) {
        return block.basefee;
    }
}

contract FeeTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testFee() public {
        BlockBaseFee feeContract = new BlockBaseFee();
        vm.fee(10);
        assertEq(feeContract.baseFee(), 10, "fee failed");
    }

    function testFeeFuzzed(uint64 fee) public {
        BlockBaseFee feeContract = new BlockBaseFee();
        vm.fee(fee);
        assertEq(feeContract.baseFee(), fee, "fee failed");
    }
}
