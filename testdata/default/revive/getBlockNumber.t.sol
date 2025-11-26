// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "ds-test/test.sol";
import "cheats/Vm.sol";

contract BlockNumber {
    function number() public view returns (uint256) {
        return block.number;
    }
}

contract GetBlockNumberTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testGetBlockNumber() public {
        vm.pvm(true);
        BlockNumber blockContract = new BlockNumber();
        uint256 height = vm.getBlockNumber();
        assertEq(height, blockContract.number(), "height should be equal to block.number");
    }

    function testGetBlockNumberWithRoll() public {
        vm.pvm(true);
        vm.roll(10);
        assertEq(vm.getBlockNumber(), 10, "could not get correct block height after roll");
    }

    function testGetBlockNumberWithRollFuzzed(uint32 jump) public {
        vm.pvm(true);
        uint256 pre = vm.getBlockNumber();
        vm.roll(pre + jump);
        assertEq(vm.getBlockNumber(), pre + jump, "could not get correct block height after roll");
    }
}
