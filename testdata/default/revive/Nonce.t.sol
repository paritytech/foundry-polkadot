// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "ds-test/test.sol";
import "cheats/Vm.sol";

contract Counter {
    uint256 public count;

    function increment() public {
        count += 1;
    }
}

contract Child {
    uint256 public id;

    constructor(uint256 _id) {
        id = _id;
    }
}

contract RevertingChild {
    uint256 public value;

    constructor(bool shouldRevert) {
        if (shouldRevert) {
            revert("Constructor revert");
        }
        value = 42;
    }
}

contract Factory {
    function callSomething() public pure {}

    function deployCreate(uint256 id) public returns (address deployed) {
        deployed = address(new Child(id));
    }

    function deployCreate2(bytes32 salt, uint256 id) public returns (address deployed) {
        bytes memory initCode = abi.encodePacked(type(Child).creationCode, abi.encode(id));
        assembly {
            deployed := create2(0, add(initCode, 0x20), mload(initCode), salt)
            if iszero(deployed) { revert(0, 0) }
        }
    }

    function deployCreateReverting(bool shouldRevert) public returns (address deployed) {
        deployed = address(new RevertingChild(shouldRevert));
    }

    function deployCreate2Reverting(bytes32 salt, bool shouldRevert) public returns (address deployed) {
        bytes memory initCode = abi.encodePacked(type(RevertingChild).creationCode, abi.encode(shouldRevert));
        assembly {
            deployed := create2(0, add(initCode, 0x20), mload(initCode), salt)
            if iszero(deployed) { revert(0, 0) }
        }
    }
}

contract NonceTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    address constant EOA_ALICE = address(0xAA11);
    Factory factory;

    function setUp() public {
        vm.startPrank(EOA_ALICE);
        factory = new Factory();
        vm.stopPrank();
    }

    // --- SECTION 1: EOA NONCE BEHAVIOR ---
    // EOA nonce should NOT increment on staticcalls (pure/view functions)
    // EOA nonce SHOULD increment on contract deployments

    function test_EOA_Nonce_IncrementsOnEveryTx() public {
        vm.startPrank(EOA_ALICE);

        // EOA_ALICE's nonce is currently 1 (from setUp)
        uint256 nonceBefore = vm.getNonce(EOA_ALICE);
        assertEq(nonceBefore, 1);

        // 1. EOA sends a staticcall (pure function) - nonce should NOT increment
        factory.callSomething();
        assertEq(vm.getNonce(EOA_ALICE), nonceBefore, "EOA: Nonce should not increment on staticcall");

        // 2. EOA deploys a new contract (uses CREATE) - nonce SHOULD increment
        new Counter();
        assertEq(vm.getNonce(EOA_ALICE), nonceBefore + 1, "EOA: Nonce should increment after deployment");
    }

    // --- SECTION 2: CONTRACT NONCE BEHAVIOR ---
    // The Contract nonce increments ONLY on contract creation.

    function test_Contract_Nonce_NoIncrementOnCall() public {
        uint256 nonceBefore = vm.getNonce(address(factory)); // Factory nonce should be 0 (it hasn't created a contract yet)

        // 1. EOA calls Factory, Factory executes internal function callSomething()
        vm.startPrank(EOA_ALICE);
        factory.callSomething();
        vm.stopPrank();

        // Contract's nonce should NOT change, as no new contract was created.
        assertEq(vm.getNonce(address(factory)), nonceBefore, "Contract: Nonce should NOT change on simple call");
    }

    function test_Contract_Nonce_IncrementsOnCREATE() public {
        uint256 nonceBefore = vm.getNonce(address(factory)); // Factory nonce should be 0

        // 1. EOA calls Factory, Factory executes deployCreate()
        vm.startPrank(EOA_ALICE);
        factory.deployCreate(1);
        vm.stopPrank();

        // The contract's nonce MUST increment to 1 after using the CREATE opcode.
        assertEq(vm.getNonce(address(factory)), nonceBefore + 1, "Contract: Nonce MUST increment after CREATE");
    }

    function test_Contract_Nonce_IncrementsOnCREATE2() public {
        uint256 nonceBefore = vm.getNonce(address(factory)); // Factory nonce should be 0
        bytes32 salt = bytes32(uint256(12345));

        // 1. EOA calls Factory, Factory executes deployCreate2()
        vm.startPrank(EOA_ALICE);
        factory.deployCreate2(salt, 1);
        vm.stopPrank();

        // The contract's nonce MUST increment to 1 after using the CREATE2 opcode.
        assertEq(vm.getNonce(address(factory)), nonceBefore + 1, "Contract: Nonce MUST increment after CREATE2");
    }

    // --- SECTION 3: CONSTRUCTOR REVERT BEHAVIOR ---
    // Test nonce behavior when constructor reverts

    function test_Contract_Nonce_CREATE_ConstructorRevert() public {
        uint256 nonceBefore = vm.getNonce(address(factory));
        // Try to deploy with reverting constructor
        vm.startPrank(EOA_ALICE);
        try factory.deployCreateReverting(true) {
            fail();
        } catch {}
        vm.stopPrank();

        uint256 nonceAfter = vm.getNonce(address(factory));
        // Nonce should NOT increment when constructor reverts (entire CREATE operation rolls back)
        assertEq(nonceAfter, nonceBefore, "Nonce should NOT increment on constructor revert (CREATE)");
    }

    function test_Contract_Nonce_CREATE2_ConstructorRevert() public {
        uint256 nonceBefore = vm.getNonce(address(factory));
        bytes32 salt = bytes32(uint256(99999));

        // Try to deploy with reverting constructor via CREATE2
        vm.startPrank(EOA_ALICE);
        try factory.deployCreate2Reverting(salt, true) {
            fail();
        } catch {}
        vm.stopPrank();

        uint256 nonceAfter = vm.getNonce(address(factory));
        // Nonce should NOT increment when constructor reverts (entire CREATE2 operation rolls back)
        assertEq(nonceAfter, nonceBefore, "Nonce should NOT increment on constructor revert (CREATE2)");
    }

    // --- SECTION 4: MULTIPLE DEPLOYMENTS WITH SAME BYTECODE ---

    function test_CREATE2_SameBytecode_DifferentSalts() public {
        uint256 nonceBefore = vm.getNonce(address(factory));

        vm.startPrank(EOA_ALICE);

        // Deploy same bytecode with different salts - should work
        address addr1 = factory.deployCreate2(bytes32(uint256(1)), 100);
        address addr2 = factory.deployCreate2(bytes32(uint256(2)), 200);

        vm.stopPrank();

        // Both deployments should succeed with different addresses
        assertTrue(addr1 != address(0), "First deployment failed");
        assertTrue(addr2 != address(0), "Second deployment failed");
        assertTrue(addr1 != addr2, "Addresses should be different");

        // Nonce should increment twice
        assertEq(vm.getNonce(address(factory)), nonceBefore + 2, "Nonce should increment for each CREATE2");
    }

    function test_CREATE2_RevertThenSuccess_SameSalt() public {
        uint256 nonceBefore = vm.getNonce(address(factory));
        bytes32 salt = bytes32(uint256(54321));

        vm.startPrank(EOA_ALICE);

        // First: Try with reverting constructor
        try factory.deployCreate2Reverting(salt, true) {
            fail();
        } catch {}

        uint256 nonceAfterRevert = vm.getNonce(address(factory));
        assertEq(nonceAfterRevert, nonceBefore, "Nonce should NOT increment after failed CREATE2");

        // Second: Deploy successfully with SAME salt but shouldRevert=false
        address deployed = factory.deployCreate2Reverting(salt, false);

        vm.stopPrank();

        // Second deployment should succeed
        assertTrue(deployed != address(0), "Second deployment with same salt should succeed");
        assertEq(vm.getNonce(address(factory)), nonceBefore + 1, "Only successful deployment should increment nonce");
    }

    // --- SECTION 5: MIMICKING THE FAILING test_record SCENARIO ---

    function test_CREATE_Then_CREATE2_SameBytecode() public {
        vm.startPrank(EOA_ALICE);

        uint256 nonceBefore = vm.getNonce(address(factory));

        // First: Deploy with CREATE (succeeds)
        address addr1 = factory.deployCreateReverting(false);
        assertTrue(addr1 != address(0), "CREATE deployment failed");

        uint256 nonceAfter1 = vm.getNonce(address(factory));
        assertEq(nonceAfter1, nonceBefore + 1, "Nonce should increment after CREATE");

        // Second: Deploy with CREATE2 and REVERTING constructor
        try factory.deployCreate2Reverting(bytes32(0), true) {
            fail();
        } catch {}

        uint256 nonceAfter2 = vm.getNonce(address(factory));
        assertEq(nonceAfter2, nonceBefore + 1, "Nonce should NOT increment after failed CREATE2");

        // Third: Try another CREATE2 with different salt to see if system is still working
        address addr3 = factory.deployCreate2Reverting(bytes32(uint256(1)), false);
        assertTrue(addr3 != address(0), "Subsequent CREATE2 should work");
        assertEq(vm.getNonce(address(factory)), nonceBefore + 2, "Nonce should continue incrementing");

        vm.stopPrank();
    }

    function test_Multiple_CREATE2_SameBytecode() public {
        vm.startPrank(EOA_ALICE);

        // Deploy same Child contract bytecode 3 times with different salts
        address addr1 = factory.deployCreate2(bytes32(uint256(1)), 111);
        address addr2 = factory.deployCreate2(bytes32(uint256(2)), 222);
        address addr3 = factory.deployCreate2(bytes32(uint256(3)), 333);

        vm.stopPrank();

        assertTrue(addr1 != address(0), "Deploy 1 failed");
        assertTrue(addr2 != address(0), "Deploy 2 failed");
        assertTrue(addr3 != address(0), "Deploy 3 failed");
        assertTrue(addr1 != addr2 && addr2 != addr3 && addr1 != addr3, "All addresses should be unique");
    }
}
