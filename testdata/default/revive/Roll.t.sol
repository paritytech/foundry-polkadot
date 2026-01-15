// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.30;

import "ds-test/test.sol";
import "cheats/Vm.sol";

contract BlockNumber {
    function number() public view returns (uint256) {
        return block.number;
    }

    function hash(uint256 blockNum) public view returns (bytes32) {
        return blockhash(blockNum);
    }
}

contract RollTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testRoll() public {
        vm.polkadot(true);
        BlockNumber blockContract = new BlockNumber();
        vm.roll(10);
        assertEq(blockContract.number(), 10, "roll failed");
    }

    function testRollFuzzed(uint32 jump) public {
        vm.polkadot(true);
        BlockNumber blockContract = new BlockNumber();
        uint256 pre = blockContract.number();
        vm.roll(pre + jump);
        assertEq(blockContract.number(), pre + jump, "roll failed");
    }

    function testRollHash() public {
        vm.polkadot(true);
        BlockNumber blockContract = new BlockNumber();
        assertEq(blockContract.hash(blockContract.number()), 0x0, "initial block hash is incorrect");
        vm.roll(5);
        bytes32 hash = blockContract.hash(5);
        assertTrue(blockContract.hash(4) != 0x0, "new block hash is incorrect");
        vm.roll(10);
        assertTrue(blockContract.hash(5) != blockContract.hash(10), "block hash collision");
        vm.roll(5);
        assertEq(blockContract.hash(5), hash, "block 5 changed hash");
        // Make sure blockhashes match the EVM's blockhashes
        assertEq(blockContract.hash(5), blockhash(5));
        assertEq(blockContract.hash(4), blockhash(4));
    }
}
