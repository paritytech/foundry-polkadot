// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "ds-test/test.sol";
import "cheats/Vm.sol";
import "../../default/logs/console.sol";

contract Storage {
    uint256 public slot0 = 10;
    uint256 public slot1 = 20;

    function setSlot0(uint256 value) public {
        slot0 = value;
    }

    function setSlot1(uint256 value) public {
        slot1 = value;
    }
}

contract StoreTestExtra is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);
    Storage store;

    function setUp() public {
        vm.pvm(true);
        store = new Storage();
        vm.makePersistent(address(store));
    }

    function testStoregeMigrationWorks() public {
        assertEq(store.slot0(), 10, "initial value for slot 0 is incorrect");
        assertEq(store.slot1(), 20, "initial value for slot 1 is incorrect");
        vm.store(address(store), bytes32(0), bytes32(uint256(1)));
        vm.pvm(false);
        assertEq(store.slot0(), 1, "store failed");
        assertEq(store.slot1(), 20, "store failed");
    }

    function testStoregeMigration2Works() public {
        assertEq(store.slot0(), 10, "initial value for slot 0 is incorrect");
        assertEq(store.slot1(), 20, "initial value for slot 1 is incorrect");
        store.setSlot0(1);
        vm.pvm(false);
        assertEq(store.slot0(), 1, "store failed");
        assertEq(store.slot1(), 20, "store failed");
    }
}
