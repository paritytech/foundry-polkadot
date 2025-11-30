// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

library ReviveDetector {
    /// @notice The address of the Foundry VM cheatcode contract
    address constant HEVM_ADDRESS = address(bytes20(uint160(uint256(keccak256("hevm cheat code")))));

    /// @notice Detects if we're running in pallet-revive or Foundry's native EVM
    /// @return true if in pallet-revive, false if in native EVM
    function isInRevive() internal returns (bool) {
        // Try to call a cheatcode using low-level call
        // In native EVM: This succeeds (cheatcodes work)
        // In pallet-revive: This reverts (cheatcode address is just a regular address)

        // Call getBlockNumber() which has selector 0x42cbb15c
        (bool success,) = HEVM_ADDRESS.call(abi.encodeWithSignature("getBlockNumber()"));

        // If call succeeded, we're in Foundry's native EVM
        // If call failed, we're in pallet-revive
        return !success;
    }
}

abstract contract RequiresRevive {
    modifier onlyRevive() {
        require(ReviveDetector.isInRevive(), "must be executed in pallet-revive");
        _;
    }
}

abstract contract RequiresNativeEVM {
    modifier onlyNativeEVM() {
        require(!ReviveDetector.isInRevive(), "must be executed in native EVM");
        _;
    }
}
