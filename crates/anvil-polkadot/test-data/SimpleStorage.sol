pragma solidity ^0.8.0;

contract SimpleStorage {
    // Storage slot 0
    uint256 public storedValue;
    string public storedData;

    event ValueChanged(address indexed changer, uint256 oldValue, uint256 newValue);
    event DataUpdated(address indexed updater, string data);
    
    constructor() {
        storedValue = 0;
        emit ValueChanged(address(0), 0, 0);
    }

    function setValue(uint256 _value) public {
        uint256 oldValue = storedValue;
        storedValue = _value;
        emit ValueChanged(msg.sender, oldValue, _value);
    }

    function getValue() public view returns (uint256) {
        return storedValue;
    }
    
    function setData(string memory _data) public {
        storedData = _data;
        emit DataUpdated(msg.sender, _data);
    }

    function emitBoth(uint256 _value, string memory _data) public {
        uint256 oldValue = storedValue;
        storedValue = _value;
        emit ValueChanged(msg.sender, oldValue, _value);
        
        storedData = _data;
        emit DataUpdated(msg.sender, _data);
    }
}