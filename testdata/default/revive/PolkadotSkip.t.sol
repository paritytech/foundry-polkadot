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
        // We emit the event we expect to see.
        vm.expectEmit();
        emit Added(3);

        Calculator calc = new Calculator();
        uint8 sum = calc.add(1, 2);
        assertEq(3, sum);
    }
}

contract PolkadotSkipTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);
    EvmTargetContract helper;

    function setUp() external {
        vm.pvm(true);
        helper = new EvmTargetContract();
        // ensure we can call cheatcodes from the helper
        vm.allowCheatcodes(address(helper));
        // and that the contract is kept between vm switches
        vm.makePersistent(address(helper));
    }

    function testUseCheatcodesInEvmWithSkip() external {
        vm.polkadotSkip();
        helper.exec();
    }

    function testAutoSkipAfterDeployInEvmWithSkip() external {
        vm.polkadotSkip();
        EvmTargetContract helper2 = new EvmTargetContract();

        // this should auto execute in EVM
        helper2.exec();
    }
}
