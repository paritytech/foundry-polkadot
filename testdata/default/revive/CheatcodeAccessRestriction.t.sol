// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "ds-test/test.sol";
import "cheats/Vm.sol";

contract ScriptWithCheatcodes is DSTest {
    address public derivedAddress;
    uint256 private ownerKey;
    Vm constant vm = Vm(HEVM_ADDRESS);

    function setUp() public {
        // This pattern tries to use vm.envUint and vm.addr from a non-test contract
        ownerKey = vm.envUint("TEST_OWNER_KEY");
        derivedAddress = vm.addr(ownerKey);
    }
}

contract CheatcodeAccessRestrictionTest is DSTest {
    ScriptWithCheatcodes script;
    Vm constant vm = Vm(HEVM_ADDRESS);

    function setUp() public {
        vm.setEnv("TEST_OWNER_KEY", vm.toString(uint256(12345)));
    }

    function testScriptCheatcodesShouldFailInPolkadotMode() public {
        script = new ScriptWithCheatcodes();
        vm.expectRevert("Cheatcodes are not available in polkadot runtime.");
        script.setUp();
    }
}
