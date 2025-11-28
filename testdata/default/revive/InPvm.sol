// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

library InPvmLib {
    function _inPvm() internal view returns (bool) {
        // In PVM mode, we can use a simple heuristic
        // You could also use a specific precompile or system contract call
        // For now, we'll use a simple approach
        try this._pvmCheck() returns (bool result) {
            return result;
        } catch {
            return false;
        }
    }

    function _pvmCheck() external pure returns (bool) {
        // This is a placeholder - in actual PVM, you might check for
        // specific precompiles or behaviors unique to PVM
        return true;
    }
}

abstract contract InPvm {
    modifier inPvm() {
        require(InPvmLib._inPvm(), "must be executed in PVM");
        _;
    }
}

abstract contract DeployOnlyInPvm is InPvm {
    constructor() {
        require(InPvmLib._inPvm(), "must be deployed in PVM");
    }
}
