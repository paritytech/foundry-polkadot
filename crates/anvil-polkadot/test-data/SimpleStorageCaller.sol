pragma solidity ^0.8.0;

import "./SimpleStorage.sol";

// A contract that acts as a proxy for calls to a `SimpleStorage` contract
contract SimpleStorageCaller {
    // A state variable to hold the instantiated contract reference
    SimpleStorage public simpleStorageInstance;

    // The constructor takes the address of the deployed SimpleStorage contract
    constructor(address _storageAddress) {
        // Instantiate the SimpleStorage contract at the given address
        simpleStorageInstance = SimpleStorage(_storageAddress);
    }

    // A function that calls setValue() on the SimpleStorage contract (Sub-call 1)
    function callSetValue(uint _newValue) public {
        simpleStorageInstance.setValue(_newValue);
    }

    // A function that calls getValue() on the SimpleStorage contract (Sub-call 2)
    function callGetValue() public view returns (uint) {
        return simpleStorageInstance.getValue();
    }
}

