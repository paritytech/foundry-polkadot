// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "ds-test/test.sol";
import "cheats/Vm.sol";

contract BlockTimestamp {
    function timestamp() public view returns (uint256) {
        return block.timestamp;
    }
}

contract GetBlockTimestampTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testGetTimestamp() public {
        vm.pvm(true);
        BlockTimestamp blockTimestamp = new BlockTimestamp();
        assertEq(blockTimestamp.timestamp(), 1, "timestamp should be 1");
    }

    function testGetTimestampWithWarp() public {
        vm.pvm(true);
        BlockTimestamp blockTimestamp = new BlockTimestamp();
        assertEq(blockTimestamp.timestamp(), 1, "timestamp should be 1");
        vm.warp(10);
        assertEq(blockTimestamp.timestamp(), 10, "warp failed");
    }

    function testGetTimestampWithWarpFuzzed(uint32 jump) public {
        vm.pvm(true);
        BlockTimestamp blockTimestamp = new BlockTimestamp();
        uint256 pre = blockTimestamp.timestamp();
        vm.warp(pre + jump);
        assertEq(blockTimestamp.timestamp(), pre + jump, "warp failed");
    }
}
