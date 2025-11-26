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
        BlockChainId blockChainId = new BlockChainId();
        vm.chainId(10);
        assertEq(blockChainId.chainId(), 10, "chainId switch failed");
    }

    function testChainIdFuzzed(uint64 chainId) public {
        vm.pvm(true);
        BlockChainId blockChainId = new BlockChainId();
        vm.chainId(chainId);
        assertEq(blockChainId.chainId(), chainId, "chainId switch failed");
    }
}
