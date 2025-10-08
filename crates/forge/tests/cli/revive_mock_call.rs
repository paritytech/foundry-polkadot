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

contract Mock {
    uint256 state = 0;

    function numberA() public pure returns (uint256) {
        return 1;
    }

    function numberB() public pure returns (uint256) {
        return 2;
    }

    function add(uint256 a, uint256 b) public pure returns (uint256) {
        return a + b;
    }

    function pay(uint256 a) public payable returns (uint256) {
        return a;
    }

    function noReturnValue() public {
        // Does nothing of value, but also ensures that Solidity will 100%
        // generate an `extcodesize` check.
        state += 1;
    }
}

contract NestedMock {
    Mock private inner;

    constructor(Mock _inner) {
        inner = _inner;
    }

    function sum() public view returns (uint256) {
        return inner.numberA() + inner.numberB();
    }
}

contract NestedMockDelegateCall {
    Mock private inner;

    constructor(Mock _inner) {
        inner = _inner;
    }

    function sum() public returns (uint256) {
        (, bytes memory dataA) = address(inner).delegatecall(abi.encodeWithSelector(Mock.numberA.selector));
        (, bytes memory dataB) = address(inner).delegatecall(abi.encodeWithSelector(Mock.numberB.selector));
        return abi.decode(dataA, (uint256)) + abi.decode(dataB, (uint256));
    }
}

contract MockCallTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testMockGetters() public {
        vm.pvm(true);
        Mock target = new Mock();

        // pre-mock
        assertEq(target.numberA(), 1);
        assertEq(target.numberB(), 2);

        vm.mockCall(address(target), abi.encodeWithSelector(target.numberB.selector), abi.encode(10));

        // post-mock
        assertEq(target.numberA(), 1);
        assertEq(target.numberB(), 10);
    }

    function testMockNested() public {
        vm.pvm(true);

        Mock inner = new Mock();
        NestedMock target = new NestedMock(inner);

        // pre-mock
        assertEq(target.sum(), 3);
        console.log("SUM BEFORE MOCK", address(inner));
        vm.mockCall(address(inner), abi.encodeWithSelector(inner.numberB.selector), abi.encode(9));

        // post-mock
        assertEq(target.sum(), 10);
    }

    // Ref: https://github.com/foundry-rs/foundry/issues/8066
    function testMockNestedDelegate() public {
        vm.pvm(true);

        Mock inner = new Mock();
        NestedMockDelegateCall target = new NestedMockDelegateCall(inner);

        assertEq(target.sum(), 3);

        vm.mockCall(address(inner), abi.encodeWithSelector(inner.numberB.selector), abi.encode(9));

        assertEq(target.sum(), 10);
    }

    function testMockSelector() public {
        vm.pvm(true);

        Mock target = new Mock();
        assertEq(target.add(5, 5), 10);

        vm.mockCall(address(target), abi.encodeWithSelector(target.add.selector), abi.encode(11));

        assertEq(target.add(5, 5), 11);
    }

    function testMockCalldata() public {
        vm.pvm(true);

        Mock target = new Mock();
        assertEq(target.add(5, 5), 10);
        assertEq(target.add(6, 4), 10);

        vm.mockCall(address(target), abi.encodeWithSelector(target.add.selector, 5, 5), abi.encode(11));

        assertEq(target.add(5, 5), 11);
        assertEq(target.add(6, 4), 10);
    }

    function testClearMockedCalls() public {
        vm.pvm(true);

        Mock target = new Mock();

        vm.mockCall(address(target), abi.encodeWithSelector(target.numberB.selector), abi.encode(10));

        assertEq(target.numberA(), 1);
        assertEq(target.numberB(), 10);

        vm.clearMockedCalls();

        assertEq(target.numberA(), 1);
        assertEq(target.numberB(), 2);
    }

    function testMockCallMultiplePartialMatch() public {
        vm.pvm(true);

        Mock mock = new Mock();

        vm.mockCall(address(mock), abi.encodeWithSelector(mock.add.selector), abi.encode(10));
        vm.mockCall(address(mock), abi.encodeWithSelector(mock.add.selector, 2), abi.encode(20));
        vm.mockCall(address(mock), abi.encodeWithSelector(mock.add.selector, 2, 3), abi.encode(30));

        assertEq(mock.add(1, 2), 10);
        assertEq(mock.add(2, 2), 20);
        assertEq(mock.add(2, 3), 30);
    }

    function testMockCallWithValue() public {
        vm.pvm(true);
        Mock mock = new Mock();

        vm.mockCall(address(mock), 10, abi.encodeWithSelector(mock.pay.selector), abi.encode(10));

        assertEq(mock.pay{value: 10}(1), 10);
        assertEq(mock.pay(1), 1);

        for (uint256 i = 0; i < 100; i++) {
            vm.mockCall(address(mock), i, abi.encodeWithSelector(mock.pay.selector), abi.encode(i * 2));
        }

        assertEq(mock.pay(1), 0);
        assertEq(mock.pay{value: 10}(1), 20);
        assertEq(mock.pay{value: 50}(1), 100);
    }

    function testMockCallWithValueCalldataPrecedence() public {
        vm.pvm(true);

        Mock mock = new Mock();

        vm.mockCall(address(mock), 10, abi.encodeWithSelector(mock.pay.selector), abi.encode(10));
        vm.mockCall(address(mock), abi.encodeWithSelector(mock.pay.selector, 2), abi.encode(2));

        assertEq(mock.pay{value: 10}(1), 10);
        assertEq(mock.pay{value: 10}(2), 2);
        assertEq(mock.pay(2), 2);
    }

    function testMockCallEmptyAccount() public {
        vm.pvm(true);

        Mock mock = Mock(address(100));

        vm.mockCall(address(mock), abi.encodeWithSelector(mock.add.selector), abi.encode(10));
        vm.mockCall(address(mock), mock.noReturnValue.selector, abi.encode());

        assertEq(mock.add(1, 2), 10);
        mock.noReturnValue();
    }
}

contract MockCallRevertTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    error TestError(bytes msg);

    bytes constant ERROR_MESSAGE = "ERROR_MESSAGE";

    function testMockGettersRevert() public {
        vm.pvm(true);

        Mock target = new Mock();

        // pre-mock
        assertEq(target.numberA(), 1);
        assertEq(target.numberB(), 2);

        vm.mockCallRevert(address(target), target.numberB.selector, ERROR_MESSAGE);

        // post-mock
        assertEq(target.numberA(), 1);
        try target.numberB() {
            revert();
        } catch (bytes memory err) {
            require(keccak256(err) == keccak256(ERROR_MESSAGE));
        }
    }

    function testMockRevertWithCustomError() public {
        vm.pvm(true);

        Mock target = new Mock();

        assertEq(target.numberA(), 1);
        assertEq(target.numberB(), 2);

        bytes memory customError = abi.encodeWithSelector(TestError.selector, ERROR_MESSAGE);

        vm.mockCallRevert(address(target), abi.encodeWithSelector(target.numberB.selector), customError);

        assertEq(target.numberA(), 1);
        try target.numberB() {
            revert();
        } catch (bytes memory err) {
            require(keccak256(err) == keccak256(customError));
        }
    }

    function testMockNestedRevert() public {
        vm.pvm(true);
        Mock inner = new Mock();
        NestedMock target = new NestedMock(inner);

        assertEq(target.sum(), 3);

        vm.mockCallRevert(address(inner), abi.encodeWithSelector(inner.numberB.selector), ERROR_MESSAGE);

        try target.sum() {
            revert();
        } catch (bytes memory err) {
            require(keccak256(err) == keccak256(ERROR_MESSAGE));
        }
    }

    function testMockCalldataRevert() public {
        vm.pvm(true);

        Mock target = new Mock();
        assertEq(target.add(5, 5), 10);
        assertEq(target.add(6, 4), 10);

        vm.mockCallRevert(address(target), abi.encodeWithSelector(target.add.selector, 5, 5), ERROR_MESSAGE);

        assertEq(target.add(6, 4), 10);

        try target.add(5, 5) {
            revert();
        } catch (bytes memory err) {
            require(keccak256(err) == keccak256(ERROR_MESSAGE));
        }
    }

    function testClearMockRevertedCalls() public {
        vm.pvm(true);

        Mock target = new Mock();

        vm.mockCallRevert(address(target), abi.encodeWithSelector(target.numberB.selector), ERROR_MESSAGE);

        vm.clearMockedCalls();

        assertEq(target.numberA(), 1);
        assertEq(target.numberB(), 2);
    }

    function testMockCallRevertPartialMatch() public {
        vm.pvm(true);

        Mock mock = new Mock();

        vm.mockCallRevert(address(mock), abi.encodeWithSelector(mock.add.selector, 2), ERROR_MESSAGE);

        assertEq(mock.add(1, 2), 3);

        try mock.add(2, 3) {
            revert();
        } catch (bytes memory err) {
            require(keccak256(err) == keccak256(ERROR_MESSAGE));
        }
    }

    function testMockCallRevertWithValue() public {
        vm.pvm(true);

        Mock mock = new Mock();

        vm.mockCallRevert(address(mock), 10, abi.encodeWithSelector(mock.pay.selector), ERROR_MESSAGE);

        assertEq(mock.pay(1), 1);
        assertEq(mock.pay(2), 2);

        try mock.pay{value: 10}(1) {
            revert();
        } catch (bytes memory err) {
            require(keccak256(err) == keccak256(ERROR_MESSAGE));
        }
    }

    function testMockCallResetsMockCallRevert() public {
        vm.pvm(true);

        Mock mock = new Mock();

        vm.mockCallRevert(address(mock), abi.encodeWithSelector(mock.add.selector), ERROR_MESSAGE);

        vm.mockCall(address(mock), abi.encodeWithSelector(mock.add.selector), abi.encode(5));
        assertEq(mock.add(2, 3), 5);
    }

    function testMockCallRevertResetsMockCall() public {
        vm.pvm(true);

        Mock mock = new Mock();

        vm.mockCall(address(mock), abi.encodeWithSelector(mock.add.selector), abi.encode(5));
        assertEq(mock.add(2, 3), 5);

        vm.mockCallRevert(address(mock), abi.encodeWithSelector(mock.add.selector), ERROR_MESSAGE);

        try mock.add(2, 3) {
            revert();
        } catch (bytes memory err) {
            require(keccak256(err) == keccak256(ERROR_MESSAGE));
        }
    }

    function testMockCallRevertWithCall() public {
        vm.pvm(true);

        Mock mock = new Mock();

        bytes memory customError = abi.encodeWithSelector(TestError.selector, ERROR_MESSAGE);

        vm.mockCallRevert(address(mock), abi.encodeWithSelector(mock.add.selector), customError);

        (bool success, bytes memory data) = address(mock).call(abi.encodeWithSelector(Mock.add.selector, 2, 3));
        assertEq(success, false);
        assertEq(data, customError);
    }

    function testMockCallEmptyAccount() public {
        vm.pvm(true);

        Mock mock = Mock(address(100));

        vm.mockCallRevert(address(mock), abi.encodeWithSelector(mock.add.selector), ERROR_MESSAGE);

        try mock.add(2, 3) {
            revert();
        } catch (bytes memory err) {
            require(keccak256(err) == keccak256(ERROR_MESSAGE));
        }
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

Ran 11 tests for src/MockCall.t.sol:MockCallRevertTest
[PASS] testClearMockRevertedCalls() ([GAS])
[PASS] testMockCallEmptyAccount() ([GAS])
[PASS] testMockCallResetsMockCallRevert() ([GAS])
[PASS] testMockCallRevertPartialMatch() ([GAS])
[PASS] testMockCallRevertResetsMockCall() ([GAS])
[PASS] testMockCallRevertWithCall() ([GAS])
[PASS] testMockCallRevertWithValue() ([GAS])
[PASS] testMockCalldataRevert() ([GAS])
[PASS] testMockGettersRevert() ([GAS])
[PASS] testMockNestedRevert() ([GAS])
[PASS] testMockRevertWithCustomError() ([GAS])
Suite result: ok. 11 passed; 0 failed; 0 skipped; [ELAPSED]

Ran 10 tests for src/MockCall.t.sol:MockCallTest
[PASS] testClearMockedCalls() ([GAS])
[PASS] testMockCallEmptyAccount() ([GAS])
[PASS] testMockCallMultiplePartialMatch() ([GAS])
[PASS] testMockCallWithValue() ([GAS])
[PASS] testMockCallWithValueCalldataPrecedence() ([GAS])
[PASS] testMockCalldata() ([GAS])
[PASS] testMockGetters() ([GAS])
[PASS] testMockNested() ([GAS])
[PASS] testMockNestedDelegate() ([GAS])
[PASS] testMockSelector() ([GAS])
Suite result: ok. 10 passed; 0 failed; 0 skipped; [ELAPSED]

Ran 2 test suites [ELAPSED]: 21 tests passed, 0 failed, 0 skipped (21 total tests)

"#]]);
});
