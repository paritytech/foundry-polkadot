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

contract MockFunctionContract {
    uint256 public a;

    function mocked_function() public {
        a = 321;
    }

    function mocked_args_function(uint256 x) public {
        a = 321 + x;
    }
}

contract ModelMockFunctionContract {
    uint256 public a;

    function mocked_function() public {
        a = 123;
    }

    function mocked_args_function(uint256 x) public {
        a = 123 + x;
    }
}

contract MockFunctionTest is DSTest {
    MockFunctionContract my_contract;
    ModelMockFunctionContract model_contract;
    Vm vm = Vm(HEVM_ADDRESS);

    function setUp() public {
        vm.pvm(true);
        my_contract = new MockFunctionContract();
        model_contract = new ModelMockFunctionContract();
    }

    function test_mockx_function() public {
        vm.mockFunction(
            address(my_contract),
            address(model_contract),
            abi.encodeWithSelector(MockFunctionContract.mocked_function.selector)
        );
        my_contract.mocked_function();
        assertEq(my_contract.a(), 123);
    }

    function test_mock_function_concrete_args() public {
        vm.mockFunction(
            address(my_contract),
            address(model_contract),
            abi.encodeWithSelector(MockFunctionContract.mocked_args_function.selector, 456)
        );
        my_contract.mocked_args_function(456);
        assertEq(my_contract.a(), 123 + 456);
        my_contract.mocked_args_function(567);
        assertEq(my_contract.a(), 321 + 567);
    }

    function test_mock_function_all_args() public {
        vm.mockFunction(
            address(my_contract),
            address(model_contract),
            abi.encodeWithSelector(MockFunctionContract.mocked_args_function.selector)
        );
        my_contract.mocked_args_function(678);
        assertEq(my_contract.a(), 123 + 678);
        my_contract.mocked_args_function(789);
        assertEq(my_contract.a(), 123 + 789);
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

Ran 3 tests for src/MockCall.t.sol:MockFunctionTest
[PASS] test_mock_function_all_args() ([GAS])
[PASS] test_mock_function_concrete_args() ([GAS])
[PASS] test_mockx_function() ([GAS])
Suite result: ok. 3 passed; 0 failed; 0 skipped; [ELAPSED]

Ran 1 test suite [ELAPSED]: 3 tests passed, 0 failed, 0 skipped (3 total tests)

"#]]);
});
