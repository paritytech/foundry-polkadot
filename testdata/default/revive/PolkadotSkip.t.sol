// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "ds-test/test.sol";
import "cheats/Vm.sol";

contract Calculator {
    event Added(uint8 indexed sum);

    function add(uint8 a, uint8 b) public returns (uint8) {
        uint8 sum = a + b;
        emit Added(sum);
        return sum;
    }
}

contract EvmTargetContract is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    event Added(uint8 indexed sum);

    function exec() public {
        emit Added(3);

        Calculator calc = new Calculator();
        uint8 sum = calc.add(1, 2);
        assertEq(3, sum);
        vm.setNonce(address(this), 10);
    }
}

contract PolkadotSkipTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);
    EvmTargetContract helper;

    function setUp() external {
        assertEq(vm.getNonce(address(this)), 1);
        helper = new EvmTargetContract();
        assertEq(vm.getNonce(address(this)), 2);

        // ensure we can call cheatcodes from the helper
        vm.allowCheatcodes(address(helper));
    }

    function testUseCheatcodesInEvmWithSkip() external {
        vm.polkadotSkip();
        helper.exec();
        assertEq(vm.getNonce(address(helper)), 10);
    }

    function testAutoSkipAfterDeployInEvmWithSkip() external {
        assertEq(vm.getNonce(address(this)), 2);
        vm.polkadotSkip();
        EvmTargetContract helper2 = new EvmTargetContract();
        // this should auto execute in EVM
        helper2.exec();
        assertEq(vm.getNonce(address(helper2)), 10);
    }

    function testreviveWhenUseCheatcodeWithoutSkip() external {
        uint256 nonceBefore = vm.getNonce(address(helper));
        helper.exec();
        assertEq(vm.getNonce(address(helper)), nonceBefore + 1);
    }
}
