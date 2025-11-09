// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "ds-test/test.sol";
import "cheats/Vm.sol";

contract Worker {
    uint256 public result;

    function doWork() public returns (uint256) {
        uint256 sum = 0;
        for (uint256 i = 0; i < 100; i++) {
            sum += i;
        }
        result = sum;
        return sum;
    }

    function expensiveWork() public returns (uint256) {
        uint256 sum = 0;
        for (uint256 i = 0; i < 1000; i++) {
            sum += i;
        }
        result = sum;
        return sum;
    }
}

contract GasMeteringTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);
    Worker public worker;

    function setUp() public {
        vm.pvm(true);
        worker = new Worker();
    }

    function testPauseGasMeteringWithPvmCall() public {
        uint256 gasStart = gasleft();
        worker.doWork();
        uint256 gasUsedNormal = gasStart - gasleft();

        vm.pauseGasMetering();
        uint256 gasPausedStart = gasleft();
        worker.doWork();
        uint256 gasUsedPaused = gasPausedStart - gasleft();
        vm.resumeGasMetering();

        assertTrue(gasUsedNormal > 0);
        assertEq(gasUsedPaused, 0);
    }

    function testResumeGasMeteringWithPvmCall() public {
        vm.pauseGasMetering();
        worker.doWork();
        vm.resumeGasMetering();

        uint256 gasStart = gasleft();
        worker.doWork();
        uint256 gasUsed = gasStart - gasleft();

        assertTrue(gasUsed > 0);
    }

    function testResetGasMeteringWithPvmCall() public {
        uint256 gasStart = gasleft();
        worker.expensiveWork();
        uint256 gasAfterWork = gasleft();
        uint256 gasConsumed = gasStart - gasAfterWork;

        vm.resetGasMetering();
        uint256 gasAfterReset = gasleft();

        assertTrue(gasAfterReset > gasAfterWork);
        uint256 gasRecovered = gasAfterReset - gasAfterWork;
        assertTrue(gasRecovered > gasConsumed / 2);
    }

    function testCreateDuringPausedMetering() public {
        vm.pauseGasMetering();
        uint256 gasStart = gasleft();

        Worker newWorker = new Worker();
        newWorker.doWork();

        uint256 gasUsed = gasStart - gasleft();
        vm.resumeGasMetering();

        assertEq(gasUsed, 0);
    }
}
