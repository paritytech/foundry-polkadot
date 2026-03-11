// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "ds-test/test.sol";
import "cheats/Vm.sol";

contract BlockHash {
    function getBlockhash(uint256 blockNumber) public view returns (bytes32) {
        return blockhash(blockNumber);
    }
}

contract SetBlockhash is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testSetBlockhash() public {
        vm.roll(10);
        BlockHash blockHash = new BlockHash();
        bytes32 expectedHash = 0x1234567890123456789012345678901234567890123456789012345678901234;
        vm.setBlockhash(9, expectedHash);
        assertEq(blockHash.getBlockhash(9), expectedHash);
    }
}
