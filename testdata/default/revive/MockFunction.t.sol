pragma solidity ^0.8.18;

import "utils/DSTest.sol";
import "utils/Vm.sol";
import "utils/console.sol";

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

    // mockFunction: delegates calls AND preserves real code
    function test_mockFunction_preserves_real_code() public {
        address target = address(my_contract);
        uint256 originalSize;
        assembly {
            originalSize := extcodesize(target)
        }
        bytes32 originalHash = target.codehash;
        assertTrue(originalSize > 1, "Should have real code");

        vm.mockFunction(
            address(my_contract),
            address(model_contract),
            abi.encodeWithSelector(MockFunctionContract.mocked_function.selector)
        );

        my_contract.mocked_function();
        assertEq(my_contract.a(), 123, "Should use model behavior");

        uint256 sizeAfter;
        assembly {
            sizeAfter := extcodesize(target)
        }
        assertEq(sizeAfter, originalSize, "EXTCODESIZE preserved");
        assertEq(target.codehash, originalHash, "EXTCODEHASH preserved");
    }
}
