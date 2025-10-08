// Just a copy of cheatcodes Prank.t.sol adapted to work with pvm backend.
// The adaptions are only to switch back and forth between evm and pvm.
forgetest!(mock_call, |prj, cmd| {
    prj.insert_ds_test();
    prj.insert_vm();
    prj.insert_console();
    prj.add_source(
        "MockCall.t.sol",
        r#"
// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;


import "./test.sol";
import "./Vm.sol";
import {console} from "./console.sol";

contract MockCallsTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testMockCallsLastShouldPersist() public {
        vm.pvm(true);
        address mockUser = vm.addr(vm.randomUint());
        address mockErc20 = vm.addr(vm.randomUint());
        bytes memory data = abi.encodeWithSignature("balanceOf(address)", mockUser);
        bytes[] memory mocks = new bytes[](2);
        mocks[0] = abi.encode(2 ether);
        mocks[1] = abi.encode(7.219 ether);
        vm.mockCalls(mockErc20, data, mocks);
        (, bytes memory ret1) = mockErc20.call(data);
        assertEq(abi.decode(ret1, (uint256)), 2 ether);
        (, bytes memory ret2) = mockErc20.call(data);
        assertEq(abi.decode(ret2, (uint256)), 7.219 ether);
        (, bytes memory ret3) = mockErc20.call(data);
        assertEq(abi.decode(ret3, (uint256)), 7.219 ether);
    }

    function testMockCallsWithValue() public {
        vm.pvm(true);

        address mockUser = vm.addr(vm.randomUint());
        address mockErc20 = vm.addr(vm.randomUint());
        bytes memory data = abi.encodeWithSignature("balanceOf(address)", mockUser);
        bytes[] memory mocks = new bytes[](3);
        mocks[0] = abi.encode(2 ether);
        mocks[1] = abi.encode(1 ether);
        mocks[2] = abi.encode(6.423 ether);
        vm.mockCalls(mockErc20, 1 ether, data, mocks);
        (, bytes memory ret1) = mockErc20.call{value: 1 ether}(data);
        assertEq(abi.decode(ret1, (uint256)), 2 ether);
        (, bytes memory ret2) = mockErc20.call{value: 1 ether}(data);
        assertEq(abi.decode(ret2, (uint256)), 1 ether);
        (, bytes memory ret3) = mockErc20.call{value: 1 ether}(data);
        assertEq(abi.decode(ret3, (uint256)), 6.423 ether);
    }

    function testMockCalls() public {
        vm.pvm(true);

        address mockUser = vm.addr(vm.randomUint());
        address mockErc20 = vm.addr(vm.randomUint());
        bytes memory data = abi.encodeWithSignature("balanceOf(address)", mockUser);
        bytes[] memory mocks = new bytes[](3);
        mocks[0] = abi.encode(2 ether);
        mocks[1] = abi.encode(1 ether);
        mocks[2] = abi.encode(6.423 ether);
        vm.mockCalls(mockErc20, data, mocks);
        (, bytes memory ret1) = mockErc20.call(data);
        assertEq(abi.decode(ret1, (uint256)), 2 ether);
        (, bytes memory ret2) = mockErc20.call(data);
        assertEq(abi.decode(ret2, (uint256)), 1 ether);
        (, bytes memory ret3) = mockErc20.call(data);
        assertEq(abi.decode(ret3, (uint256)), 6.423 ether);
    }
}

"#,
    )
    .unwrap();

    let res = cmd.args(["test", "--resolc", "--resolc-startup"]).assert_success();

    res.stderr_eq(str![""]).stdout_eq(str![[r#"
[COMPILING_FILES] with [SOLC_VERSION]
[SOLC_VERSION] [ELAPSED]
Compiler run successful!
[COMPILING_FILES] with [RESOLC_VERSION]
[RESOLC_VERSION] [ELAPSED]
Compiler run successful!

Ran 3 tests for src/MockCall.t.sol:MockCallsTest
[PASS] testMockCalls() ([GAS])
[PASS] testMockCallsLastShouldPersist() ([GAS])
[PASS] testMockCallsWithValue() ([GAS])
Suite result: ok. 3 passed; 0 failed; 0 skipped; [ELAPSED]

Ran 1 test suite [ELAPSED]: 3 tests passed, 0 failed, 0 skipped (3 total tests)

"#]]);
});
