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
    res.stdout_eq(str![[r#"
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

contract BlockTimestampRevive {
    function getBlockTimestamp() public view returns (uint256) {
        return block.timestamp;
    }
}

contract Warp is DSTest {
  Vm constant vm = Vm(HEVM_ADDRESS);

  function test_Warp() public {
      uint256 original = block.timestamp;
      vm.warp(100);
      BlockTimestampRevive revive = new BlockTimestampRevive();
      uint256 newValue = revive.getBlockTimestamp();
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

forgetest!(chainid, |prj, cmd| {
    prj.insert_ds_test();
    prj.insert_vm();
    prj.insert_console();
    prj.add_source(
        "ChainId.t.sol",
        r#"
import "./test.sol";
import "./Vm.sol";
import {console} from "./console.sol";

contract ChainIdRevive {
    function chain_id() public view returns (uint256) {
        return block.chainid;
    }
}


contract ChainIdTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testChainIdRevive() public {
        ChainIdRevive chainIdRevive = new ChainIdRevive();

        assertEq(chainIdRevive.chain_id(), block.chainid);

        uint256 newChainId = 99;
        vm.chainId(newChainId);
        assertEq(newChainId, block.chainid);
        assertEq(chainIdRevive.chain_id(), newChainId);
    }
}
"#,
    )
    .unwrap();

    let res = cmd.args(["test", "--resolc", "-vvvv", "--polkadot"]).assert_success();
    res.stderr_eq(str![""]).stdout_eq(str![[r#"
[COMPILING_FILES] with [SOLC_VERSION]
[SOLC_VERSION] [ELAPSED]
Compiler run successful!
[COMPILING_FILES] with [RESOLC_VERSION]
[RESOLC_VERSION] [ELAPSED]
Compiler run successful!

Ran 1 test for src/ChainId.t.sol:ChainIdTest
[PASS] testChainIdRevive() ([GAS])
Traces:
  [..] ChainIdTest::testChainIdRevive()
    ├─ [..] → new <unknown>@[..]
    │   └─ ← [Return] 2357 bytes of code
    ├─ [..] [..]::chain_id() [staticcall]
    │   └─ ← [Return] 31337 [3.133e4]
    ├─ [0] VM::chainId(99)
    │   └─ ← [Return]
    ├─ [..] [..]::chain_id() [staticcall]
    │   └─ ← [Return] 99
    └─ ← [Stop]

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

    let res = cmd.args(["test", "--resolc", "--polkadot", "-vvv"]).assert_success();
    res.stderr_eq("").stdout_eq(str![[r#"
[COMPILING_FILES] with [SOLC_VERSION]
[SOLC_VERSION] [ELAPSED]
Compiler run successful!
[COMPILING_FILES] with [RESOLC_VERSION]
[RESOLC_VERSION] [ELAPSED]
Compiler run successful!

Ran 7 tests for src/Test.t.sol:RecordLogsTest
[PASS] testEmitRecordEmit() ([GAS])
[PASS] testRecordOffGetsNothing() ([GAS])
[PASS] testRecordOnEmitDifferentDepths() ([GAS])
[PASS] testRecordOnNoLogs() ([GAS])
[PASS] testRecordOnSingleLog() ([GAS])
[PASS] testRecordOnSingleLogTopic0() ([GAS])
[PASS] testRecordsConsumednAsRead() ([GAS])
Suite result: ok. 7 passed; 0 failed; 0 skipped; [ELAPSED]

Ran 1 test suite [ELAPSED]: 7 tests passed, 0 failed, 0 skipped (7 total tests)

"#]]);
});

forgetest!(before_test_setup, |prj, cmd| {
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
    vm.deal(address(counter), 1 ether);
  }

  function callMe(uint256 number, uint256 amount) public {
    counter.setNumber(number);
    vm.deal(address(counter), amount * 1 ether);   
  }

  function beforeTestSetup(
    bytes4 testSelector
  ) public pure returns (bytes[] memory beforeTestCalldata) {
    if (testSelector == this.testA.selector) {
        beforeTestCalldata = new bytes[](1);
        beforeTestCalldata[0] = abi.encodeWithSignature("callMe(uint256,uint256)", 5, 2);
    }
    if (testSelector == this.testB.selector) {
        beforeTestCalldata = new bytes[](1);
        beforeTestCalldata[0] = abi.encodeWithSignature("callMe(uint256,uint256)", 10, 3);
    }
    if (testSelector == this.testC.selector) {
        beforeTestCalldata = new bytes[](1);
        beforeTestCalldata[0] = abi.encodeWithSignature("callMe(uint256,uint256)", 15, 4);
    }
    if (testSelector == this.testFuzz_SetNumber.selector) {
      beforeTestCalldata = new bytes[](1);
      beforeTestCalldata[0] = abi.encodeWithSignature("callMe(uint256,uint256)", 15, 4);
  }
  if (testSelector == this.testFuzz_SetNumber2.selector) {
    beforeTestCalldata = new bytes[](1);
    beforeTestCalldata[0] = abi.encodeWithSignature("callMe(uint256,uint256)", 1, 4);
}
  }

  function testA() public {
      assertEq(counter.number(), 5);
      assertEq(address(counter).balance, 2 ether);
      counter.setNumber(55); 
      assertEq(counter.number(), 55);
      counter.increment(); 
      assertEq(counter.number(), 56);
  }
  function testB() public {
    assertEq(counter.number(), 10);
    assertEq(address(counter).balance, 3 ether);
    counter.setNumber(55); 
    assertEq(counter.number(), 55);
    counter.increment(); 
    assertEq(counter.number(), 56);
}
function testC() public {
    assertEq(counter.number(), 15);
    assertEq(address(counter).balance, 4 ether);
    counter.setNumber(55); 
    assertEq(counter.number(), 55);
    counter.increment(); 
    assertEq(counter.number(), 56);
}

  function testFuzz_SetNumber(uint256 x) public {
      assertEq(counter.number(), 15);
      counter.setNumber(x); 
      assertEq(counter.number(), x);
  }
  
  function testFuzz_SetNumber2(uint256 x) public {
    assertEq(counter.number(), 1);
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
    res.stderr_eq("").stdout_eq(str![[r#"
[COMPILING_FILES] with [SOLC_VERSION]
[SOLC_VERSION] [ELAPSED]
Compiler run successful!
[COMPILING_FILES] with [RESOLC_VERSION]
[RESOLC_VERSION] [ELAPSED]
Compiler run successful!

Ran 6 tests for src/CounterTest.t.sol:CounterTest
[PASS] testA() ([GAS])
[PASS] testB() ([GAS])
[PASS] testC() ([GAS])
[PASS] testFuzz_SetNumber(uint256) (runs: 256, [AVG_GAS])
[PASS] testFuzz_SetNumber2(uint256) (runs: 256, [AVG_GAS])
[PASS] testFuzz_SetNumber3(uint256) (runs: 256, [AVG_GAS])
Suite result: ok. 6 passed; 0 failed; 0 skipped; [ELAPSED]

Ran 1 test suite [ELAPSED]: 6 tests passed, 0 failed, 0 skipped (6 total tests)

"#]]);
});
