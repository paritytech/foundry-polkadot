pragma solidity ^0.8.0;

contract ImmutableStorage {
    uint256 public immutable immutableValue;
    address public immutable immutableAddress;

    constructor(uint256 _value, address _addr) {
        immutableValue = _value;
        immutableAddress = _addr;
    }

    function getImmutableValue() public view returns (uint256) {
        return immutableValue;
    }

    function getImmutableAddress() public view returns (address) {
        return immutableAddress;
    }
}
