// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "ds-test/test.sol";
import "cheats/Vm.sol";

contract EvmReviveMigrationTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    address constant alice = address(0x1);
    address constant bob = address(0x2);

    function setUp() public {
        // Give alice and bob some initial balance in EVM mode
        vm.deal(alice, 1 ether);
        vm.deal(bob, 2 ether);

        // Mark accounts as persistent so they migrate when switching to PVM
        vm.makePersistent(alice);
        vm.makePersistent(bob);
    }

    function testBasicEvmToPvmMigration() public {
        // Start in EVM mode - verify initial balances
        assertEq(alice.balance, 1 ether);
        assertEq(bob.balance, 2 ether);

        // Switch to PVM mode
        vm.pvm(true);

        // Balances should be preserved after migration
        assertEq(alice.balance, 1 ether);
        assertEq(bob.balance, 2 ether);

        // Switch back to EVM mode
        vm.pvm(false);

        // Balances should still be preserved
        assertEq(alice.balance, 1 ether);
        assertEq(bob.balance, 2 ether);
    }

    function testBalanceModificationInPvm() public {
        // Start in EVM mode
        assertEq(alice.balance, 1 ether);

        // Switch to PVM mode
        vm.pvm(true);

        // Modify balance in PVM mode
        vm.deal(alice, 3 ether);
        assertEq(alice.balance, 3 ether);

        // Switch back to EVM mode
        vm.pvm(false);

        // Balance modification should be preserved
        assertEq(alice.balance, 3 ether);
    }

    function testNonceMigration() public {
        // Start in EVM mode
        uint64 initialNonce = vm.getNonce(alice);

        // Switch to PVM mode
        vm.pvm(true);

        // Increment nonce in PVM mode
        vm.setNonce(alice, initialNonce + 5);
        assertEq(vm.getNonce(alice), initialNonce + 5);

        // Switch back to EVM mode
        vm.pvm(false);

        // Nonce should be preserved
        assertEq(vm.getNonce(alice), initialNonce + 5);
    }

    function testMultipleAccountMigration() public {
        // Give more accounts some balance
        address charlie = address(0x3);
        address david = address(0x4);

        vm.deal(charlie, 0.5 ether);
        vm.deal(david, 1.5 ether);

        // Mark new accounts as persistent
        vm.makePersistent(charlie);
        vm.makePersistent(david);

        // Verify initial state
        assertEq(alice.balance, 1 ether);
        assertEq(bob.balance, 2 ether);
        assertEq(charlie.balance, 0.5 ether);
        assertEq(david.balance, 1.5 ether);

        // Switch to PVM mode
        vm.pvm(true);

        // All balances should be preserved
        assertEq(alice.balance, 1 ether);
        assertEq(bob.balance, 2 ether);
        assertEq(charlie.balance, 0.5 ether);
        assertEq(david.balance, 1.5 ether);

        // Modify some balances in PVM
        vm.deal(alice, 10 ether);
        vm.deal(charlie, 5 ether);

        // Switch back to EVM mode
        vm.pvm(false);

        // All changes should be preserved
        assertEq(alice.balance, 10 ether);
        assertEq(bob.balance, 2 ether);  // unchanged
        assertEq(charlie.balance, 5 ether);
        assertEq(david.balance, 1.5 ether);  // unchanged
    }

    function testPrecisionPreservation() public {
        // Test that dust/precision is preserved during migration
        uint256 preciseAmount = 1 ether + 123456; // Some wei precision

        vm.deal(alice, preciseAmount);
        assertEq(alice.balance, preciseAmount);

        // Switch to PVM and back
        vm.pvm(true);
        vm.pvm(false);

        // Precise amount should be preserved
        assertEq(alice.balance, preciseAmount);
    }

    function testRoundTripMigration() public {
        // Test multiple switches to ensure stability
        uint256 originalBalance = 1.123456789 ether;
        vm.deal(alice, originalBalance);

        // EVM -> PVM -> EVM -> PVM -> EVM
        vm.pvm(true);
        assertEq(alice.balance, originalBalance);

        vm.pvm(false);
        assertEq(alice.balance, originalBalance);

        vm.pvm(true);
        assertEq(alice.balance, originalBalance);

        vm.pvm(false);
        assertEq(alice.balance, originalBalance);
    }

    function testDustConversionPrecision() public {
        // Test specific dust scenarios that challenge the 128-bit -> 256-bit conversion

        // Test maximum 128-bit value plus some wei (should cause dust)
        uint256 maxU128 = 2**128 - 1;
        uint256 testBalance1 = maxU128 + 123456789; // This should create dust

        vm.deal(alice, testBalance1);
        uint256 beforePvm = alice.balance;

        // Switch to PVM (EVM -> PVM conversion may lose precision)
        vm.pvm(true);
        uint256 inPvm = alice.balance;

        // Switch back to EVM (should restore original precision via dust)
        vm.pvm(false);
        uint256 afterPvm = alice.balance;

        // The balance should be preserved exactly due to dust mechanism
        assertEq(beforePvm, afterPvm, "Dust conversion failed to preserve precision");

        // Test edge case: very small amounts (1 wei)
        vm.deal(bob, 1);
        vm.pvm(true);
        assertEq(bob.balance, 1, "1 wei should be preserved");
        vm.pvm(false);
        assertEq(bob.balance, 1, "1 wei should survive round trip");

        // Test edge case: amounts that don't fit in u128
        uint256 largeBalance = type(uint128).max + 1000000000000000000; // max u128 + 1 ether
        vm.deal(alice, largeBalance);

        vm.pvm(true);
        // In PVM, this should be capped or handled via dust
        uint256 pvmBalance = alice.balance;

        vm.pvm(false);
        // Should restore via dust mechanism
        uint256 restoredBalance = alice.balance;

        // The conversion should handle overflow gracefully
        // (exact behavior depends on pallet-revive implementation)
        assertTrue(restoredBalance <= largeBalance, "Balance should not exceed original");
    }

    function testSubWeiPrecision() public {
        // Test various sub-wei precision scenarios
        uint256[] memory testAmounts = new uint256[](5);
        testAmounts[0] = 1; // 1 wei
        testAmounts[1] = 999; // sub-gwei
        testAmounts[2] = 1000000000000000; // 0.001 ether
        testAmounts[3] = 1234567890123456789; // ~1.23 ether with precision
        testAmounts[4] = type(uint128).max - 1; // just under max u128

        for (uint i = 0; i < testAmounts.length; i++) {
            uint256 amount = testAmounts[i];

            vm.deal(alice, amount);
            uint256 original = alice.balance;

            // Round trip through PVM
            vm.pvm(true);
            vm.pvm(false);

            uint256 finalBalance = alice.balance;

            assertEq(original, finalBalance, "Sub-wei precision lost in round trip");
        }
    }

    function testBytecodeMigration() public {
        // Deploy a simple contract in PVM mode
        vm.pvm(true);
        SimpleStorage storageContract = new SimpleStorage();

        // Mark the contract as persistent so it migrates
        vm.makePersistent(address(storageContract));

        // Set initial value in PVM mode
        storageContract.set(42);
        assertEq(storageContract.get(), 42);

        // Switch to EVM mode - bytecode and state should migrate
        vm.pvm(false);

        // Verify state was preserved after migration to EVM

        // Modify state in EVM mode
        //storageContract.set(0);
        assertEq(storageContract.get(), 42);

        storageContract.set(100);

        assertEq(storageContract.get(), 100);
    }
}

contract SimpleStorage {
    uint256 private value;

    function set(uint256 newValue) public {
        value = newValue;
    }

    function get() public view returns (uint256) {
        return value;
    }
}