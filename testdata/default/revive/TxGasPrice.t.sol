// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "ds-test/test.sol";
import "cheats/Vm.sol";

contract TxGasPriceTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testTxGasPriceWorks() public {
        // Set a new gas price
        uint256 newGasPrice = 100_000_000_000; // 100 gwei
        vm.txGasPrice(newGasPrice);
        
        // Verify the gas price was updated
        assertEq(tx.gasprice, newGasPrice, "gas price should be updated");
    }

    function testTxGasPriceWorksWithZero() public {
        // Set gas price to zero
        vm.txGasPrice(0);
        
        // Verify the gas price was updated to zero
        assertEq(tx.gasprice, 0, "gas price should be zero");
    }

    function testTxGasPriceWorksWithLargeValue() public {
        uint256 largeGasPrice = 1_000_000_000_000_000; // 1 million gwei
        vm.txGasPrice(largeGasPrice);
        
        // Verify the gas price was updated
        assertEq(tx.gasprice, largeGasPrice, "gas price should be updated to large value");
    }

    function testTxGasPriceWorksInBothModes() public {
        // Test in EVM mode
        vm.pvm(false);
        uint256 evmGasPrice = 50_000_000_000; // 50 gwei
        vm.txGasPrice(evmGasPrice);
        assertEq(tx.gasprice, evmGasPrice, "gas price should work in EVM mode");
        
        // Test in PVM mode
        vm.pvm(true);
        uint256 pvmGasPrice = 75_000_000_000; // 75 gwei
        vm.txGasPrice(pvmGasPrice);
        assertEq(tx.gasprice, pvmGasPrice, "gas price should work in PVM mode");
    }
}

