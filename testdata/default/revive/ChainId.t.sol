// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "ds-test/test.sol";
import "cheats/Vm.sol";

contract BlockChainId {
    function chainId() public view returns (uint256) {
        return block.chainid;
    }
}

contract ChainIdTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testChainId() public {
        vm.pvm(true);
        BlockChainId chainContract = new BlockChainId();
        uint256 newChainId = 99;
        vm.chainId(newChainId);
        assertEq(chainContract.chainId(), newChainId);
    }
}
