// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "ds-test/test.sol";
import "cheats/Vm.sol";
import {ReviveDetector, RequiresRevive} from "./ReviveDetector.sol";
import "../../default/logs/console.sol";

contract Counter is RequiresRevive {
    uint256 public number;

    function inc() public onlyRevive {
        number += 1;
    }

    function reset() public onlyRevive {
        number = 0;
    }
}

contract CounterHandler is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    uint256 public incCounter;
    uint256 public resetCounter;
    bool public isResetLast;
    Counter public counter;

    constructor(Counter _counter) {
        counter = _counter;
    }

    function inc() public {
        console.log("inc");
        incCounter += 1;
        isResetLast = false;

        vm.deal(tx.origin, 1 ether); // ensure caller has funds
        counter.inc();
    }

    function reset() public {
        console.log("reset");
        resetCounter += 1;
        isResetLast = true;

        vm.deal(tx.origin, 1 ether); // ensure caller has funds
        counter.reset();
    }
}

// partial from forge-std/StdInvariant.sol
abstract contract StdInvariant {
    struct FuzzSelector {
        address addr;
        bytes4[] selectors;
    }

    address[] internal _targetedContracts;

    function targetContracts() public view returns (address[] memory) {
        return _targetedContracts;
    }

    FuzzSelector[] internal _targetedSelectors;

    function targetSelectors() public view returns (FuzzSelector[] memory) {
        return _targetedSelectors;
    }

    address[] internal _targetedSenders;

    function targetSenders() public view returns (address[] memory) {
        return _targetedSenders;
    }
}

contract PolkadotSkipInvariantWithHandler is DSTest, StdInvariant {
    Vm constant vm = Vm(HEVM_ADDRESS);
    Counter cnt;
    CounterHandler handler;

    function setUp() public {
        vm.pvm(true);
        cnt = new Counter();
        vm.makePersistent(address(cnt));

        vm.polkadotSkip();
        handler = new CounterHandler(cnt);

        // add the handler selectors to the fuzzing targets
        bytes4[] memory selectors = new bytes4[](2);
        selectors[0] = CounterHandler.inc.selector;
        selectors[1] = CounterHandler.reset.selector;

        _targetedContracts.push(address(handler));
        _targetedSelectors.push(FuzzSelector({addr: address(handler), selectors: selectors}));
    }

    /// forge-config: default.invariant.fail-on-revert = true
    function invariant_ghostVariables() external {
        uint256 num = cnt.number();

        if (handler.resetCounter() == 0) {
            assert(handler.incCounter() == num);
        } else if (handler.isResetLast()) {
            assert(num == 0);
        } else {
            assert(num != 0);
        }
    }
}

contract PolkadotSkipInvariantWithoutHandler is DSTest, StdInvariant {
    Vm constant vm = Vm(HEVM_ADDRESS);
    Counter cnt;

    uint256 constant dealAmount = 1 ether;

    function setUp() public {
        vm.pvm(true);
        cnt = new Counter();

        // so we can fund them ahead of time for fees
        _targetedSenders.push(address(65536 + 1));
        _targetedSenders.push(address(65536 + 12));
        _targetedSenders.push(address(65536 + 123));
        _targetedSenders.push(address(65536 + 1234));

        for (uint256 i = 0; i < _targetedSenders.length; i++) {
            vm.deal(_targetedSenders[i], dealAmount);
        }

        // add the counter selectors to the fuzzing targets
        bytes4[] memory selectors = new bytes4[](2);
        selectors[0] = Counter.inc.selector;
        selectors[1] = Counter.reset.selector;

        _targetedContracts.push(address(cnt));
        _targetedSelectors.push(FuzzSelector({addr: address(cnt), selectors: selectors}));
    }

    /// forge-config: default.invariant.fail-on-revert = true
    function invariant_itWorks() external {}
}
