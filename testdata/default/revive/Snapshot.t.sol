// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "utils/DSTest.sol";
import "utils/Vm.sol";

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

    function testOutOfOrderRevert() public {
        uint256 snap1 = vm.snapshotState();
        store.setSlots(300, 400);

        vm.snapshotState();
        store.setSlots(500, 600);

        vm.snapshotState();
        store.setSlots(700, 800);

        assertEq(store.slot0(), 700, "should be at 700");

        vm.revertToState(snap1);
        assertEq(store.slot0(), 10, "out-of-order revert to snap1 failed");

        store.setSlots(999, 888);
        assertEq(store.slot0(), 999, "post-revert write failed");
        assertEq(store.slot1(), 888, "post-revert write failed for slot1");
    }

    function testSnapshotAfterOutOfOrderRevert() public {
        uint256 snap1 = vm.snapshotState();
        store.setSlots(300, 400);

        vm.snapshotState();
        store.setSlots(500, 600);

        vm.snapshotState();
        store.setSlots(700, 800);

        vm.revertToState(snap1);
        assertEq(store.slot0(), 10, "initial revert failed");

        uint256 newSnap = vm.snapshotState();
        store.setSlots(999, 888);
        assertEq(store.slot0(), 999, "write after new snapshot failed");

        vm.revertToState(newSnap);
        assertEq(store.slot0(), 10, "revert to new snapshot failed - pallet-revive depth may be wrong");
        assertEq(store.slot1(), 20, "revert to new snapshot failed for slot1");
    }

    function testRevertChangeRevertAgain() public {
        uint256 snapshotId = vm.snapshotState();

        store.setSlots(300, 400);
        assertEq(store.slot0(), 300, "should be 300 after first change");

        vm.revertToState(snapshotId);
        assertEq(store.slot0(), 10, "should restore to 10 after first revert");

        store.setSlots(500, 600);
        assertEq(store.slot0(), 500, "should be 500 after second change");

        vm.revertToState(snapshotId);
        assertEq(store.slot0(), 10, "should restore to 10 after second revert");
        assertEq(store.slot1(), 20, "should restore to 20 after second revert");

        store.setSlots(700, 800);
        assertEq(store.slot0(), 700, "should be 700 after third change");

        vm.revertToState(snapshotId);
        assertEq(store.slot0(), 10, "should restore to 10 after third revert");
        assertEq(store.slot1(), 20, "should restore to 20 after third revert");
    }

    function testNewContractRollback() public {
        uint256 snapshotId = vm.snapshotState();

        Storage newStore = new Storage();
        address newAddr = address(newStore);
        newStore.setSlots(777, 888);
        assertEq(newStore.slot0(), 777, "new contract should have value");

        uint256 codeSizeBefore = newAddr.code.length;
        assertTrue(codeSizeBefore > 0, "contract should have code before revert");

        vm.revertToState(snapshotId);

        uint256 codeSizeAfter = newAddr.code.length;
        assertEq(codeSizeAfter, 0, "contract code should be gone after revert");
    }

    function testExistingContractStorageRollback() public {
        assertEq(store.slot0(), 10, "initial state");

        uint256 snapshotId = vm.snapshotState();

        store.setSlots(999, 888);
        assertEq(store.slot0(), 999, "modified state");

        Storage newStore = new Storage();
        newStore.setSlots(111, 222);

        vm.revertToState(snapshotId);

        assertEq(store.slot0(), 10, "existing contract should be restored");
        assertEq(store.slot1(), 20, "existing contract slot1 should be restored");
    }
}

contract SnapshotAcrossModeSwitchTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    Storage store;

    function setUp() public {
        store = new Storage();
        store.setSlots(100, 200);
    }

    function testSnapshotInRevmThenSwitchToPolkadot() public {
        vm.polkadot(false);

        uint256 snapshotId = vm.snapshotState();

        store.setSlots(300, 400);
        assertEq(store.slot0(), 300, "REVM should have 300");

        vm.polkadot(true);

        store.setSlots(500, 600);
        assertEq(store.slot0(), 500, "pallet-revive should have 500");

        vm.revertToState(snapshotId);

        assertEq(store.slot0(), 100, "should restore to 100 after revert");
        assertEq(store.slot1(), 200, "should restore to 200 after revert");
    }

    function testSnapshotInPolkadotThenSwitchToRevm() public {
        uint256 snapshotId = vm.snapshotState();

        store.setSlots(300, 400);
        assertEq(store.slot0(), 300, "pallet-revive should have 300");

        vm.polkadot(false);

        store.setSlots(500, 600);
        assertEq(store.slot0(), 500, "REVM should have 500");

        vm.revertToState(snapshotId);

        assertEq(store.slot0(), 100, "should restore to 100 after revert");
        assertEq(store.slot1(), 200, "should restore to 200 after revert");
    }

    function testNewSnapshotAfterModeSwitchBack() public {
        vm.polkadot(false);

        uint256 revmSnap = vm.snapshotState();

        store.setSlots(300, 400);
        assertEq(store.slot0(), 300, "REVM should have 300");

        vm.polkadot(true);

        uint256 polkadotSnap = vm.snapshotState();

        store.setSlots(500, 600);
        assertEq(store.slot0(), 500, "pallet-revive should have 500");

        vm.revertToState(polkadotSnap);
        assertEq(store.slot0(), 300, "polkadotSnap should restore to 300");

        vm.revertToState(revmSnap);
        assertEq(store.slot0(), 100, "revmSnap should restore to 100");
        assertEq(store.slot1(), 200, "revmSnap should restore to 200");
    }

    function testMultipleModeSwitches() public {
        assertEq(store.slot0(), 100, "initial slot0");

        vm.polkadot(false);
        store.setSlots(200, 201);
        assertEq(store.slot0(), 200, "REVM cycle 1");

        vm.polkadot(true);
        store.setSlots(300, 301);
        assertEq(store.slot0(), 300, "pallet-revive cycle 1");

        vm.polkadot(false);
        store.setSlots(400, 401);
        assertEq(store.slot0(), 400, "REVM cycle 2");

        vm.polkadot(true);
        store.setSlots(500, 501);
        assertEq(store.slot0(), 500, "pallet-revive cycle 2");

        vm.polkadot(false);
        store.setSlots(600, 601);
        assertEq(store.slot0(), 600, "REVM cycle 3");

        vm.polkadot(true);
        store.setSlots(700, 701);
        assertEq(store.slot0(), 700, "pallet-revive cycle 3");

        assertEq(store.slot0(), 700, "final value should be 700");
    }

    function testMultipleModeSwitchesWithSnapshot() public {
        uint256 initialSnap = vm.snapshotState();

        vm.polkadot(false);
        store.setSlots(200, 201);

        vm.polkadot(true);
        store.setSlots(300, 301);

        vm.polkadot(false);
        store.setSlots(400, 401);

        vm.polkadot(true);
        store.setSlots(500, 501);

        assertEq(store.slot0(), 500, "should be 500 after switches");

        vm.revertToState(initialSnap);

        assertEq(store.slot0(), 100, "should restore to initial 100");
        assertEq(store.slot1(), 200, "should restore to initial 200");
    }

    function testStatePersistsAcrossMigrations() public {
        store.setSlots(111, 222);

        vm.polkadot(false);
        assertEq(store.slot0(), 111, "REVM should have migrated value");

        vm.polkadot(true);
        assertEq(store.slot0(), 111, "pallet-revive should have value back");

        vm.polkadot(false);
        store.setSlots(333, 444);

        vm.polkadot(true);
        assertEq(store.slot0(), 333, "pallet-revive should have REVM changes");

        vm.polkadot(false);
        vm.polkadot(true);
        assertEq(store.slot0(), 333, "value should persist through empty migration");
    }
}

contract SnapshotConstructorContractTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    Storage constructorStore;
    Storage setupStore;

    constructor() {
        constructorStore = new Storage();
        constructorStore.setSlots(1, 2);
    }

    function setUp() public {
        setupStore = new Storage();
        setupStore.setSlots(10, 20);
    }

    function testConstructorContractPreserved() public {
        assertEq(constructorStore.slot0(), 1, "constructor contract initial slot0");
        assertEq(constructorStore.slot1(), 2, "constructor contract initial slot1");
        assertEq(setupStore.slot0(), 10, "setUp contract initial slot0");

        uint256 snapshotId = vm.snapshotState();

        constructorStore.setSlots(100, 200);
        setupStore.setSlots(300, 400);

        assertEq(constructorStore.slot0(), 100, "constructor contract modified");
        assertEq(setupStore.slot0(), 300, "setUp contract modified");

        vm.revertToState(snapshotId);

        assertEq(constructorStore.slot0(), 1, "constructor contract should be restored");
        assertEq(constructorStore.slot1(), 2, "constructor contract slot1 should be restored");
        assertEq(setupStore.slot0(), 10, "setUp contract should be restored");
        assertEq(setupStore.slot1(), 20, "setUp contract slot1 should be restored");
    }
}
