# Writing Tests: Comprehensive Guide

<nav>
  <tag name="vm-polkadot" section="The vm.polkadot Cheatcode" />
  <tag name="test-structure" section="Test Structure" />
  <tag name="persistence" section="Contract Persistence" />
  <tag name="patterns" section="Common Patterns" />
  <tag name="assertions" section="Assertions and Checks" />
</nav>

<atom>
  <type>reference</type>
  <diataxis_quadrant>how-to</diataxis_quadrant>
  <intent>write_polkadot_tests</intent>
</atom>

## Overview

This guide covers writing Solidity tests that work with Polkadot's pallet-revive runtime using the `vm.polkadot` cheatcode.

## Test Structure

### For Your Own Tests (Recommended)

**Use standard `forge-std/Test.sol`** - it works perfectly with `--polkadot`:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import "forge-std/Test.sol";

contract MyTest is Test {
    function setUp() public {
        // Setup code
    }

    function testMyFeature() public {
        Counter counter = new Counter();
        counter.increment();
        assertEq(counter.number(), 1);
    }
}
```

**Run with**:
```bash
forge test --polkadot
```

This is the **standard Foundry experience** - use it for your own contracts!

### For Foundry-Polkadot Integration Tests

**Only when adding tests to the foundry-polkadot repository itself** (`testdata/default/revive/`), use `DSTest`:

```solidity
// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "ds-test/test.sol";
import "cheats/Vm.sol";

contract IntegrationTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function setUp() public {
        // Setup code
    }

    function testIntegration() public {
        // Test logic for foundry-polkadot integration
    }
}
```

**Why DSTest for integration tests?**
- Used in foundry-polkadot's internal test suite
- Simpler interface for testing the integration itself
- Works with the Rust test wrappers (`rstest`)

**Summary**:

| Use Case | Base Contract | When to Use |
|----------|---------------|-------------|
| **Your own tests** | `Test` from `forge-std` | ✅ **Always** - normal contract testing |
| **Integration tests** | `DSTest` from `ds-test` | Only when adding tests to foundry-polkadot repo |

**Important**: Both work with `forge test --polkadot`. Use `forge-std/Test.sol` for normal development!

## The vm.polkadot Cheatcode

### Function Signatures

```solidity
interface Vm {
    /// Switch INTO or OUT OF Polkadot runtime.
    /// backend: "evm" or "pvm"
    function polkadot(bool enable, string memory backend) external;

    /// Auto-detect backend from CLI flags.
    function polkadot(bool enable) external;

    /// Skip Polkadot execution for this test - runs only in REVM.
    /// Use this when cheatcodes are not available in Polkadot runtime.
    function polkadotSkip() external;
}
```

### Usage

**Enable Polkadot runtime**:
```solidity
vm.polkadot(true);  // Use backend from CLI (--polkadot flag)
```

**Disable Polkadot runtime**:
```solidity
vm.polkadot(false);  // Switch back to REVM
```

**Explicit backend selection**:
```solidity
vm.polkadot(true, "evm");  // Force EVM backend
vm.polkadot(true, "pvm");  // Force PVM backend
```

### Execution Matrix

| CLI Flag | Default Environment | `vm.polkadot(false)` | `vm.polkadot(true, "evm")` | `vm.polkadot(true, "pvm")` |
|----------|---------------------|----------------------|----------------------------|----------------------------|
| `forge test` | REVM | REVM | ❌ Invalid | ❌ Invalid |
| `forge test --polkadot` | Polkadot EVM | REVM | Polkadot EVM | Polkadot PVM |
| `forge test --polkadot=pvm` | Polkadot PVM | REVM | Polkadot EVM | Polkadot PVM |

**Key Points**:
- Without `--polkadot` flag, `vm.polkadot(true)` is invalid
- `vm.polkadot(false)` always switches to REVM
- Explicit backend selection overrides CLI flags

### When to Use vm.polkadot

**Common Scenarios**:

1. **Test migration behavior**:
```solidity
function testMigration() public {
    Counter counter = new Counter();  // Deployed in pallet-revive
    vm.makePersistent(address(counter));

    counter.increment();
    assertEq(counter.number(), 1);

    vm.polkadot(false);  // Switch to REVM
    assertEq(counter.number(), 1);  // State persists
}
```

2. **Compare REVM vs Polkadot behavior**:
```solidity
function testBehaviorDifference() public {
    vm.polkadot(false);  // REVM
    Counter counterREVM = new Counter();
    counterREVM.increment();

    vm.polkadot(true);  // Polkadot
    Counter counterPolkadot = new Counter();
    vm.makePersistent(address(counterPolkadot));
    counterPolkadot.increment();

    // Compare results
    assertEq(counterREVM.number(), counterPolkadot.number());
}
```

3. **Test only in Polkadot**:
```solidity
function testPolkadotOnly() public {
    // Implicitly in Polkadot when --polkadot flag is used
    // No need to call vm.polkadot(true) explicitly
    Counter counter = new Counter();
    counter.increment();
    assertEq(counter.number(), 1);
}
```

**When NOT to use**:

- Most tests don't need explicit `vm.polkadot()` calls
- The `--polkadot` flag handles runtime selection automatically
- Only use when testing migration or comparing behaviors

### vm.polkadotSkip - Skip Polkadot Execution

**Purpose**: Force test to run entirely in REVM, preventing contracts from being deployed to pallet-revive.

**Critical Understanding**:

- The cheatcode precompile (at `HEVM_ADDRESS`) **only exists in REVM**
- When using `--polkadot` flag, deployed contracts run in pallet-revive
- State is synced bidirectionally between REVM and pallet-revive
- When contracts in pallet-revive try to call `HEVM_ADDRESS`, it doesn't exist there
- `vm.polkadotSkip()` keeps contracts in REVM where `HEVM_ADDRESS` exists

**When to use**:

- Tests that call cheatcodes from deployed contracts
- Cheatcodes that need to be called from within contracts (not just test contract)
- Debugging REVM-specific behavior

**Error without skip**:
```
[FAIL: Cheatcodes are not available in polkadot runtime.]
```

**What this error means**:

- Your contract in pallet-revive is trying to call `HEVM_ADDRESS`
- The cheatcode precompile doesn't exist in pallet-revive environment
- State is synced, but the cheatcode address itself is REVM-only
- Solution: Use `vm.polkadotSkip()` to keep contracts in REVM where cheatcodes exist

**Solution - Add vm.polkadotSkip()**:

```solidity
function testWithStateDependent() public {
    // Keep everything in REVM - deployed contracts won't go to pallet-revive
    vm.polkadotSkip();

    // Now cheatcodes that need REVM state work correctly
    uint256 snapshot = vm.snapshot();

    Counter counter = new Counter();  // Deployed in REVM, not pallet-revive
    counter.increment();

    vm.revertTo(snapshot);  // Works because state is in REVM
}
```

**Example - Using snapshot/revert**:

```solidity
function testSnapshot() public {
    // snapshot/revert need unified state in REVM
    vm.polkadotSkip();

    Counter counter = new Counter();

    // Take snapshot of REVM state
    uint256 snapshot = vm.snapshot();

    counter.increment();
    assertEq(counter.number(), 1);

    // Revert REVM state to snapshot
    vm.revertTo(snapshot);
    assertEq(counter.number(), 0);
}
```

**Example - Using record/accesses**:

```solidity
function testStorageAccess() public {
    // record/accesses need to track REVM storage
    vm.polkadotSkip();

    Counter counter = new Counter();

    vm.record();
    counter.increment();
    (bytes32[] memory reads, bytes32[] memory writes) = vm.accesses(address(counter));

    // Verify storage was accessed in REVM
    assertEq(writes.length, 1);
}
```

**Important Notes**:
- `vm.polkadotSkip()` MUST be called at the start of the test function
- Test will run in REVM even when `--polkadot` flag is used
- Use sparingly - prefer tests that work in both modes
- Document why the skip is necessary

**Pattern - Dual Tests**:

```solidity
// Test that works in both modes
function testIncrement() public {
    Counter counter = new Counter();
    counter.increment();
    assertEq(counter.number(), 1);
}

// REVM-only version with advanced cheatcodes
function testIncrementWithSnapshot() public {
    vm.polkadotSkip();  // Skip Polkadot

    Counter counter = new Counter();
    uint256 snapshot = vm.snapshot();

    counter.increment();
    assertEq(counter.number(), 1);

    vm.revertTo(snapshot);
    assertEq(counter.number(), 0);
}
```

## Contract Persistence

### Why Persistence Matters

When switching between REVM and pallet-revive:
- Only **persistent** contracts migrate
- Non-persistent contracts are lost
- Use `vm.makePersistent` to mark contracts for migration

### Making Contracts Persistent

```solidity
function testWithPersistence() public {
    Counter counter = new Counter();

    // Mark for migration - REQUIRED for vm.polkadot(false)
    vm.makePersistent(address(counter));

    counter.increment();
    assertEq(counter.number(), 1);

    // Switch to REVM - counter migrates
    vm.polkadot(false);

    // State persists
    assertEq(counter.number(), 1);
}
```

### What Gets Migrated

For persistent contracts:
- ✅ **Balance**: Account balance
- ✅ **Nonce**: Transaction nonce
- ✅ **Bytecode**: Contract code
- ✅ **Storage**: All storage slots (including immutables)

For non-persistent contracts:
- ❌ Lost when switching VMs

### Multiple Contracts

```solidity
function testMultipleContracts() public {
    Counter counter1 = new Counter();
    Counter counter2 = new Counter();

    // Make both persistent
    vm.makePersistent(address(counter1));
    vm.makePersistent(address(counter2));

    counter1.increment();
    counter2.increment();

    vm.polkadot(false);

    // Both persist
    assertEq(counter1.number(), 1);
    assertEq(counter2.number(), 1);
}
```

### Persistence Best Practices

1. **Always persist contracts before switching**:
```solidity
// ❌ WRONG - contract lost
Counter counter = new Counter();
vm.polkadot(false);  // counter is lost!

// ✅ CORRECT
Counter counter = new Counter();
vm.makePersistent(address(counter));
vm.polkadot(false);  // counter migrates
```

2. **Persist in setUp for test-wide access**:
```solidity
Counter counter;

function setUp() public {
    counter = new Counter();
    vm.makePersistent(address(counter));
}

function testA() public {
    counter.increment();
    vm.polkadot(false);
    assertEq(counter.number(), 1);  // Works
}
```

3. **Persist helper contracts**:
```solidity
function testWithHelper() public {
    Counter counter = new Counter();
    Helper helper = new Helper(address(counter));

    // Persist both
    vm.makePersistent(address(counter));
    vm.makePersistent(address(helper));

    vm.polkadot(false);
    // Both accessible
}
```

## Common Patterns

### Pattern 1: Basic Contract Test

```solidity
contract CounterTest is DSTest {
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
        counter.decrement();
        assertEq(counter.number(), 0);
    }
}
```

**Usage**:
```bash
# REVM
forge test

# Polkadot
forge test --polkadot
```

### Pattern 2: Migration Test

```solidity
contract MigrationTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testStatePersists() public {
        SimpleStorage store = new SimpleStorage();
        vm.makePersistent(address(store));

        // Set value in Polkadot
        store.set(42);
        assertEq(store.get(), 42);

        // Switch to REVM
        vm.polkadot(false);

        // Value persists
        assertEq(store.get(), 42);
    }

    function testMultipleMigrations() public {
        SimpleStorage store = new SimpleStorage();
        vm.makePersistent(address(store));

        store.set(1);

        // Polkadot → REVM
        vm.polkadot(false);
        assertEq(store.get(), 1);

        // REVM → Polkadot
        vm.polkadot(true);
        store.set(2);

        // Polkadot → REVM
        vm.polkadot(false);
        assertEq(store.get(), 2);
    }
}
```

### Pattern 3: Complex Storage

```solidity
contract StorageTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testComplexStorage() public {
        Store store = new Store();
        vm.makePersistent(address(store));

        // Store various types
        store.setUint(42);
        store.setAddress(address(0x1234));
        store.setBytes32(keccak256("test"));
        store.setString("hello");

        // Migrate
        vm.polkadot(false);

        // All values persist
        assertEq(store.getUint(), 42);
        assertEq(store.getAddress(), address(0x1234));
        assertEq(store.getBytes32(), keccak256("test"));
        assertEq(store.getString(), "hello");
    }

    function testMapping() public {
        Store store = new Store();
        vm.makePersistent(address(store));

        // Set mapping values
        store.setMapping(1, 100);
        store.setMapping(2, 200);
        store.setMapping(3, 300);

        vm.polkadot(false);

        // Mapping values persist
        assertEq(store.getMapping(1), 100);
        assertEq(store.getMapping(2), 200);
        assertEq(store.getMapping(3), 300);
    }
}
```

### Pattern 4: Contract Interactions

```solidity
contract InteractionTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testContractCall() public {
        Caller caller = new Caller();
        Callee callee = new Callee();

        vm.makePersistent(address(caller));
        vm.makePersistent(address(callee));

        // Caller calls Callee
        caller.callCallee(address(callee), 42);

        // Check state in both contracts
        assertEq(callee.value(), 42);
        assertTrue(caller.lastCallSuccess());
    }

    function testDelegateCall() public {
        Proxy proxy = new Proxy();
        Implementation impl = new Implementation();

        vm.makePersistent(address(proxy));
        vm.makePersistent(address(impl));

        // Proxy delegates to Implementation
        proxy.delegateToImpl(address(impl), 42);

        // State stored in Proxy
        assertEq(proxy.value(), 42);
    }
}
```

### Pattern 5: Event Testing

```solidity
contract EventTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testEvent() public {
        Counter counter = new Counter();

        // Expect event
        vm.expectEmit(true, true, false, true);
        emit Incremented(1);

        counter.increment();
    }

    function testMultipleEvents() public {
        Counter counter = new Counter();

        vm.expectEmit(true, true, false, true);
        emit Incremented(1);
        counter.increment();

        vm.expectEmit(true, true, false, true);
        emit Incremented(2);
        counter.increment();
    }
}
```

### Pattern 6: Revert Testing

```solidity
contract RevertTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testRevertWithMessage() public {
        Counter counter = new Counter();

        vm.expectRevert("Counter: already at zero");
        counter.decrement();  // Reverts when count is 0
    }

    function testRevertWithoutMessage() public {
        Counter counter = new Counter();

        vm.expectRevert();
        counter.decrement();
    }

    function testRevertWithCustomError() public {
        Counter counter = new Counter();

        vm.expectRevert(Counter.Underflow.selector);
        counter.decrement();
    }
}
```

### Pattern 7: Time and Block Manipulation

```solidity
contract TimeTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testTimestamp() public {
        TimeLock lock = new TimeLock();

        // Set timestamp
        vm.warp(1000000);

        lock.lock();
        assertFalse(lock.canUnlock());

        // Fast forward
        vm.warp(2000000);
        assertTrue(lock.canUnlock());
    }

    function testBlockNumber() public {
        BlockChecker checker = new BlockChecker();

        // Set block number
        vm.roll(100);

        assertEq(checker.currentBlock(), 100);

        // Advance blocks
        vm.roll(200);
        assertEq(checker.currentBlock(), 200);
    }
}
```

**Important**: Timestamps and block numbers are clamped to u64::MAX in Polkadot mode.

### Pattern 8: Balance and Ether

```solidity
contract BalanceTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testDeal() public {
        address user = address(0x1234);

        // Set balance
        vm.deal(user, 10 ether);
        assertEq(user.balance, 10 ether);
    }

    function testPayable() public {
        Payable payable = new Payable();
        vm.makePersistent(address(payable));

        vm.deal(address(this), 10 ether);

        payable.deposit{value: 1 ether}();
        assertEq(address(payable).balance, 1 ether);

        vm.polkadot(false);
        assertEq(address(payable).balance, 1 ether);
    }
}
```

**Important**: Balances are clamped to u128::MAX in Polkadot mode.

## Assertions and Checks

### DSTest Assertions

**Boolean**:
```solidity
assertTrue(condition);
assertFalse(condition);
```

**Equality**:
```solidity
assertEq(uint a, uint b);
assertEq(int a, int b);
assertEq(address a, address b);
assertEq(bytes32 a, bytes32 b);
assertEq(string memory a, string memory b);
assertEq(bytes memory a, bytes memory b);
```

**Inequality**:
```solidity
// Use assertTrue with comparison
assertTrue(a != b);
assertTrue(a > b);
assertTrue(a < b);
assertTrue(a >= b);
assertTrue(a <= b);
```

**Failure**:
```solidity
fail("Explicit failure message");
```

### Custom Assertions

```solidity
function assertApproxEq(uint a, uint b, uint maxDelta) internal {
    uint delta = a > b ? a - b : b - a;
    assertTrue(delta <= maxDelta);
}

function assertBetween(uint value, uint min, uint max) internal {
    assertTrue(value >= min && value <= max);
}
```

### Checking State

```solidity
function testStateChecks() public {
    Counter counter = new Counter();
    vm.makePersistent(address(counter));

    // Check initial state
    assertEq(counter.number(), 0);

    counter.increment();

    // Check after operation
    assertEq(counter.number(), 1);

    vm.polkadot(false);

    // Check after migration
    assertEq(counter.number(), 1);
}
```

## Advanced Patterns

### Pattern: Testing with Multiple Accounts

```solidity
contract MultiAccountTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testMultipleAccounts() public {
        Counter counter = new Counter();

        address alice = address(0x1);
        address bob = address(0x2);

        // Alice increments
        vm.prank(alice);
        counter.increment();

        // Bob increments
        vm.prank(bob);
        counter.increment();

        assertEq(counter.number(), 2);
    }
}
```

### Pattern: Testing with Mock Calls

```solidity
contract MockTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testMockCall() public {
        Counter counter = new Counter();

        // Mock the number() call
        vm.mockCall(
            address(counter),
            abi.encodeWithSelector(Counter.number.selector),
            abi.encode(999)
        );

        assertEq(counter.number(), 999);  // Returns mocked value

        vm.clearMockedCalls();
        assertEq(counter.number(), 0);  // Returns real value
    }
}
```

### Pattern: Testing with Labels

```solidity
contract LabelTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testWithLabels() public {
        Counter counter = new Counter();

        // Add labels for better traces
        vm.label(address(counter), "Counter");
        vm.label(address(this), "Test");

        counter.increment();

        // Traces show "Counter" instead of address
    }
}
```

### Pattern: Fuzz Testing

```solidity
contract FuzzTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testFuzzIncrement(uint256 iterations) public {
        // For Polkadot, consider setting max_fuzz_int in foundry.toml
        Counter counter = new Counter();

        for (uint256 i = 0; i < iterations && i < 1000; i++) {
            counter.increment();
        }

        assertTrue(counter.number() >= 0);
    }

    function testFuzzBalance(uint128 amount) public {
        // Use u128 for balance compatibility
        address user = address(0x1);
        vm.deal(user, amount);
        assertEq(user.balance, amount);
    }
}
```

**Note**: Use `uint128` for balances and `uint64` for timestamps/block numbers when fuzzing for Polkadot compatibility. Alternatively, set `max_fuzz_int` in `foundry.toml` to limit fuzz values.

## Numeric Limits and Compatibility

### Type Constraints

| Type | Standard EVM | Polkadot | Behavior |
|------|--------------|----------|----------|
| Balance | `uint256` | `uint128` | Values > u128::MAX clamped with warning |
| Block number | `uint256` | `uint64` | Values > u64::MAX clamped with warning |
| Timestamp | `uint256` | `uint64` | Values > u64::MAX clamped with warning |
| Gas | `uint256` | `uint64` | Values > u64::MAX clamped with warning |

### Writing Compatible Tests

**Use appropriate types for fuzzing**:
```solidity
// ✅ GOOD - u128 for balances
function testBalance(uint128 amount) public {
    vm.deal(address(0x1), amount);
}

// ❌ BAD - u256 may exceed limit
function testBalance(uint256 amount) public {
    vm.deal(address(0x1), amount);  // Warning if amount > u128::MAX
}

// ✅ GOOD - u64 for timestamps
function testTime(uint64 timestamp) public {
    vm.warp(timestamp);
}

// ❌ BAD - u256 may exceed limit
function testTime(uint256 timestamp) public {
    vm.warp(timestamp);  // Warning if timestamp > u64::MAX
}
```

### Handling Warnings

```solidity
function testLargeValues() public {
    // This will generate warnings in Polkadot mode
    uint256 largeValue = type(uint256).max;

    // Value is clamped
    vm.warp(largeValue);  // Clamped to u64::MAX

    // Test continues with clamped value
    assertTrue(block.timestamp == type(uint64).max);
}
```

## Testing Checklist

Before committing tests:

- [ ] Uses `DSTest` as base contract
- [ ] Imports `ds-test/test.sol` and `cheats/Vm.sol`
- [ ] Declares `Vm constant vm = Vm(HEVM_ADDRESS);`
- [ ] Uses `vm.makePersistent` before switching VMs
- [ ] Uses appropriate types (u128 for balances, u64 for timestamps)
- [ ] Tests pass in REVM: `forge test`
- [ ] Tests pass in Polkadot: `forge test --polkadot`
- [ ] No warnings about clamped values
- [ ] Assertions are clear and meaningful

## Examples from Codebase

### EvmToReviveMigration.t.sol

```solidity
contract EvmToReviveMigrationTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testBasicMigration() public {
        SimpleStorage store = new SimpleStorage();
        vm.makePersistent(address(store));

        store.set(42);
        assertEq(store.get(), 42);

        vm.polkadot(false);
        assertEq(store.get(), 42);
    }
}
```

### MockCall.t.sol

```solidity
contract MockCallTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testMockCall() public {
        Callee callee = new Callee();

        vm.mockCall(
            address(callee),
            abi.encodeWithSelector(Callee.getValue.selector),
            abi.encode(999)
        );

        assertEq(callee.getValue(), 999);
    }
}
```

### Store.t.sol

```solidity
contract StoreTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testStorageManipulation() public {
        Store store = new Store();

        vm.store(
            address(store),
            bytes32(uint256(0)),
            bytes32(uint256(42))
        );

        assertEq(store.getValue(), 42);
    }
}
```

## Common Mistakes

### ❌ Mistake 1: Forgetting makePersistent

```solidity
// ❌ WRONG
function testMigration() public {
    Counter counter = new Counter();
    counter.increment();

    vm.polkadot(false);  // Counter is lost!
    assertEq(counter.number(), 1);  // Fails
}

// ✅ CORRECT
function testMigration() public {
    Counter counter = new Counter();
    vm.makePersistent(address(counter));

    counter.increment();
    vm.polkadot(false);
    assertEq(counter.number(), 1);  // Works
}
```

### ❌ Mistake 2: Using Wrong Base Contract

```solidity
// ❌ WRONG - Don't use forge-std/Test
import "forge-std/Test.sol";
contract MyTest is Test { }

// ✅ CORRECT - Use DSTest
import "ds-test/test.sol";
import "cheats/Vm.sol";
contract MyTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);
}
```

### ❌ Mistake 3: Large Numeric Values

```solidity
// ❌ BAD - Will cause warnings
function testTimestamp() public {
    vm.warp(type(uint256).max);  // > u64::MAX
}

// ✅ GOOD
function testTimestamp() public {
    vm.warp(1234567890);  // Within u64 range
}
```

### ❌ Mistake 4: Not Testing Both Modes

```solidity
// ❌ BAD - Only tests one mode
forge test  // or forge test --polkadot

// ✅ GOOD - Test both
forge test              // REVM
forge test --polkadot  // Polkadot
```

## References

- [Running Tests Guide](running-tests.md) - How to execute tests
- [Integration Tests Guide](integration-tests.md) - Adding Solidity + Rust test pairs
- [State Migration Guide](state-migration.md) - Understanding state synchronization
- [Troubleshooting Guide](troubleshooting.md) - Common issues and solutions
- [DSTest Documentation](https://book.getfoundry.sh/reference/ds-test) - DSTest API reference

## Related Files

- [testdata/default/revive/](../../../testdata/default/revive/) - Example test contracts
- [crates/forge/tests/it/revive/](../../../crates/forge/tests/it/revive/) - Rust test wrappers
- [CLAUDE.md](../../../CLAUDE.md) - Project documentation with cheatcode reference
