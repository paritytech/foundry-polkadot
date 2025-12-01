// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "ds-test/test.sol";
import "cheats/Vm.sol";

contract Storage {
    uint256 public slot0;
    uint256 public slot1;

    function setSlots(uint256 a, uint256 b) public {
        slot0 = a;
        slot1 = b;
    }

    function blockNumber() public returns (uint256) {
        return block.number;
    }

    function blockTimestamp() public returns (uint256) {
        return block.timestamp;
    }
}

contract StateSnapshotTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    Storage store;

    function setUp() public {
        store = new Storage();
        store.setSlots(10, 20);
    }

    function testStateSnapshot() public {
        uint256 snapshotId = vm.snapshotState();
        store.setSlots(300, 400);

        assertEq(store.slot0(), 300);
        assertEq(store.slot1(), 400);

        vm.revertToState(snapshotId);
        assertEq(store.slot0(), 10, "snapshot revert for slot 0 unsuccessful");
        assertEq(store.slot1(), 20, "snapshot revert for slot 1 unsuccessful");
    }

    function testStateSnapshot2() public {
        uint256 snapshotId = vm.snapshotState();
        store.setSlots(300, 400);

        assertEq(store.slot0(), 300);
        assertEq(store.slot1(), 400);

        uint256 snapshotId2 = vm.snapshotState();
        store.setSlots(500, 600);

        assertEq(store.slot0(), 500);
        assertEq(store.slot1(), 600);

        uint256 snapshotId3 = vm.snapshotState();
        store.setSlots(700, 800);

        assertEq(store.slot0(), 700);
        assertEq(store.slot1(), 800);

        uint256 snapshotId4 = vm.snapshotState();
        store.setSlots(800, 900);

        assertEq(store.slot0(), 800);
        assertEq(store.slot1(), 900);

        vm.revertToState(snapshotId4);
        assertEq(store.slot0(), 700, "snapshot revert for slot 0 unsuccessful");
        assertEq(store.slot1(), 800, "snapshot revert for slot 1 unsuccessful");

        vm.revertToState(snapshotId3);
        assertEq(store.slot0(), 500, "snapshot revert for slot 0 unsuccessful");
        assertEq(store.slot1(), 600, "snapshot revert for slot 1 unsuccessful");

        vm.revertToState(snapshotId2);
        assertEq(store.slot0(), 300, "snapshot revert for slot 0 unsuccessful");
        assertEq(store.slot1(), 400, "snapshot revert for slot 1 unsuccessful");

        vm.revertToState(snapshotId);
        assertEq(store.slot0(), 10, "snapshot revert for slot 0 unsuccessful");
        assertEq(store.slot1(), 20, "snapshot revert for slot 1 unsuccessful");
    }

    function testStateSnapshotRevertDelete() public {
        uint256 snapshotId = vm.snapshotState();
        store.setSlots(300, 400);

        assertEq(store.slot0(), 300);
        assertEq(store.slot1(), 400);

        vm.revertToStateAndDelete(snapshotId);
        assertEq(store.slot0(), 10, "snapshot revert for slot 0 unsuccessful");
        assertEq(store.slot1(), 20, "snapshot revert for slot 1 unsuccessful");
        // nothing to revert to anymore
        assert(!vm.revertToState(snapshotId));
    }

    function testStateSnapshotDelete() public {
        uint256 snapshotId = vm.snapshotState();
        store.setSlots(300, 400);
        vm.deleteStateSnapshot(snapshotId);
        // nothing to revert to anymore
        assert(!vm.revertToState(snapshotId));
    }

    function testStateSnapshotDeleteAll() public {
        uint256 snapshotId = vm.snapshotState();
        store.setSlots(300, 400);
        vm.deleteStateSnapshots();
        // nothing to revert to anymore
        assert(!vm.revertToState(snapshotId));
    }

    // <https://github.com/foundry-rs/foundry/issues/6411>
    function testStateSnapshotsMany() public {
        uint256 snapshotId;
        for (uint256 c = 0; c < 10; c++) {
            for (uint256 cc = 0; cc < 10; cc++) {
                snapshotId = vm.snapshotState();
                vm.revertToStateAndDelete(snapshotId);
                assert(!vm.revertToState(snapshotId));
            }
        }
    }

    // tests that snapshots can also revert changes to `block`
    function testBlockValues() public {
        uint256 num = store.blockNumber();
        uint256 time = store.blockTimestamp();

        uint256 snapshotId = vm.snapshotState();
        Storage store2 = new Storage();
        store2.setSlots(300, 400);

        assertEq(store2.slot0(), 300);
        assertEq(store2.slot1(), 400);

        vm.warp(1337);
        assertEq(store.blockTimestamp(), 1337);

        vm.roll(99);
        assertEq(store.blockNumber(), 99);

        assert(vm.revertToState(snapshotId));

        assertEq(store.blockNumber(), num, "snapshot revert for block.number unsuccessful");
        assertEq(store.blockTimestamp(), time, "snapshot revert for block.timestamp unsuccessful");
    }
}
