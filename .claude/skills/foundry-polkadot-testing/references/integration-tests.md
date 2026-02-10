# Integration Tests: Comprehensive Guide

<nav>
  <tag name="overview" section="Overview" />
  <tag name="solidity-tests" section="Creating Solidity Tests" />
  <tag name="rust-wrappers" section="Creating Rust Wrappers" />
  <tag name="registration" section="Module Registration" />
  <tag name="testing" section="Testing and Validation" />
</nav>

<atom>
  <type>reference</type>
  <diataxis_quadrant>how-to</diataxis_quadrant>
  <intent>add_integration_tests</intent>
</atom>

## Overview

**Important**: This guide is for adding integration tests **to the foundry-polkadot repository itself**.

**For testing your own contracts**, use standard `forge-std/Test.sol` - see [Writing Tests Guide](writing-tests.md).

Integration tests for Polkadot/Revive functionality consist of **two parts**:

1. **Solidity Test Contract** - The actual test logic (uses DSTest)
2. **Rust Test Wrapper** - Runs the Solidity test in both EVM and PVM modes

This guide walks you through adding a complete integration test to validate foundry-polkadot's integration with pallet-revive.

## Integration Test Structure

```
foundry-polkadot/
├── testdata/default/revive/
│   └── YourTest.t.sol          ← Solidity test contract
└── crates/forge/tests/it/revive/
    ├── mod.rs                   ← Module registration
    └── your_test.rs             ← Rust test wrapper
```

## Step-by-Step: Adding a New Integration Test

### Step 1: Create Solidity Test Contract

**Location**: `testdata/default/revive/YourTest.t.sol`

**Note**: These integration tests use `DSTest` (not `forge-std/Test.sol`) because they're testing foundry-polkadot's internal integration. Your own contract tests should use `forge-std/Test.sol`.

**Template**:

```solidity
// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "ds-test/test.sol";
import "cheats/Vm.sol";

contract YourTestContract is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function setUp() public {
        // Setup code runs once before each test function
    }

    function testYourFeature() public {
        // Test logic here
    }

    function testAnotherFeature() public {
        // Another test
    }
}
```

**Key Requirements**:

1. **Use `DSTest`**, NOT `forge-std/Test.sol`
2. **Import** `"ds-test/test.sol"` and `"cheats/Vm.sol"`
3. **Declare** `Vm constant vm = Vm(HEVM_ADDRESS);`
4. **License**: Use `MIT OR Apache-2.0`
5. **Solidity version**: `^0.8.18` or compatible

### Step 2: Create Rust Test Wrapper

**Location**: `crates/forge/tests/it/revive/your_feature.rs`

**Template**:

```rust
use crate::{config::*, test_helpers::TEST_DATA_REVIVE};
use foundry_test_utils::Filter;
use revive_strategy::ReviveRuntimeMode;
use revm::primitives::hardfork::SpecId;
use rstest::rstest;

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_your_feature(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new(".*", ".*", ".*/revive/YourTest.t.sol");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}
```

**Key Components**:

1. **`#[rstest]`**: Parameterized test framework
2. **`#[case::pvm]` and `#[case::evm]`**: Test both backends
3. **`#[tokio::test(flavor = "multi_thread")]`**: Async runtime
4. **`TEST_DATA_REVIVE.runner_revive(runtime_mode)`**: Test runner for revive tests
5. **`Filter::new(".*", ".*", ".*/revive/YourTest.t.sol")`**: Path to Solidity test
6. **`SpecId::PRAGUE`**: EVM specification version

### Step 3: Register the Module

**Location**: `crates/forge/tests/it/revive/mod.rs`

Add your module to the list:

```rust
//! Revive strategy tests

pub mod cheat_etch;
pub mod cheat_gas_metering;
pub mod cheat_migrations;
pub mod cheat_mock_calls;
pub mod cheat_rpc;
pub mod cheat_storage;
pub mod tx_gas_price;
pub mod your_feature;  // ← Add this line
```

**Important**: Module name must match filename (snake_case).

## Complete Example: Testing a New Cheatcode

Let's walk through adding a test for a new cheatcode called `expectCreate`.

### Example: Solidity Test

**File**: `testdata/default/revive/ExpectCreate.t.sol`

```solidity
// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "ds-test/test.sol";
import "cheats/Vm.sol";

contract Helper {
    uint256 public value;

    constructor(uint256 _value) {
        value = _value;
    }
}

contract ExpectCreateTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testExpectCreateSuccess() public {
        // Expect a contract to be created
        vm.expectCreate();

        // Deploy contract
        Helper helper = new Helper(42);

        // Verify deployment
        assertEq(helper.value(), 42);
    }

    function testExpectCreateWithAddress() public {
        // Calculate expected address
        address expectedAddr = vm.computeCreateAddress(address(this), vm.getNonce(address(this)));

        // Expect create at specific address
        vm.expectCreate(expectedAddr);

        // Deploy
        Helper helper = new Helper(100);

        // Verify address
        assertEq(address(helper), expectedAddr);
        assertEq(helper.value(), 100);
    }

    function testExpectCreateFails() public {
        // Expect create that doesn't happen
        vm.expectCreate();

        // Don't deploy anything
        // This test should fail
    }

    function testMultipleCreates() public {
        // Expect multiple creates
        vm.expectCreate();
        Helper helper1 = new Helper(1);

        vm.expectCreate();
        Helper helper2 = new Helper(2);

        assertEq(helper1.value(), 1);
        assertEq(helper2.value(), 2);
    }
}
```

### Example: Rust Wrapper

**File**: `crates/forge/tests/it/revive/cheat_expect_create.rs`

```rust
use crate::{config::*, test_helpers::TEST_DATA_REVIVE};
use foundry_test_utils::Filter;
use revive_strategy::ReviveRuntimeMode;
use revm::primitives::hardfork::SpecId;
use rstest::rstest;

#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_revive_expect_create(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new(".*", ".*", ".*/revive/ExpectCreate.t.sol");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}
```

### Example: Module Registration

**File**: `crates/forge/tests/it/revive/mod.rs`

```rust
//! Revive strategy tests

pub mod cheat_etch;
pub mod cheat_expect_create;  // ← Added
pub mod cheat_gas_metering;
pub mod cheat_migrations;
pub mod cheat_mock_calls;
pub mod cheat_rpc;
pub mod cheat_storage;
pub mod tx_gas_price;
```

## Testing Multiple Scenarios

### Pattern: Multiple Test Functions

```solidity
contract MultipleTests is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    Counter counter;

    function setUp() public {
        counter = new Counter();
    }

    function testIncrement() public {
        counter.increment();
        assertEq(counter.number(), 1);
    }

    function testDecrement() public {
        counter.increment();
        counter.increment();
        counter.decrement();
        assertEq(counter.number(), 1);
    }

    function testReset() public {
        counter.increment();
        counter.reset();
        assertEq(counter.number(), 0);
    }
}
```

Each `test*` function runs independently with its own `setUp()`.

### Pattern: Helper Contracts

```solidity
// Helper contracts defined in the same file
contract Helper {
    function compute(uint256 x) public pure returns (uint256) {
        return x * 2;
    }
}

contract HelperTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testWithHelper() public {
        Helper helper = new Helper();
        assertEq(helper.compute(21), 42);
    }
}
```

### Pattern: Complex State

```solidity
contract ComplexStateTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    struct Data {
        uint256 value;
        address owner;
        bool active;
    }

    function testStructStorage() public {
        Store store = new Store();
        vm.makePersistent(address(store));

        // Store struct data
        store.setData(Data({
            value: 42,
            owner: address(0x1234),
            active: true
        }));

        // Migrate
        vm.polkadot(false);

        // Verify struct persists
        Data memory data = store.getData();
        assertEq(data.value, 42);
        assertEq(data.owner, address(0x1234));
        assertTrue(data.active);
    }
}
```

## Running Your Tests

### Test Solidity Contract Directly

```bash
# Run in REVM
forge test --match-contract YourTestContract -vvv

# Run in Polkadot EVM
forge test --polkadot --match-contract YourTestContract -vvv

# Run in Polkadot PVM
forge test --polkadot=pvm --match-contract YourTestContract -vvv

# Run specific test function
forge test --polkadot --match-test testYourFeature -vvv
```

### Test Rust Wrapper

```bash
# Run your specific test (both EVM and PVM)
cargo test --package forge --test it test_your_feature

# Run with logging
env RUST_LOG=warn cargo test --package forge --test it test_your_feature -- --nocapture

# Run with detailed logs
env RUST_LOG=warn,revive_strategy=info cargo test --package forge --test it test_your_feature -- --nocapture

# Run only EVM mode
cargo test --package forge --test it test_your_feature::evm

# Run only PVM mode
cargo test --package forge --test it test_your_feature::pvm
```

### Run All Revive Tests

```bash
# All integration tests
cargo test --package forge --test it test_revive_

# With logging
env RUST_LOG=warn cargo test --package forge --test it test_revive_ -- --nocapture
```

## Advanced Patterns

### Pattern: Conditional Testing

```solidity
contract ConditionalTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testEvmOnly() public {
        // This test might behave differently in EVM vs PVM
        Counter counter = new Counter();
        counter.increment();
        assertEq(counter.number(), 1);
    }
}
```

For tests that should skip PVM mode, filter in Rust:

```rust
#[tokio::test(flavor = "multi_thread")]
async fn test_evm_only() {
    let runtime_mode = ReviveRuntimeMode::Evm;
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new(".*", ".*", ".*/revive/EvmOnly.t.sol");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}
```

### Pattern: Testing Reverts

```solidity
contract RevertTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testRevertWithMessage() public {
        Counter counter = new Counter();

        vm.expectRevert("Counter: underflow");
        counter.decrement();
    }

    function testRevertWithCustomError() public {
        Counter counter = new Counter();

        vm.expectRevert(Counter.Underflow.selector);
        counter.decrement();
    }

    function testNoRevert() public {
        Counter counter = new Counter();

        counter.increment();
        counter.decrement();
        // No revert expected
    }
}
```

### Pattern: Testing Events

```solidity
contract EventTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    event Transfer(address indexed from, address indexed to, uint256 value);

    function testEvent() public {
        Token token = new Token();

        // Expect event with specific parameters
        vm.expectEmit(true, true, false, true);
        emit Transfer(address(0), address(this), 1000);

        token.mint(address(this), 1000);
    }

    function testMultipleEvents() public {
        Token token = new Token();

        vm.expectEmit(true, true, false, true);
        emit Transfer(address(0), address(this), 1000);
        token.mint(address(this), 1000);

        vm.expectEmit(true, true, false, true);
        emit Transfer(address(this), address(0x1), 100);
        token.transfer(address(0x1), 100);
    }
}
```

### Pattern: Fuzz Testing

```solidity
contract FuzzTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testFuzzIncrement(uint8 times) public {
        Counter counter = new Counter();

        for (uint8 i = 0; i < times; i++) {
            counter.increment();
        }

        assertEq(counter.number(), times);
    }

    function testFuzzBalance(uint128 amount) public {
        // Use u128 for Polkadot compatibility
        address user = address(0x1);
        vm.deal(user, amount);
        assertEq(user.balance, amount);
    }
}
```

**Note**: For Polkadot compatibility, consider setting `max_fuzz_int` in `foundry.toml` to limit fuzz values (e.g., u64::MAX for timestamps/block numbers, u128::MAX for balances).

## Validation and Debugging

### Pre-commit Checklist

Before committing your integration test:

- [ ] Solidity test in `testdata/default/revive/`
- [ ] Rust wrapper in `crates/forge/tests/it/revive/`
- [ ] Module registered in `mod.rs`
- [ ] Solidity test passes: `forge test --polkadot --match-contract YourTest`
- [ ] Rust test passes: `cargo test --package forge --test it test_your_feature`
- [ ] Both EVM and PVM modes pass
- [ ] No warnings about clamped values
- [ ] Code formatted: `cargo fmt --all` and `forge fmt`

### Common Issues

#### Issue: Solidity test not found

```bash
# Error: No tests match filter
cargo test --package forge --test it test_your_feature
```

**Solution**: Check filter path in Rust test:
```rust
// ❌ WRONG
let filter = Filter::new(".*", ".*", "YourTest.t.sol");

// ✅ CORRECT
let filter = Filter::new(".*", ".*", ".*/revive/YourTest.t.sol");
```

#### Issue: Module not registered

```bash
# Error: unresolved import `crate::revive::your_feature`
```

**Solution**: Add to `mod.rs`:
```rust
pub mod your_feature;
```

#### Issue: Test fails in PVM but passes in EVM

**Likely causes**:
1. PVM bytecode not compiled
2. Library incompatibility
3. Proxy pattern not supported

**Solution**: Test EVM mode only or fix PVM compatibility:
```rust
#[tokio::test(flavor = "multi_thread")]
async fn test_evm_only() {
    let runtime_mode = ReviveRuntimeMode::Evm;
    // ...
}
```

#### Issue: Numeric overflow warnings

```bash
# Warning: Block number exceeds u64::MAX
```

**Solution**: Use appropriate types in fuzz tests:
```solidity
// ✅ Use u64 for timestamps/block numbers
function testFuzz(uint64 timestamp) public {
    vm.warp(timestamp);
}

// ✅ Use u128 for balances
function testFuzz(uint128 balance) public {
    vm.deal(address(0x1), balance);
}
```

### Debugging with Logs

**Solidity logs**:
```solidity
function testWithLogs() public {
    emit log("Starting test");
    emit log_uint(counter.number());

    counter.increment();

    emit log("After increment");
    emit log_uint(counter.number());
}
```

Run with verbosity:
```bash
forge test --polkadot --match-test testWithLogs -vvv
```

**Rust logs**:
```bash
# Detailed revive strategy logs
env RUST_LOG=warn,revive_strategy=debug \
  cargo test --package forge --test it test_your_feature -- --nocapture
```

## Testing Best Practices

### 1. One Feature Per Test

```solidity
// ✅ GOOD - Each test focuses on one feature
function testIncrement() public {
    counter.increment();
    assertEq(counter.number(), 1);
}

function testDecrement() public {
    counter.decrement();
    assertEq(counter.number(), 0);
}

// ❌ BAD - Tests too many things
function testEverything() public {
    counter.increment();
    counter.decrement();
    counter.reset();
    // ...
}
```

### 2. Clear Test Names

```solidity
// ✅ GOOD - Descriptive names
function testIncrementIncreasesCountByOne() public { }
function testDecrementRevertsWhenZero() public { }

// ❌ BAD - Vague names
function test1() public { }
function testCounter() public { }
```

### 3. Use setUp for Common Setup

```solidity
// ✅ GOOD - Setup in setUp()
Counter counter;

function setUp() public {
    counter = new Counter();
}

function testIncrement() public {
    counter.increment();
}

// ❌ BAD - Duplicate setup
function testIncrement() public {
    Counter counter = new Counter();
    counter.increment();
}

function testDecrement() public {
    Counter counter = new Counter();  // Duplicate
    counter.decrement();
}
```

### 4. Test Edge Cases

```solidity
function testIncrementAtMax() public {
    // Set counter to max value
    vm.store(
        address(counter),
        bytes32(uint256(0)),
        bytes32(type(uint256).max)
    );

    // Should revert on overflow
    vm.expectRevert();
    counter.increment();
}

function testDecrementAtZero() public {
    // Counter starts at 0
    vm.expectRevert("Counter: underflow");
    counter.decrement();
}
```

### 5. Test State Persistence

```solidity
function testStatePersistsAfterMigration() public {
    Counter counter = new Counter();
    vm.makePersistent(address(counter));

    counter.increment();
    uint256 before = counter.number();

    vm.polkadot(false);

    uint256 after = counter.number();
    assertEq(before, after);
}
```

## Examples from Existing Tests

### cheat_migrations.rs

```rust
#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_revive_basic_migration(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new(".*", ".*", ".*/revive/EvmToReviveMigration.t.sol");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}
```

### cheat_mock_calls.rs

```rust
#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_revive_mock_calls(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new(".*", ".*", ".*/revive/MockCall.t.sol");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}
```

### cheat_storage.rs

```rust
#[rstest]
#[case::pvm(ReviveRuntimeMode::Pvm)]
#[case::evm(ReviveRuntimeMode::Evm)]
#[tokio::test(flavor = "multi_thread")]
async fn test_revive_storage(#[case] runtime_mode: ReviveRuntimeMode) {
    let runner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
    let filter = Filter::new(".*", ".*", ".*/revive/Store.t.sol");

    TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
}
```

## Summary

To add an integration test:

1. **Create** `testdata/default/revive/YourTest.t.sol` (Solidity)
2. **Create** `crates/forge/tests/it/revive/your_test.rs` (Rust)
3. **Register** module in `crates/forge/tests/it/revive/mod.rs`
4. **Test** with `forge test --polkadot` and `cargo test`
5. **Validate** both EVM and PVM modes pass

Your test will automatically run in both EVM and PVM modes!

## References

- [Running Tests Guide](running-tests.md) - How to execute tests
- [Writing Tests Guide](writing-tests.md) - How to write Solidity tests
- [State Migration Guide](state-migration.md) - Understanding state sync
- [Troubleshooting Guide](troubleshooting.md) - Common issues
- [Existing Tests](../../../testdata/default/revive/) - Example Solidity tests
- [Existing Wrappers](../../../crates/forge/tests/it/revive/) - Example Rust wrappers
