use foundry_compilers::artifacts::EvmVersion;

forgetest!(counter_test, |prj, cmd| {
    prj.insert_ds_test();
    prj.insert_vm();
    prj.insert_console();
    prj.add_source(
        "Counter.sol",
        r#"
    // SPDX-License-Identifier: UNLICENSED
    pragma solidity ^0.8.13;

    contract Counter {
        uint256 public number = 0;

        function setNumber(uint256 newNumber) public {
            number = newNumber;
        }

        function increment() public {
            number = number + 1;
        }
    }
    "#,
    )
    .unwrap();
    prj.add_source(
        "CounterTest.t.sol",
        r#"
import "./test.sol";
import "./Vm.sol";
import {Counter} from "./Counter.sol";
import {console} from "./console.sol";

contract CounterTest is DSTest {
  Vm constant vm = Vm(HEVM_ADDRESS);
  Counter public counter;

  function setUp() public {
    counter = new Counter(); 
    counter.setNumber(5);
    assertEq(counter.number(), 5);
  }

  function test_Increment() public {
      assertEq(counter.number(), 5);
      counter.setNumber(55); 
      assertEq(counter.number(), 55);
      counter.increment(); 
      assertEq(counter.number(), 56);
  }

  function testFuzz_SetNumber(uint256 x) public {
      assertEq(counter.number(), 5);
      counter.setNumber(x); 
      assertEq(counter.number(), x);
  }
  
  function testFuzz_SetNumber2(uint256 x) public {
    assertEq(counter.number(), 5);
    counter.setNumber(x); 
    assertEq(counter.number(), x);
  }

  function testFuzz_SetNumber3(uint256 x) public {
    assertEq(counter.number(), 5);
    counter.setNumber(x); 
    assertEq(counter.number(), x);
  }
}
"#,
    )
    .unwrap();
    prj.update_config(|config| config.evm_version = EvmVersion::Cancun);

    let res = cmd.args(["test", "--resolc", "-vvv", "--polkadot"]).assert();
    res.stderr_eq(str![""]).stdout_eq(str![[r#"
[COMPILING_FILES] with [SOLC_VERSION]
[SOLC_VERSION] [ELAPSED]
Compiler run successful!
[COMPILING_FILES] with [RESOLC_VERSION]
[RESOLC_VERSION] [ELAPSED]
Compiler run successful!

Ran 4 tests for src/CounterTest.t.sol:CounterTest
[PASS] testFuzz_SetNumber(uint256) (runs: 256, [AVG_GAS])
[PASS] testFuzz_SetNumber2(uint256) (runs: 256, [AVG_GAS])
[PASS] testFuzz_SetNumber3(uint256) (runs: 256, [AVG_GAS])
[PASS] test_Increment() ([GAS])
Suite result: ok. 4 passed; 0 failed; 0 skipped; [ELAPSED]

Ran 1 test suite [ELAPSED]: 4 tests passed, 0 failed, 0 skipped (4 total tests)

"#]]);
});

forgetest!(set_get_nonce_revive, |prj, cmd| {
    prj.insert_ds_test();
    prj.insert_vm();
    prj.insert_console();
    prj.add_source(
        "SetNonce.t.sol",
        r#"
import "./test.sol";
import "./Vm.sol";
import {console} from "./console.sol";

contract SetNonce is DSTest {
  Vm constant vm = Vm(HEVM_ADDRESS);

  function test_SetNonce() public {
      uint64 original = vm.getNonce(address(this));
      vm.setNonce(address(this), 64);
      uint64 newValue = vm.getNonce(address(this));
      assert(original != newValue);
      assertEq(newValue, 64);
  }
}
"#,
    )
    .unwrap();
    prj.update_config(|config| config.evm_version = EvmVersion::Cancun);

    let res = cmd.args(["test", "--resolc", "-vvv", "--polkadot"]).assert_success();
    res.stderr_eq(str![""]).stdout_eq(str![[r#"
[COMPILING_FILES] with [SOLC_VERSION]
[SOLC_VERSION] [ELAPSED]
Compiler run successful!
[COMPILING_FILES] with [RESOLC_VERSION]
[RESOLC_VERSION] [ELAPSED]
Compiler run successful!

Ran 1 test for src/SetNonce.t.sol:SetNonce
[PASS] test_SetNonce() ([GAS])
Suite result: ok. 1 passed; 0 failed; 0 skipped; [ELAPSED]

Ran 1 test suite [ELAPSED]: 1 tests passed, 0 failed, 0 skipped (1 total tests)

"#]]);
});

forgetest!(roll_revive, |prj, cmd| {
    prj.insert_ds_test();
    prj.insert_vm();
    prj.insert_console();
    prj.add_source(
        "Roll.t.sol",
        r#"
import "./test.sol";
import "./Vm.sol";
import {console} from "./console.sol";

contract Roll is DSTest {
  Vm constant vm = Vm(HEVM_ADDRESS);

  function test_Roll() public {
      uint256 original = block.number;
      vm.roll(10);
      uint256 newValue = block.number;
      assert(original != newValue);
      assertEq(newValue, 10);
  }
}
"#,
    )
    .unwrap();

    let res = cmd.args(["test", "--resolc", "-vvv", "--polkadot"]).assert_success();
    res.stderr_eq(str![""]).stdout_eq(str![[r#"
[COMPILING_FILES] with [SOLC_VERSION]
[SOLC_VERSION] [ELAPSED]
Compiler run successful!
[COMPILING_FILES] with [RESOLC_VERSION]
[RESOLC_VERSION] [ELAPSED]
Compiler run successful!

Ran 1 test for src/Roll.t.sol:Roll
[PASS] test_Roll() ([GAS])
Suite result: ok. 1 passed; 0 failed; 0 skipped; [ELAPSED]

Ran 1 test suite [ELAPSED]: 1 tests passed, 0 failed, 0 skipped (1 total tests)

"#]]);
});

forgetest!(warp_revive, |prj, cmd| {
    prj.insert_ds_test();
    prj.insert_vm();
    prj.insert_console();
    prj.add_source(
        "Warp.t.sol",
        r#"
import "./test.sol";
import "./Vm.sol";
import {console} from "./console.sol";

contract Warp is DSTest {
  Vm constant vm = Vm(HEVM_ADDRESS);

  function test_Warp() public {
      uint256 original = block.timestamp;
      vm.warp(100);
      uint256 newValue = block.timestamp;
      assert(original != newValue);
      assertEq(newValue, 100);
  }
}
"#,
    )
    .unwrap();

    let res = cmd.args(["test", "--resolc", "-vvv", "--polkadot"]).assert_success();
    res.stderr_eq(str![""]).stdout_eq(str![[r#"
[COMPILING_FILES] with [SOLC_VERSION]
[SOLC_VERSION] [ELAPSED]
Compiler run successful!
[COMPILING_FILES] with [RESOLC_VERSION]
[RESOLC_VERSION] [ELAPSED]
Compiler run successful!

Ran 1 test for src/Warp.t.sol:Warp
[PASS] test_Warp() ([GAS])
Suite result: ok. 1 passed; 0 failed; 0 skipped; [ELAPSED]

Ran 1 test suite [ELAPSED]: 1 tests passed, 0 failed, 0 skipped (1 total tests)

"#]]);
});

forgetest!(deal, |prj, cmd| {
    prj.insert_ds_test();
    prj.insert_vm();
    prj.insert_console();
    prj.add_source(
        "Balance.t.sol",
        r#"
import "./test.sol";
import "./Vm.sol";
import {console} from "./console.sol";

contract Balance is DSTest {
Vm constant vm = Vm(HEVM_ADDRESS);

function test_Balance() public {
  vm.deal(address(this), 64 ether);
  uint256 newValue = address(this).balance;
  assertEq(newValue, 64 ether);
  vm.deal(address(this), 65 ether);
  uint256 newValue2 = address(this).balance;
  assertEq(newValue2, 65 ether);
}
}
"#,
    )
    .unwrap();

    let res = cmd.args(["test", "--resolc", "-vvv", "--polkadot"]).assert_success();
    res.stderr_eq(str![""]).stdout_eq(str![[r#"
[COMPILING_FILES] with [SOLC_VERSION]
[SOLC_VERSION] [ELAPSED]
Compiler run successful!
[COMPILING_FILES] with [RESOLC_VERSION]
[RESOLC_VERSION] [ELAPSED]
Compiler run successful!

Ran 1 test for src/Balance.t.sol:Balance
[PASS] test_Balance() ([GAS])
Suite result: ok. 1 passed; 0 failed; 0 skipped; [ELAPSED]

Ran 1 test suite [ELAPSED]: 1 tests passed, 0 failed, 0 skipped (1 total tests)

"#]]);
});

forgetest!(vm_load, |prj, cmd| {
    prj.insert_ds_test();
    prj.insert_vm();
    prj.insert_console();
    prj.add_source(
        "Counter.sol",
        r#"
  // SPDX-License-Identifier: UNLICENSED
  pragma solidity ^0.8.13;

  contract Counter {
      uint256 public number;

      constructor (uint256 number_) {
        number = number_;
      }
  }
  "#,
    )
    .unwrap();
    prj.add_source(
        "Load.t.sol",
        r#"
import "./test.sol";
import "./Vm.sol";
import {console} from "./console.sol";
import {Counter} from "./Counter.sol";

contract Load is DSTest {
Vm constant vm = Vm(HEVM_ADDRESS);

function testFuzz_Load(uint256 x) public {
    address counter = address(new Counter(x));
    bytes32 res = vm.load(counter, bytes32(uint256(0)));
    assertEq(uint256(res), x);
}
}
"#,
    )
    .unwrap();

    let res = cmd.args(["test", "--resolc", "--polkadot", "-vvv"]).assert_success();
    res.stderr_eq(str![""]).stdout_eq(str![[r#"
[COMPILING_FILES] with [SOLC_VERSION]
[SOLC_VERSION] [ELAPSED]
Compiler run successful!
[COMPILING_FILES] with [RESOLC_VERSION]
[RESOLC_VERSION] [ELAPSED]
Compiler run successful!

Ran 1 test for src/Load.t.sol:Load
[PASS] testFuzz_Load(uint256) (runs: 256, [AVG_GAS])
Suite result: ok. 1 passed; 0 failed; 0 skipped; [ELAPSED]

Ran 1 test suite [ELAPSED]: 1 tests passed, 0 failed, 0 skipped (1 total tests)

"#]]);
});

// Test --polkadot flag: EVM execution on pallet-revive backend
forgetest!(polkadot_evm_backend, |prj, cmd| {
    prj.insert_ds_test();
    prj.insert_vm();
    prj.add_source(
        "Counter.sol",
        r#"
// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
contract Counter {
    uint256 public number;
    constructor(uint256 _initial) {
        number = _initial;
    }
    function increment() public {
        number = number + 1;
    }
    function getNumber() public view returns (uint256) {
        return number;
    }
}
"#,
    )
    .unwrap();

    prj.add_source(
        "CounterTest.t.sol",
        r#"
import "./test.sol";
import {Counter} from "./Counter.sol";
contract CounterTest is DSTest {
    function test_PolkadotEVMBackend() public {
        // This test runs EVM bytecode on pallet-revive EVM backend
        Counter counter = new Counter(42);
        assertEq(counter.getNumber(), 42);
        counter.increment();
        assertEq(counter.getNumber(), 43);
        counter.increment();
        assertEq(counter.getNumber(), 44);
    }
}
"#,
    )
    .unwrap();

    // Test with --polkadot flag (EVM backend on pallet-revive)
    cmd.args(["test", "--polkadot", "-vvv"]).assert_success();
});

forgetest!(trace_counter_test, |prj, cmd| {
    prj.insert_ds_test();
    prj.insert_vm();
    prj.insert_console();
    prj.add_source(
        "Counter.sol",
        r#"
  // SPDX-License-Identifier: UNLICENSED
  pragma solidity ^0.8.13;

  contract Counter {
      uint256 public number = 0;
      event Increment(uint256 result);
      event SetNumber(uint256 result);
      error Revert(string text);

      function setNumber(uint256 newNumber) public {
          number = newNumber;
          emit SetNumber(number);
      }

      function failed_call() public pure {
        revert Revert("failure");
      }

      function increment() public {
          number = number + 1;
          emit Increment(number);

      }
  }
  "#,
    )
    .unwrap();
    prj.add_source(
        "CounterTest.t.sol",
        r#"
import "./test.sol";
import "./Vm.sol";
import {Counter} from "./Counter.sol";
import {console} from "./console.sol";

contract CounterTest is DSTest {
Vm constant vm = Vm(HEVM_ADDRESS);
Counter public counter;

function setUp() public {
  counter = new Counter(); 
  vm.expectEmit();
  emit Counter.SetNumber(5);

  counter.setNumber(5);
  assertEq(counter.number(), 5);
}

function test_Increment() public {
    assertEq(counter.number(), 5);
    counter.setNumber(55); 
    assertEq(counter.number(), 55);
    counter.increment(); 
    assertEq(counter.number(), 56);
}

function test_expectRevert() public {
  vm.expectRevert(abi.encodeWithSelector(Counter.Revert.selector, "failure"));
  counter.failed_call();
}
}
"#,
    )
    .unwrap();
    prj.update_config(|config| config.evm_version = EvmVersion::Cancun);

    let res = cmd.args(["test", "--resolc", "--polkadot", "-vvvvv"]).assert_success();
    res.stderr_eq("").stdout_eq(str![[r#"
[COMPILING_FILES] with [SOLC_VERSION]
[SOLC_VERSION] [ELAPSED]
Compiler run successful!
[COMPILING_FILES] with [RESOLC_VERSION]
[RESOLC_VERSION] [ELAPSED]
Compiler run successful!

Ran 2 tests for src/CounterTest.t.sol:CounterTest
[PASS] test_Increment() ([GAS])
Traces:
  [765075403] CounterTest::setUp()
    ├─ [262294819] → new <unknown>@0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC
    │   └─ ← [Return] 7404 bytes of code
    ├─ [0] VM::expectEmit()
    │   └─ ← [Return]
    ├─ emit SetNumber(result: 5)
    ├─ [385250826] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::setNumber(5)
    │   ├─ emit SetNumber(result: 5)
    │   └─ ← [Stop]
    ├─ [117489011] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::number() [staticcall]
    │   └─ ← [Return] 5
    └─ ← [Stop]

  [737726031] CounterTest::test_Increment()
    ├─ [117489011] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::number() [staticcall]
    │   └─ ← [Return] 5
    ├─ [385250826] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::setNumber(55)
    │   ├─ emit SetNumber(result: 55)
    │   └─ ← [Stop]
    ├─ [117489011] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::number() [staticcall]
    │   └─ ← [Return] 55
    ├─ [0] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::increment()
    │   ├─ emit Increment(result: 56)
    │   └─ ← [Stop]
    ├─ [117489011] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::number() [staticcall]
    │   └─ ← [Return] 56
    └─ ← [Stop]

[PASS] test_expectRevert() ([GAS])
Traces:
  [765075403] CounterTest::setUp()
    ├─ [262294819] → new <unknown>@0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC
    │   └─ ← [Return] 7404 bytes of code
    ├─ [0] VM::expectEmit()
    │   └─ ← [Return]
    ├─ emit SetNumber(result: 5)
    ├─ [385250826] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::setNumber(5)
    │   ├─ emit SetNumber(result: 5)
    │   └─ ← [Stop]
    ├─ [117489011] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::number() [staticcall]
    │   └─ ← [Return] 5
    └─ ← [Stop]

  [56930227] CounterTest::test_expectRevert()
    ├─ [0] VM::expectRevert(custom error 0xf28dceb3: 0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000006456941a80000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000076661696c7572650000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000)
    │   └─ ← [Return]
    ├─ [56921388] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::failed_call() [staticcall]
    │   └─ ← [Revert] Revert("failure")
    └─ ← [Stop]

Suite result: ok. 2 passed; 0 failed; 0 skipped; [ELAPSED]

Ran 1 test suite [ELAPSED]: 2 tests passed, 0 failed, 0 skipped (2 total tests)

"#]]);
});

forgetest!(record_rw, |prj, cmd| {
    prj.insert_ds_test();
    prj.insert_vm();
    prj.insert_console();
    prj.add_source(
        "Contracts.sol",
        r#"
    
    pragma solidity ^0.8.18;

contract RecordAccess {
    function record(NestedRecordAccess target) public {
        assembly {
            sstore(1, add(sload(1), 1))
        }

        target.record();
    }
}

contract NestedRecordAccess {
    function record() public {
        assembly {
            sstore(2, add(sload(2), 1))
        }
    }
}
"#,
    )
    .unwrap();
    prj.add_source(
        "Test.t.sol",
        r#"
        pragma solidity ^0.8.18;
        import "./test.sol";
        import "./Vm.sol";
        import "./Contracts.sol";
        import {console} from "./console.sol";
contract RecordTest is DSTest {
  Vm constant vm = Vm(HEVM_ADDRESS);

  function testRecordAccess() public {
      RecordAccess target = new RecordAccess();
      NestedRecordAccess inner = new NestedRecordAccess();
      // Start recording
      vm.record();
      target.record(inner);

      // Verify Records
      (bytes32[] memory reads, bytes32[] memory writes) = vm.accesses(address(target));
      (bytes32[] memory innerReads, bytes32[] memory innerWrites) = vm.accesses(address(inner));

      assertEq(reads.length, 2, "number of reads is incorrect");
      assertEq(reads[0], bytes32(uint256(1)), "key for read 0 is incorrect");
      assertEq(reads[1], bytes32(uint256(1)), "key for read 1 is incorrect");

      assertEq(writes.length, 1, "number of writes is incorrect");
      assertEq(writes[0], bytes32(uint256(1)), "key for write is incorrect");

      assertEq(innerReads.length, 2, "number of nested reads is incorrect");
      assertEq(innerReads[0], bytes32(uint256(2)), "key for nested read 0 is incorrect");
      assertEq(innerReads[1], bytes32(uint256(2)), "key for nested read 1 is incorrect");

      assertEq(innerWrites.length, 1, "number of nested writes is incorrect");
      assertEq(innerWrites[0], bytes32(uint256(2)), "key for nested write is incorrect");
  }

  function testStopRecordAccess() public {
    RecordAccess target = new RecordAccess();
    NestedRecordAccess inner = new NestedRecordAccess();
    // Start recording
    vm.record();
    target.record(inner);

      // Verify Records
      (bytes32[] memory reads, bytes32[] memory writes) = vm.accesses(address(target));

      assertEq(reads.length, 2, "number of reads is incorrect");
      assertEq(reads[0], bytes32(uint256(1)), "key for read 0 is incorrect");
      assertEq(reads[1], bytes32(uint256(1)), "key for read 1 is incorrect");

      assertEq(writes.length, 1, "number of writes is incorrect");
      assertEq(writes[0], bytes32(uint256(1)), "key for write is incorrect");

      vm.stopRecord();
      target.record(inner);

      // Verify that there are no new Records
      (reads, writes) = vm.accesses(address(target));

      assertEq(reads.length, 2, "number of reads is incorrect");
      assertEq(reads[0], bytes32(uint256(1)), "key for read 0 is incorrect");
      assertEq(reads[1], bytes32(uint256(1)), "key for read 1 is incorrect");

      assertEq(writes.length, 1, "number of writes is incorrect");
      assertEq(writes[0], bytes32(uint256(1)), "key for write is incorrect");

      vm.record();
      vm.stopRecord();

      // verify reset all records
      (reads, writes) = vm.accesses(address(target));

      assertEq(reads.length, 0, "number of reads is incorrect");
      assertEq(writes.length, 0, "number of writes is incorrect");
  }
}

    "#,
    )
    .unwrap();
    prj.update_config(|config| config.evm_version = EvmVersion::Cancun);

    let res = cmd.args(["test", "--resolc", "--polkadot", "-vvvvv"]).assert_success();
    res.stdout_eq(str![[r#"
[COMPILING_FILES] with [SOLC_VERSION]
[SOLC_VERSION] [ELAPSED]
Compiler run successful!
[COMPILING_FILES] with [RESOLC_VERSION]
[RESOLC_VERSION] [ELAPSED]
Compiler run successful!

Ran 2 tests for src/Test.t.sol:RecordTest
[PASS] testRecordAccess() ([GAS])
Traces:
  [961089406] RecordTest::testRecordAccess()
    ├─ [16788608] → new <unknown>@0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC
    │   └─ ← [Return] 4095 bytes of code
    ├─ [16788608] → new <unknown>@0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f
    │   └─ ← [Return] 2182 bytes of code
    ├─ [0] VM::record()
    │   └─ ← [Return]
    ├─ [927440089] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::record(0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f)
    │   ├─ [0] 0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f::record()
    │   │   └─ ← [Return]
    │   └─ ← [Stop]
    ├─ [0] VM::accesses(0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC)
    │   └─ ← [Return] [0x0000000000000000000000000000000000000000000000000000000000000001, 0x0000000000000000000000000000000000000000000000000000000000000001], [0x0000000000000000000000000000000000000000000000000000000000000001]
    ├─ [0] VM::accesses(0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f)
    │   └─ ← [Return] [0x0000000000000000000000000000000000000000000000000000000000000002, 0x0000000000000000000000000000000000000000000000000000000000000002], [0x0000000000000000000000000000000000000000000000000000000000000002]
    └─ ← [Stop]

[PASS] testStopRecordAccess() ([GAS])
Traces:
  [961093272] RecordTest::testStopRecordAccess()
    ├─ [16788608] → new <unknown>@0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC
    │   └─ ← [Return] 4095 bytes of code
    ├─ [16788608] → new <unknown>@0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f
    │   └─ ← [Return] 2182 bytes of code
    ├─ [0] VM::record()
    │   └─ ← [Return]
    ├─ [927440089] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::record(0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f)
    │   ├─ [0] 0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f::record()
    │   │   └─ ← [Return]
    │   └─ ← [Stop]
    ├─ [0] VM::accesses(0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC)
    │   └─ ← [Return] [0x0000000000000000000000000000000000000000000000000000000000000001, 0x0000000000000000000000000000000000000000000000000000000000000001], [0x0000000000000000000000000000000000000000000000000000000000000001]
    ├─ [0] VM::stopRecord()
    │   └─ ← [Return]
    ├─ [0] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::record(0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f)
    │   ├─ [0] 0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f::record()
    │   │   └─ ← [Return]
    │   └─ ← [Stop]
    ├─ [0] VM::accesses(0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC)
    │   └─ ← [Return] [0x0000000000000000000000000000000000000000000000000000000000000001, 0x0000000000000000000000000000000000000000000000000000000000000001], [0x0000000000000000000000000000000000000000000000000000000000000001]
    ├─ [0] VM::record()
    │   └─ ← [Return]
    ├─ [0] VM::stopRecord()
    │   └─ ← [Return]
    ├─ [0] VM::accesses(0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC)
    │   └─ ← [Return] [], []
    └─ ← [Stop]

Suite result: ok. 2 passed; 0 failed; 0 skipped; [ELAPSED]

Ran 1 test suite [ELAPSED]: 2 tests passed, 0 failed, 0 skipped (2 total tests)

"#]]);
});

forgetest!(record_logs, |prj, cmd| {
    prj.insert_ds_test();
    prj.insert_vm();
    prj.insert_console();
    prj.add_source(
        "Contracts.sol",
        r#"
  
  pragma solidity ^0.8.18;

  contract Emitter {
    event LogAnonymous(bytes data) anonymous;

    event LogTopic0(bytes data);

    event LogTopic1(uint256 indexed topic1, bytes data);

    event LogTopic12(uint256 indexed topic1, uint256 indexed topic2, bytes data);

    event LogTopic123(uint256 indexed topic1, uint256 indexed topic2, uint256 indexed topic3, bytes data);

    function emitAnonymousEvent(bytes memory data) public {
        emit LogAnonymous(data);
    }

    function emitEvent(bytes memory data) public {
        emit LogTopic0(data);
    }

    function emitEvent(uint256 topic1, bytes memory data) public {
        emit LogTopic1(topic1, data);
    }

    function emitEvent(uint256 topic1, uint256 topic2, bytes memory data) public {
        emit LogTopic12(topic1, topic2, data);
    }

    function emitEvent(uint256 topic1, uint256 topic2, uint256 topic3, bytes memory data) public {
        emit LogTopic123(topic1, topic2, topic3, data);
    }
}

contract Emitterv2 {
    Emitter emitter = new Emitter();

    function emitEvent(uint256 topic1, uint256 topic2, uint256 topic3, bytes memory data) public {
        emitter.emitEvent(topic1, topic2, topic3, data);
    }

    function getEmitterAddr() public view returns (address) {
        return address(emitter);
    }
}
"#,
    )
    .unwrap();
    prj.add_source(
        "Test.t.sol",
        r#"
      pragma solidity ^0.8.18;
      import "./test.sol";
      import "./Vm.sol";
      import "./Contracts.sol";
      import {console} from "./console.sol";
      contract RecordLogsTest is DSTest {
        Vm constant vm = Vm(HEVM_ADDRESS);
        Emitter emitter;
        bytes32 internal seedTestData = keccak256(abi.encodePacked("Some data"));
    
        // Used on testRecordOnEmitDifferentDepths()
        event LogTopic(uint256 indexed topic1, bytes data);
    
        function setUp() public {
            emitter = new Emitter();
        }
    
        function generateTestData(uint8 n) internal returns (bytes memory) {
            bytes memory output = new bytes(n);
    
            for (uint8 i = 0; i < n; i++) {
                output[i] = seedTestData[i % 32];
                if (i % 32 == 31) {
                    seedTestData = keccak256(abi.encodePacked(seedTestData));
                }
            }
    
            return output;
        }
    
        function testRecordOffGetsNothing() public {
            emitter.emitEvent(1, 2, 3, generateTestData(48));
            Vm.Log[] memory entries = vm.getRecordedLogs();
    
            assertEq(entries.length, 0);
        }
    
        function testRecordOnNoLogs() public {
            vm.recordLogs();
            Vm.Log[] memory entries = vm.getRecordedLogs();
    
            assertEq(entries.length, 0);
        }
    
        function testRecordOnSingleLog() public {
            bytes memory testData = "Event Data in String";
    
            vm.recordLogs();
            emitter.emitEvent(1, 2, 3, testData);
            Vm.Log[] memory entries = vm.getRecordedLogs();
    
            assertEq(entries.length, 1);
            assertEq(entries[0].topics.length, 4);
            assertEq(entries[0].topics[0], keccak256("LogTopic123(uint256,uint256,uint256,bytes)"));
            assertEq(entries[0].topics[1], bytes32(uint256(1)));
            assertEq(entries[0].topics[2], bytes32(uint256(2)));
            assertEq(entries[0].topics[3], bytes32(uint256(3)));
            assertEq(abi.decode(entries[0].data, (string)), string(testData));
            assertEq(entries[0].emitter, address(emitter));
        }
    
        // TODO
        // This crashes on decoding!
        //   The application panicked (crashed).
        //   Message:  index out of bounds: the len is 0 but the index is 0
        //   Location: <local-dir>/evm/src/trace/decoder.rs:299
        function NOtestRecordOnAnonymousEvent() public {
            bytes memory testData = generateTestData(48);
    
            vm.recordLogs();
            emitter.emitAnonymousEvent(testData);
            Vm.Log[] memory entries = vm.getRecordedLogs();
    
            assertEq(entries.length, 1);
        }
    
        function testRecordOnSingleLogTopic0() public {
            bytes memory testData = generateTestData(48);
    
            vm.recordLogs();
            emitter.emitEvent(testData);
            Vm.Log[] memory entries = vm.getRecordedLogs();
    
            assertEq(entries.length, 1);
            assertEq(entries[0].topics.length, 1);
            assertEq(entries[0].topics[0], keccak256("LogTopic0(bytes)"));
            // While not a proper string, this conversion allows the comparison.
            assertEq(abi.decode(entries[0].data, (string)), string(testData));
            assertEq(entries[0].emitter, address(emitter));
        }
    
        function testEmitRecordEmit() public {
            bytes memory testData0 = generateTestData(32);
            emitter.emitEvent(1, 2, testData0);
    
            vm.recordLogs();
            bytes memory testData1 = generateTestData(16);
            emitter.emitEvent(3, testData1);
            Vm.Log[] memory entries = vm.getRecordedLogs();
    
            assertEq(entries.length, 1, "entries length");
            assertEq(entries[0].topics.length, 2);
            assertEq(entries[0].topics[0], keccak256("LogTopic1(uint256,bytes)"));
            assertEq(entries[0].topics[1], bytes32(uint256(3)));
            assertEq(abi.decode(entries[0].data, (string)), string(testData1));
            assertEq(entries[0].emitter, address(emitter));
        }
    
        function testRecordOnEmitDifferentDepths() public {
            vm.recordLogs();
    
            bytes memory testData0 = generateTestData(16);
            emit LogTopic(1, testData0);
    
            bytes memory testData1 = generateTestData(20);
            emitter.emitEvent(2, 3, testData1);
    
            bytes memory testData2 = generateTestData(24);
            Emitterv2 emitter2 = new Emitterv2();
            emitter2.emitEvent(4, 5, 6, testData2);
    
            Vm.Log[] memory entries = vm.getRecordedLogs();
    
            assertEq(entries.length, 3);
    
            assertEq(entries[0].topics.length, 2);
            assertEq(entries[0].topics[0], keccak256("LogTopic(uint256,bytes)"));
            assertEq(entries[0].topics[1], bytes32(uint256(1)));
            assertEq(abi.decode(entries[0].data, (string)), string(testData0));
            assertEq(entries[0].emitter, address(this));
    
            assertEq(entries[1].topics.length, 3);
            assertEq(entries[1].topics[0], keccak256("LogTopic12(uint256,uint256,bytes)"));
            assertEq(entries[1].topics[1], bytes32(uint256(2)));
            assertEq(entries[1].topics[2], bytes32(uint256(3)));
            assertEq(abi.decode(entries[1].data, (string)), string(testData1));
            assertEq(entries[1].emitter, address(emitter));
    
            assertEq(entries[2].topics.length, 4);
            assertEq(entries[2].topics[0], keccak256("LogTopic123(uint256,uint256,uint256,bytes)"));
            assertEq(entries[2].topics[1], bytes32(uint256(4)));
            assertEq(entries[2].topics[2], bytes32(uint256(5)));
            assertEq(entries[2].topics[3], bytes32(uint256(6)));
            assertEq(abi.decode(entries[2].data, (string)), string(testData2));
            assertEq(entries[2].emitter, emitter2.getEmitterAddr());
        }
    
        function testRecordsConsumednAsRead() public {
            Vm.Log[] memory entries;
    
            emitter.emitEvent(1, generateTestData(16));
    
            // hit record now
            vm.recordLogs();
    
            entries = vm.getRecordedLogs();
            assertEq(entries.length, 0);
    
            // emit after calling .getRecordedLogs()
            emitter.emitEvent(2, 3, generateTestData(24));
    
            entries = vm.getRecordedLogs();
            assertEq(entries.length, 1);
            assertEq(entries[0].topics.length, 3);
            assertEq(entries[0].emitter, address(emitter));
    
            // let's emit two more!
            emitter.emitEvent(4, 5, 6, generateTestData(20));
            emitter.emitEvent(generateTestData(32));
    
            entries = vm.getRecordedLogs();
            assertEq(entries.length, 2);
            assertEq(entries[0].topics.length, 4);
            assertEq(entries[1].topics.length, 1);
            assertEq(entries[0].emitter, address(emitter));
            assertEq(entries[1].emitter, address(emitter));
    
            // the last one
            emitter.emitEvent(7, 8, 9, generateTestData(24));
    
            entries = vm.getRecordedLogs();
            assertEq(entries.length, 1);
            assertEq(entries[0].topics.length, 4);
            assertEq(entries[0].emitter, address(emitter));
        }
    }
  "#,
    )
    .unwrap();
    prj.update_config(|config| config.evm_version = EvmVersion::Cancun);

    let res = cmd.args(["test", "--resolc", "--polkadot", "-vvvvv"]).assert_success();
    res.stderr_eq("").stdout_eq(str![[r#"
[COMPILING_FILES] with [SOLC_VERSION]
[SOLC_VERSION] [ELAPSED]
Compiler run successful!
[COMPILING_FILES] with [RESOLC_VERSION]
[RESOLC_VERSION] [ELAPSED]
Compiler run successful!

Ran 7 tests for src/Test.t.sol:RecordLogsTest
[PASS] testEmitRecordEmit() ([GAS])
Traces:
  [16868742] RecordLogsTest::setUp()
    ├─ [16830999] → new <unknown>@0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC
    │   └─ ← [Return] 12583 bytes of code
    └─ ← [Stop]

  [357757177] RecordLogsTest::testEmitRecordEmit()
    ├─ [183812741] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::emitEvent(1, 2, 0x43a26051362b8040b289abe93334a5e3662751aa691185ae9e9a2e1e0c169350)
    │   ├─ emit LogTopic12(topic1: 1, topic2: 2, data: 0x43a26051362b8040b289abe93334a5e3662751aa691185ae9e9a2e1e0c169350)
    │   └─ ← [Stop]
    ├─ [0] VM::recordLogs()
    │   └─ ← [Return]
    ├─ [173888857] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::emitEvent(3, 0x2e38edeff9493e0004540e975027a429)
    │   ├─ emit LogTopic1(topic1: 3, data: 0x2e38edeff9493e0004540e975027a429)
    │   └─ ← [Stop]
    ├─ [0] VM::getRecordedLogs()
    │   └─ ← [Return] [([0x7c7d81fafce31d4330303f05da0ccb9d970101c475382b40aa072986ee4caaad, 0x0000000000000000000000000000000000000000000000000000000000000003], 0x000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000102e38edeff9493e0004540e975027a42900000000000000000000000000000000, 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC)]
    ├─  storage changes:
    │   @ 1: 0x43a26051362b8040b289abe93334a5e3662751aa691185ae9e9a2e1e0c169350 → 0x2e38edeff9493e0004540e975027a429ee666d1289f2c7a4232d03ee63e14e30
    └─ ← [Stop]

[PASS] testRecordOffGetsNothing() ([GAS])
Traces:
  [16868742] RecordLogsTest::setUp()
    ├─ [16830999] → new <unknown>@0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC
    │   └─ ← [Return] 12583 bytes of code
    └─ ← [Stop]

  [202674284] RecordLogsTest::testRecordOffGetsNothing()
    ├─ [202625294] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::emitEvent(1, 2, 3, 0x43a26051362b8040b289abe93334a5e3662751aa691185ae9e9a2e1e0c1693502e38edeff9493e0004540e975027a429)
    │   ├─ emit LogTopic123(topic1: 1, topic2: 2, topic3: 3, data: 0x43a26051362b8040b289abe93334a5e3662751aa691185ae9e9a2e1e0c1693502e38edeff9493e0004540e975027a429)
    │   └─ ← [Stop]
    ├─ [0] VM::getRecordedLogs()
    │   └─ ← [Return] []
    ├─  storage changes:
    │   @ 1: 0x43a26051362b8040b289abe93334a5e3662751aa691185ae9e9a2e1e0c169350 → 0x2e38edeff9493e0004540e975027a429ee666d1289f2c7a4232d03ee63e14e30
    └─ ← [Stop]

[PASS] testRecordOnEmitDifferentDepths() ([GAS])
Traces:
  [16868742] RecordLogsTest::setUp()
    ├─ [16830999] → new <unknown>@0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC
    │   └─ ← [Return] 12583 bytes of code
    └─ ← [Stop]

  [999237291] RecordLogsTest::testRecordOnEmitDifferentDepths()
    ├─ [0] VM::recordLogs()
    │   └─ ← [Return]
    ├─ emit LogTopic(topic1: 1, data: 0x43a26051362b8040b289abe93334a5e3)
    ├─ [180758801] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::emitEvent(2, 3, 0x43a26051362b8040b289abe93334a5e3662751aa)
    │   ├─ emit LogTopic12(topic1: 2, topic2: 3, data: 0x43a26051362b8040b289abe93334a5e3662751aa)
    │   └─ ← [Stop]
    ├─ [818371229] → new <unknown>@0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f
    │   └─ ← [Return] 10554 bytes of code
    ├─ [0] 0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f::emitEvent(4, 5, 6, 0x43a26051362b8040b289abe93334a5e3662751aa691185ae)
    │   ├─ [0] 0x104fBc016F4bb334D775a19E8A6510109AC63E00::emitEvent(4, 5, 6, 0x43a26051362b8040b289abe93334a5e3662751aa691185ae)
    │   │   ├─ emit LogTopic123(topic1: 4, topic2: 5, topic3: 6, data: 0x43a26051362b8040b289abe93334a5e3662751aa691185ae)
    │   │   └─ ← [Return]
    │   └─ ← [Stop]
    ├─ [0] VM::getRecordedLogs()
    │   └─ ← [Return] [([0x61fb7db3625c10432927a76bb32400c33a94e9bb6374137c4cd59f6e465bfdcb, 0x0000000000000000000000000000000000000000000000000000000000000001], 0x0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000001043a26051362b8040b289abe93334a5e300000000000000000000000000000000, 0x7FA9385bE102ac3EAc297483Dd6233D62b3e1496), ([0x7af92d5e3102a27d908bb1859fdef71b723f3c438e5d84f3af49dab68e18dc6d, 0x0000000000000000000000000000000000000000000000000000000000000002, 0x0000000000000000000000000000000000000000000000000000000000000003], 0x0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000001443a26051362b8040b289abe93334a5e3662751aa000000000000000000000000, 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC), ([0xb6d650e5d0bbc0e92ff784e346ada394e49aa2d74a5cee8b099fa1a469bdc452, 0x0000000000000000000000000000000000000000000000000000000000000004, 0x0000000000000000000000000000000000000000000000000000000000000005, 0x0000000000000000000000000000000000000000000000000000000000000006], 0x0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000001843a26051362b8040b289abe93334a5e3662751aa691185ae0000000000000000, 0x104fBc016F4bb334D775a19E8A6510109AC63E00)]
    ├─ [0] 0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f::getEmitterAddr() [staticcall]
    │   └─ ← [Return] 0x104fBc016F4bb334D775a19E8A6510109AC63E00
    └─ ← [Stop]

[PASS] testRecordOnNoLogs() ([GAS])
Traces:
  [16868742] RecordLogsTest::setUp()
    ├─ [16830999] → new <unknown>@0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC
    │   └─ ← [Return] 12583 bytes of code
    └─ ← [Stop]

  [4118] RecordLogsTest::testRecordOnNoLogs()
    ├─ [0] VM::recordLogs()
    │   └─ ← [Return]
    ├─ [0] VM::getRecordedLogs()
    │   └─ ← [Return] []
    └─ ← [Stop]

[PASS] testRecordOnSingleLog() ([GAS])
Traces:
  [16868742] RecordLogsTest::setUp()
    ├─ [16830999] → new <unknown>@0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC
    │   └─ ← [Return] 12583 bytes of code
    └─ ← [Stop]

  [187093023] RecordLogsTest::testRecordOnSingleLog()
    ├─ [0] VM::recordLogs()
    │   └─ ← [Return]
    ├─ [187077066] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::emitEvent(1, 2, 3, 0x4576656e74204461746120696e20537472696e67)
    │   ├─ emit LogTopic123(topic1: 1, topic2: 2, topic3: 3, data: 0x4576656e74204461746120696e20537472696e67)
    │   └─ ← [Stop]
    ├─ [0] VM::getRecordedLogs()
    │   └─ ← [Return] [([0xb6d650e5d0bbc0e92ff784e346ada394e49aa2d74a5cee8b099fa1a469bdc452, 0x0000000000000000000000000000000000000000000000000000000000000001, 0x0000000000000000000000000000000000000000000000000000000000000002, 0x0000000000000000000000000000000000000000000000000000000000000003], 0x000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000144576656e74204461746120696e20537472696e67000000000000000000000000, 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC)]
    └─ ← [Stop]

[PASS] testRecordOnSingleLogTopic0() ([GAS])
Traces:
  [16868742] RecordLogsTest::setUp()
    ├─ [16830999] → new <unknown>@0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC
    │   └─ ← [Return] 12583 bytes of code
    └─ ← [Stop]

  [184656340] RecordLogsTest::testRecordOnSingleLogTopic0()
    ├─ [0] VM::recordLogs()
    │   └─ ← [Return]
    ├─ [184603101] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::emitEvent(0x43a26051362b8040b289abe93334a5e3662751aa691185ae9e9a2e1e0c1693502e38edeff9493e0004540e975027a429)
    │   ├─ emit LogTopic0(data: 0x43a26051362b8040b289abe93334a5e3662751aa691185ae9e9a2e1e0c1693502e38edeff9493e0004540e975027a429)
    │   └─ ← [Stop]
    ├─ [0] VM::getRecordedLogs()
    │   └─ ← [Return] [([0x0a28c6fad56bcbad1788721e440963b3b762934a3134924733eaf8622cb44279], 0x0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000003043a26051362b8040b289abe93334a5e3662751aa691185ae9e9a2e1e0c1693502e38edeff9493e0004540e975027a42900000000000000000000000000000000, 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC)]
    ├─  storage changes:
    │   @ 1: 0x43a26051362b8040b289abe93334a5e3662751aa691185ae9e9a2e1e0c169350 → 0x2e38edeff9493e0004540e975027a429ee666d1289f2c7a4232d03ee63e14e30
    └─ ← [Stop]

[PASS] testRecordsConsumednAsRead() ([GAS])
Traces:
  [16868742] RecordLogsTest::setUp()
    ├─ [16830999] → new <unknown>@0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC
    │   └─ ← [Return] 12583 bytes of code
    └─ ← [Stop]

  [903065419] RecordLogsTest::testRecordsConsumednAsRead()
    ├─ [173888857] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::emitEvent(1, 0x43a26051362b8040b289abe93334a5e3)
    │   ├─ emit LogTopic1(topic1: 1, data: 0x43a26051362b8040b289abe93334a5e3)
    │   └─ ← [Stop]
    ├─ [0] VM::recordLogs()
    │   └─ ← [Return]
    ├─ [0] VM::getRecordedLogs()
    │   └─ ← [Return] []
    ├─ [181776781] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::emitEvent(2, 3, 0x43a26051362b8040b289abe93334a5e3662751aa691185ae)
    │   ├─ emit LogTopic12(topic1: 2, topic2: 3, data: 0x43a26051362b8040b289abe93334a5e3662751aa691185ae)
    │   └─ ← [Stop]
    ├─ [0] VM::getRecordedLogs()
    │   └─ ← [Return] [([0x7af92d5e3102a27d908bb1859fdef71b723f3c438e5d84f3af49dab68e18dc6d, 0x0000000000000000000000000000000000000000000000000000000000000002, 0x0000000000000000000000000000000000000000000000000000000000000003], 0x0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000001843a26051362b8040b289abe93334a5e3662751aa691185ae0000000000000000, 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC)]
    ├─ [187077066] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::emitEvent(4, 5, 6, 0x43a26051362b8040b289abe93334a5e3662751aa)
    │   ├─ emit LogTopic123(topic1: 4, topic2: 5, topic3: 6, data: 0x43a26051362b8040b289abe93334a5e3662751aa)
    │   └─ ← [Stop]
    ├─ [172108813] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::emitEvent(0x43a26051362b8040b289abe93334a5e3662751aa691185ae9e9a2e1e0c169350)
    │   ├─ emit LogTopic0(data: 0x43a26051362b8040b289abe93334a5e3662751aa691185ae9e9a2e1e0c169350)
    │   └─ ← [Stop]
    ├─ [0] VM::getRecordedLogs()
    │   └─ ← [Return] [([0xb6d650e5d0bbc0e92ff784e346ada394e49aa2d74a5cee8b099fa1a469bdc452, 0x0000000000000000000000000000000000000000000000000000000000000004, 0x0000000000000000000000000000000000000000000000000000000000000005, 0x0000000000000000000000000000000000000000000000000000000000000006], 0x0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000001443a26051362b8040b289abe93334a5e3662751aa000000000000000000000000, 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC), ([0x0a28c6fad56bcbad1788721e440963b3b762934a3134924733eaf8622cb44279], 0x0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002043a26051362b8040b289abe93334a5e3662751aa691185ae9e9a2e1e0c169350, 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC)]
    ├─ [188095046] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::emitEvent(7, 8, 9, 0x2e38edeff9493e0004540e975027a429ee666d1289f2c7a4)
    │   ├─ emit LogTopic123(topic1: 7, topic2: 8, topic3: 9, data: 0x2e38edeff9493e0004540e975027a429ee666d1289f2c7a4)
    │   └─ ← [Stop]
    ├─ [0] VM::getRecordedLogs()
    │   └─ ← [Return] [([0xb6d650e5d0bbc0e92ff784e346ada394e49aa2d74a5cee8b099fa1a469bdc452, 0x0000000000000000000000000000000000000000000000000000000000000007, 0x0000000000000000000000000000000000000000000000000000000000000008, 0x0000000000000000000000000000000000000000000000000000000000000009], 0x000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000182e38edeff9493e0004540e975027a429ee666d1289f2c7a40000000000000000, 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC)]
    ├─  storage changes:
    │   @ 1: 0x43a26051362b8040b289abe93334a5e3662751aa691185ae9e9a2e1e0c169350 → 0x2e38edeff9493e0004540e975027a429ee666d1289f2c7a4232d03ee63e14e30
    └─ ← [Stop]

Suite result: ok. 7 passed; 0 failed; 0 skipped; [ELAPSED]

Ran 1 test suite [ELAPSED]: 7 tests passed, 0 failed, 0 skipped (7 total tests)

"#]]);
});

forgetest!(record_accesses, |prj, cmd| {
    prj.insert_ds_test();
    prj.insert_vm();
    prj.insert_console();

    prj.add_source(
        "Contracts.sol",
        r#"
        contract C {
          uint256 internal _reserved;
          uint256 public data;
          constructor(uint _data) payable { data = _data; }
          function setter(uint _data) public { data = _data; }
      }

      contract Proxy {
        address target;
        constructor(address _data) payable { target = _data; }
        function proxyCall(uint _data) public {
          (bool success,) = address(target).call(abi.encodeWithSelector(C.setter.selector, _data));
          if (!success) {
            assert(false);
          }
        }
      }
"#,
    )
    .unwrap();
    prj.add_source(
        "Test.t.sol",
        r#"
  pragma solidity ^0.8.18;
  import "./test.sol";
  import "./Vm.sol";
  import "./Contracts.sol";
  import {console} from "./console.sol";

  contract StateDiffTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);
    address existing;
    address proxy;

    function setUp() public {
      existing = address(new C{value: 1 ether}(100));
      proxy = address(new Proxy(existing));
    }

    function testCreateaccesses() public {
      vm.startStateDiffRecording();
      C target = new C{value: 1 ether}(100);
      Vm.AccountAccess[] memory records = vm.stopAndReturnStateDiff();
      assertEq(records.length, 1, "Records");
      assertEq(records[0].account, address(target), "Account");
      assertEq(records[0].accessor, address(this), "Accessor");
      assertEq(records[0].initialized, true);
      assertEq(records[0].oldBalance, 0, "oldBalance");
      assertEq(records[0].newBalance, 1 ether, "newBalance");
      assertEq(records[0].value, 1 ether, "value");
      assertEq(records[0].data, abi.encode(uint(100)), "data");
      assertEq(records[0].reverted, false);
       
      assertEq(records[0].storageAccesses.length, 2, "accesses"); // check the write
      assertEq(records[0].storageAccesses[1].account, address(target), "access address");
      assertEq(records[0].storageAccesses[1].slot, bytes32(uint256(1)), "slot");
      assertEq(records[0].storageAccesses[1].isWrite, true);
      assertEq(records[0].storageAccesses[1].previousValue, bytes32(uint(0)), "previousValue");
      assertEq(records[0].storageAccesses[1].newValue, bytes32(uint(100)), "newValue");
      assertEq(records[0].storageAccesses[1].reverted, false);    
    }

    function testCallaccesses() public {
      vm.startStateDiffRecording();
      (bool success,) = address(existing).call(abi.encodeWithSelector(C.setter.selector, 55));
      if (!success) {
        assert(false);
      }
      Vm.AccountAccess[] memory records = vm.stopAndReturnStateDiff();
      assertEq(records.length, 1, "records");
      assertEq(records[0].account, address(existing), "Account");
      assertEq(records[0].accessor, address(this), "Accessor");
      assertEq(records[0].initialized, true);
      assertEq(records[0].oldBalance, 1 ether, "oldBalance");
      assertEq(records[0].newBalance, 1 ether, "newBalance");
      assertEq(records[0].value, 0 ether, "value");
      assertEq(records[0].data, abi.encodeWithSelector(C.setter.selector, 55), "data");
      assertEq(records[0].reverted, false);
       
      assertEq(records[0].storageAccesses.length, 2, "accesses"); // check the write
      assertEq(records[0].storageAccesses[1].account, address(existing), "access address");
      assertEq(records[0].storageAccesses[1].slot, bytes32(uint256(1)), "slot");
      assertEq(records[0].storageAccesses[1].isWrite, true);
      assertEq(records[0].storageAccesses[1].previousValue, bytes32(uint(100)), "previousValue");
      assertEq(records[0].storageAccesses[1].newValue, bytes32(uint(55)), "newValue");
      assertEq(records[0].storageAccesses[1].reverted, false);    
    }
    function testCallProxyaccesses() public {
      vm.startStateDiffRecording();
      (bool success,) = address(proxy).call(abi.encodeWithSelector(Proxy.proxyCall.selector, 55));
      if (!success) {
        assert(false);
      }
      Vm.AccountAccess[] memory records = vm.stopAndReturnStateDiff();
      assertEq(records.length, 2, "records");
      assertEq(records[1].account, address(existing), "Account"); // checks the access from Proxy to C
      assertEq(records[1].accessor, address(proxy), "Accessor");
      assertEq(records[1].initialized, true);
      assertEq(records[1].oldBalance, 1 ether, "oldBalance");
      assertEq(records[1].newBalance, 1 ether, "newBalance");
      assertEq(records[1].value, 0 ether, "value");
      assertEq(records[1].data, abi.encodeWithSelector(C.setter.selector, 55), "data");
      assertEq(records[1].reverted, false);
       
      assertEq(records[1].storageAccesses.length, 2, "accesses"); // check the write
      assertEq(records[1].storageAccesses[1].account, address(existing), "access address");
      assertEq(records[1].storageAccesses[1].slot, bytes32(uint256(1)), "slot");
      assertEq(records[1].storageAccesses[1].isWrite, true);
      assertEq(records[1].storageAccesses[1].previousValue, bytes32(uint(100)), "previousValue");
      assertEq(records[1].storageAccesses[1].newValue, bytes32(uint(55)), "newValue");
      assertEq(records[1].storageAccesses[1].reverted, false);    
    }
  }
  "#,
    )
    .unwrap();
    prj.update_config(|config| config.evm_version = EvmVersion::Cancun);

    let res = cmd.args(["test", "--resolc", "--polkadot", "-vvvvv"]).assert_success();
    res.stdout_eq(str![[r#"
[COMPILING_FILES] with [SOLC_VERSION]
[SOLC_VERSION] [ELAPSED]
Compiler run successful!
[COMPILING_FILES] with [RESOLC_VERSION]
[RESOLC_VERSION] [ELAPSED]
Compiler run successful!

Ran 3 tests for src/Test.t.sol:StateDiffTest
[PASS] testCallProxyaccesses() ([GAS])
Traces:
  [585251161] StateDiffTest::setUp()
    ├─ [292049387] → new <unknown>@0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC
    │   └─ ← [Return] 5531 bytes of code
    ├─ [293109162] → new <unknown>@0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f
    │   └─ ← [Return] 6405 bytes of code
    └─ ← [Stop]

  [728077974] StateDiffTest::testCallProxyaccesses()
    ├─ [0] VM::startStateDiffRecording()
    │   └─ ← [Return]
    ├─ [728040641] 0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f::proxyCall(55)
    │   ├─ [0] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::setter(55)
    │   │   └─ ← [Return]
    │   └─ ← [Stop]
    ├─ [0] VM::stopAndReturnStateDiff()
    │   └─ ← [Return] [((0, 31337 [3.133e4]), 0, 0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f, 0x7FA9385bE102ac3EAc297483Dd6233D62b3e1496, true, 0, 1000000000000000000 [1e18], 0x, 0, 0xac1b14ff0000000000000000000000000000000000000000000000000000000000000037, false, [(0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f, 0x0000000000000000000000000000000000000000000000000000000000000000, false, 0x0000000000000000000000007d8cb8f412b3ee9ac79558791333f41d2b1ccdac, 0x0000000000000000000000007d8cb8f412b3ee9ac79558791333f41d2b1ccdac, false)], 1), ((0, 31337 [3.133e4]), 0, 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC, 0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f, true, 1000000000000000000 [1e18], 1000000000000000000 [1e18], 0x, 0, 0xd423740b0000000000000000000000000000000000000000000000000000000000000037, false, [(0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC, 0x0000000000000000000000000000000000000000000000000000000000000001, false, 0x0000000000000000000000000000000000000000000000000000000000000064, 0x0000000000000000000000000000000000000000000000000000000000000064, false), (0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC, 0x0000000000000000000000000000000000000000000000000000000000000001, true, 0x0000000000000000000000000000000000000000000000000000000000000064, 0x0000000000000000000000000000000000000000000000000000000000000037, false)], 2)]
    └─ ← [Stop]

[PASS] testCallaccesses() ([GAS])
Traces:
  [585251161] StateDiffTest::setUp()
    ├─ [292049387] → new <unknown>@0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC
    │   └─ ← [Return] 5531 bytes of code
    ├─ [293109162] → new <unknown>@0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f
    │   └─ ← [Return] 6405 bytes of code
    └─ ← [Stop]

  [276825754] StateDiffTest::testCallaccesses()
    ├─ [0] VM::startStateDiffRecording()
    │   └─ ← [Return]
    ├─ [276796934] 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC::setter(55)
    │   └─ ← [Stop]
    ├─ [0] VM::stopAndReturnStateDiff()
    │   └─ ← [Return] [((0, 31337 [3.133e4]), 0, 0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC, 0x7FA9385bE102ac3EAc297483Dd6233D62b3e1496, true, 1000000000000000000 [1e18], 1000000000000000000 [1e18], 0x, 0, 0xd423740b0000000000000000000000000000000000000000000000000000000000000037, false, [(0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC, 0x0000000000000000000000000000000000000000000000000000000000000001, false, 0x0000000000000000000000000000000000000000000000000000000000000064, 0x0000000000000000000000000000000000000000000000000000000000000064, false), (0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC, 0x0000000000000000000000000000000000000000000000000000000000000001, true, 0x0000000000000000000000000000000000000000000000000000000000000064, 0x0000000000000000000000000000000000000000000000000000000000000037, false)], 1)]
    └─ ← [Stop]

[PASS] testCreateaccesses() ([GAS])
Traces:
  [585251161] StateDiffTest::setUp()
    ├─ [292049387] → new <unknown>@0x7D8CB8F412B3ee9AC79558791333F41d2b1ccDAC
    │   └─ ← [Return] 5531 bytes of code
    ├─ [293109162] → new <unknown>@0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f
    │   └─ ← [Return] 6405 bytes of code
    └─ ← [Stop]

  [292103665] StateDiffTest::testCreateaccesses()
    ├─ [0] VM::startStateDiffRecording()
    │   └─ ← [Return]
    ├─ [292049387] → new <unknown>@0x2e234DAe75C793f67A35089C9d99245E1C58470b
    │   └─ ← [Return] 5531 bytes of code
    ├─ [0] VM::stopAndReturnStateDiff()
    │   └─ ← [Return] [((0, 31337 [3.133e4]), 4, 0x2e234DAe75C793f67A35089C9d99245E1C58470b, 0x7FA9385bE102ac3EAc297483Dd6233D62b3e1496, true, 0, 1000000000000000000 [1e18], 0x, 1000000000000000000 [1e18], 0x0000000000000000000000000000000000000000000000000000000000000064, false, [(0x2e234DAe75C793f67A35089C9d99245E1C58470b, 0x0000000000000000000000000000000000000000000000000000000000000001, false, 0x0000000000000000000000000000000000000000000000000000000000000000, 0x0000000000000000000000000000000000000000000000000000000000000000, false), (0x2e234DAe75C793f67A35089C9d99245E1C58470b, 0x0000000000000000000000000000000000000000000000000000000000000001, true, 0x0000000000000000000000000000000000000000000000000000000000000000, 0x0000000000000000000000000000000000000000000000000000000000000064, false)], 1)]
    └─ ← [Stop]

Suite result: ok. 3 passed; 0 failed; 0 skipped; [ELAPSED]

Ran 1 test suite [ELAPSED]: 3 tests passed, 0 failed, 0 skipped (3 total tests)

"#]]);
});
