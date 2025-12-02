// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "ds-test/test.sol";
import "cheats/Vm.sol";

contract SimpleStorage {
    uint256 private value;

    function set(uint256 _value) public {
        value = _value;
    }

    function get() public view returns (uint256) {
        return value;
    }
}

contract StorageWithImmutables {
    uint256 public immutable deployedAt;
    address public immutable deployer;
    uint256 public immutable magicNumber;

    constructor(uint256 _magicNumber) {
        deployedAt = block.timestamp;
        deployer = msg.sender;
        magicNumber = _magicNumber;
    }

    function getDeployedAt() public view returns (uint256) {
        return deployedAt;
    }

    function getDeployer() public view returns (address) {
        return deployer;
    }

    function getMagicNumber() public view returns (uint256) {
        return magicNumber;
    }
}

interface IAuthorizationCallback {
    function onAuthorization(address caller, uint256 value) external returns (bool);
}

contract CallbackContract {
    address public owner;
    uint256 public lastValue;
    address public lastCaller;

    constructor() {
        owner = msg.sender;
    }

    // This function calls back to the caller to verify authorization
    // Similar to how Morpho calls back to verify permissions
    function executeWithCallback(uint256 value) public returns (bool) {
        // Call back to the msg.sender to verify authorization
        bool authorized = IAuthorizationCallback(msg.sender).onAuthorization(msg.sender, value);

        if (authorized) {
            lastValue = value;
            lastCaller = msg.sender;
            return true;
        }

        return false;
    }

    function getLastValue() public view returns (uint256) {
        return lastValue;
    }

    function getLastCaller() public view returns (address) {
        return lastCaller;
    }
}

contract EvmReviveMigrationTest is DSTest {
    Vm constant vm = Vm(address(uint160(uint256(keccak256("hevm cheat code")))));
    address alice = address(0x1111);

    function setUp() public {
        vm.deal(alice, 1 ether);
        // Mark accounts as persistent so they migrate when switching between EVM and PVM
        vm.makePersistent(alice);
    }

    function testBalanceMigration() public {
        // Tests run in Revive by default when using runner_revive
        vm.deal(alice, 3 ether);
        uint256 reviveBalance = alice.balance;
        assertEq(reviveBalance, 3 ether, "Revive balance should be 3 ether");

        vm.polkadot(false);

        assertEq(alice.balance, reviveBalance, "Balance should migrate from Revive to EVM");

        vm.deal(alice, 2 ether);
        uint256 evmBalance = alice.balance;
        assertEq(evmBalance, 2 ether, "Revive balance should be 2 ether");

        vm.polkadot(true);

        assertEq(alice.balance, evmBalance, "Balance should migrate from EVM to Revive");
    }

    function testNonceMigration() public {
        vm.setNonce(alice, 5);
        uint256 reviveNonce = vm.getNonce(alice);
        assertEq(reviveNonce, 5, "Nonce in Revive should be 5");

        vm.polkadot(false);

        assertEq(vm.getNonce(alice), reviveNonce, "Nonce should migrate from Revive to EVM");

        vm.setNonce(alice, 10);
        uint256 evmNonce = vm.getNonce(alice);
        assertEq(evmNonce, 10, "Nonce in Revive should be 10");

        vm.polkadot(true);
        assertEq(vm.getNonce(alice), evmNonce, "Nonce should migrate from EVM to Revive");
    }

    function testPrecisionPreservation() public {
        // Set precise balance in Revive (with wei precision)
        vm.deal(alice, 1123456789123456789);
        uint256 reviveBalance = alice.balance;
        assertEq(reviveBalance, 1123456789123456789, "Balance should be set correctly in Revive");

        vm.polkadot(false);

        assertEq(alice.balance, 1123456789123456789, "Balance precision should be preserved in migration to EVM");

        vm.deal(alice, 1123456789123456790);
        uint256 evmBalance = alice.balance;
        assertEq(evmBalance, 1123456789123456790, "Balance should be set correctly in EVM");

        vm.polkadot(true);
        assertEq(alice.balance, evmBalance, "Balance precision should be preserved in migration back to Revive");
    }

    function testBytecodeMigrationToEvm() public {
        SimpleStorage storageContract = new SimpleStorage();

        // Mark the contract as persistent so it migrates
        vm.makePersistent(address(storageContract));

        storageContract.set(42);
        assertEq(storageContract.get(), 42);

        vm.polkadot(false);

        assertEq(storageContract.get(), 42);

        storageContract.set(100);
        assertEq(storageContract.get(), 100);
    }

    function testBytecodeMigrationToRevive() public {
        vm.polkadot(false);
        SimpleStorage storageContract = new SimpleStorage();

        // Mark the contract as persistent so it migrates
        vm.makePersistent(address(storageContract));

        storageContract.set(42);
        assertEq(storageContract.get(), 42);

        vm.polkadot(true);

        assertEq(storageContract.get(), 42);

        storageContract.set(100);
        assertEq(storageContract.get(), 100);
    }

    function testStorageMigration() public {
        SimpleStorage storageContract = new SimpleStorage();

        // Mark the contract as persistent so it migrates
        vm.makePersistent(address(storageContract));

        storageContract.set(42);
        assertEq(storageContract.get(), 42);

        vm.polkadot(false);
        SimpleStorage storageContract2 = new SimpleStorage();
        vm.makePersistent(address(storageContract2));
        assertEq(storageContract.get(), 42);

        storageContract.set(100);
        storageContract2.set(100);
        assertEq(storageContract.get(), 100);

        vm.polkadot(true);
        assertEq(storageContract.get(), 100);
        assertEq(storageContract2.get(), 100);
    }

    function testTimestampMigration() public {
        uint256 initialTimestamp = 1_000_000;
        vm.warp(initialTimestamp);

        uint256 reviveTimestamp = block.timestamp;
        assertEq(reviveTimestamp, initialTimestamp, "Timestamp in Revive should match initial value");

        vm.polkadot(false);

        uint256 evmTimestamp = block.timestamp;
        assertEq(evmTimestamp, reviveTimestamp, "Timestamp should migrate from Revive to EVM");

        uint256 newEvmTimestamp = 2_000_000_000;
        vm.warp(newEvmTimestamp);
        assertEq(block.timestamp, newEvmTimestamp, "Timestamp in EVM should update correctly");

        vm.polkadot(true);

        uint256 finalReviveTimestamp = block.timestamp;
        assertEq(finalReviveTimestamp, newEvmTimestamp, "Timestamp should migrate from EVM to Revive");
    }

    function testImmutablesMigration() public {
        vm.polkadot(false);

        uint256 deploymentTimestamp = 1234567890;
        vm.warp(deploymentTimestamp);
        uint256 magicNumber = 0x42424242;
        StorageWithImmutables immutableContract = new StorageWithImmutables(magicNumber);

        vm.makePersistent(address(immutableContract));

        assertEq(immutableContract.getDeployedAt(), deploymentTimestamp, "Deployed timestamp should match in EVM");
        assertEq(immutableContract.getDeployer(), address(this), "Deployer should match in EVM");
        assertEq(immutableContract.getMagicNumber(), magicNumber, "Magic number should match in EVM");

        vm.polkadot(true);

        assertEq(
            immutableContract.getDeployedAt(), deploymentTimestamp, "Deployed timestamp should be preserved in Revive"
        );
        assertEq(immutableContract.getDeployer(), address(this), "Deployer should be preserved in Revive");
        assertEq(immutableContract.getMagicNumber(), magicNumber, "Magic number should be preserved in Revive");
    }

    // Implement the authorization callback interface
    function onAuthorization(address caller, uint256 value) external returns (bool) {
        // Simple authorization: allow if value is less than 1000
        return value < 1000;
    }

    function testCallbackFromRevive() public {
        CallbackContract callbackContract = new CallbackContract();
        // Try to execute with authorized value (should succeed)
        uint256 authorizedValue = 500;
        bool result = callbackContract.executeWithCallback(authorizedValue);
        assertTrue(result, "Authorized callback should succeed");
        assertEq(callbackContract.getLastValue(), authorizedValue, "Last value should be updated");
        assertEq(callbackContract.getLastCaller(), address(this), "Last caller should be test contract");

        // Try to execute with unauthorized value (should fail)
        uint256 unauthorizedValue = 1500;
        bool result2 = callbackContract.executeWithCallback(unauthorizedValue);
        assertTrue(!result2, "Unauthorized callback should fail");
        assertEq(callbackContract.getLastValue(), authorizedValue, "Last value should not be updated");
    }
}
