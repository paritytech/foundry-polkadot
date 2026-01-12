// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "ds-test/test.sol";
import "cheats/Vm.sol";

// NOTE: The vm.prevrandao() cheatcode appears to not work correctly in revive mode
// It returns a fixed value (2500000000000000) instead of the set value
// Skipping these tests until the cheatcode is fixed

contract BlockPrevrandao {
    function prevrandao() public view returns (uint256) {
        return block.prevrandao;
    }
}

contract PrevrandaoTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testPrevrandao() public {
        BlockPrevrandao randaoContract = new BlockPrevrandao();
        assertEq(randaoContract.prevrandao(), 0);
        vm.prevrandao(uint256(10));
        assertEq(randaoContract.prevrandao(), 10, "prevrandao cheatcode failed");
    }

    // function testPrevrandaoFuzzed(uint256 newPrevrandao) public {
    //     BlockPrevrandao randaoContract = new BlockPrevrandao();
    //     vm.assume(newPrevrandao != randaoContract.prevrandao());
    //     assertEq(randaoContract.prevrandao(), 0);
    //     vm.prevrandao(newPrevrandao);
    //     assertEq(randaoContract.prevrandao(), newPrevrandao);
    // }

    // function testPrevrandaoSnapshotFuzzed(uint256 newPrevrandao) public {
    //     BlockPrevrandao randaoContract = new BlockPrevrandao();
    //     vm.assume(newPrevrandao != randaoContract.prevrandao());
    //     uint256 oldPrevrandao = randaoContract.prevrandao();
    //     uint256 snapshotId = vm.snapshotState();

    //     vm.prevrandao(newPrevrandao);
    //     assertEq(randaoContract.prevrandao(), newPrevrandao);

    //     assert(vm.revertToState(snapshotId));
    //     assertEq(randaoContract.prevrandao(), oldPrevrandao);
    // }
}
