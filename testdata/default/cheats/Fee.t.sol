// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "ds-test/test.sol";
import "cheats/Vm.sol";

contract FeeTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testFee() public {
        vm.fee(10);
        assertEq(block.basefee, 10, "fee failed");
    }

    function testFeeFuzzed(uint64 fee) public {
        vm.fee(fee);
        assertEq(block.basefee, fee, "fee failed");
    }

    function test_SetFeeOnce() public {
        vm.pvm(true);
        uint256 before = block.basefee;
        vm.fee(25 gwei);
        uint256 afterFee = block.basefee;
        assertTrue(before != afterFee);
        assertEq(afterFee, 25 gwei);
    }

    function test_SetFeeMultipleTimes() public {
        vm.pvm(true);
        vm.fee(10 gwei);
        assertEq(block.basefee, 10 gwei);
        vm.fee(50 gwei);
        assertEq(block.basefee, 50 gwei);
        vm.fee(1 gwei);
        assertEq(block.basefee, 1 gwei);
    }
}

contract FeePersistenceTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function test_FeePersistsAcrossCalls() public {
        vm.pvm(true);
        vm.fee(30 gwei);
        assertEq(block.basefee, 30 gwei);

        // simulate another call
        helper();
    }

    function helper() internal {
        assertEq(block.basefee, 30 gwei);
    }
}

contract FeeClampTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function test_ClampLowerThanBasefee() public {
        vm.pvm(true);
        vm.fee(200 gwei);
        uint256 clamped = min(block.basefee, 150 gwei);
        assertEq(clamped, 150 gwei);
    }

    function test_ClampHigherThanBasefee() public {
        vm.pvm(true);
        vm.fee(100 gwei);
        uint256 clamped = min(block.basefee, 500 gwei);
        assertEq(clamped, 100 gwei);
    }

    function min(uint256 a, uint256 b) internal pure returns (uint256) {
        return a < b ? a : b;
    }
}

contract FeeEdgeCases is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function test_ZeroBasefee() public {
        vm.pvm(true);
        vm.fee(0);
        assertEq(block.basefee, 0);
    }

    function test_MaxBasefee() public {
        vm.pvm(true);
        uint256 max = type(uint256).max;
        vm.fee(max);
        assertEq(block.basefee, type(uint64).max);
    }
}

contract FeeCrossMode is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function test_FeeConsistencyAcrossModes() public {
        vm.pvm(false);
        vm.fee(42 gwei);
        uint256 evmFee = block.basefee;

        vm.pvm(true);
        uint256 pvmFee = block.basefee;

        assertEq(evmFee, pvmFee);
    }
}

contract BasefeeConsumer {
    function readBasefee() external view returns (uint256) {
        return block.basefee;
    }
}

contract FeeExternalTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function test_BasefeeVisibleInDeployedContract() public {
        vm.pvm(true);
        vm.fee(42 gwei);

        // try calling block.basefee from a contract that's deployed on pvm side
        BasefeeConsumer consumer = new BasefeeConsumer();
        uint256 observed = consumer.readBasefee();

        assertEq(observed, 42 gwei, "deployed contract should see updated basefee");
    }
}
