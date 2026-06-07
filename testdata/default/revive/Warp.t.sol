// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "utils/DSTest.sol";
import "utils/Vm.sol";

contract BlockTimestamp {
    function timestamp() public view returns (uint256) {
        return block.timestamp;
    }
}

contract WarpTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testWarp() public {
        BlockTimestamp timeContract = new BlockTimestamp();
        vm.warp(10);
        assertEq(timeContract.timestamp(), 10, "warp failed");
    }

    function testWarpFuzzed(uint32 jump) public {
        BlockTimestamp timeContract = new BlockTimestamp();
        uint256 pre = timeContract.timestamp();
        vm.warp(pre + jump);
        assertEq(timeContract.timestamp(), pre + jump, "warp failed");
    }

    function testWarp2() public {
        BlockTimestamp timeContract = new BlockTimestamp();
        assertEq(timeContract.timestamp(), 1);
        vm.warp(100);
        assertEq(timeContract.timestamp(), 100);
    }
}
