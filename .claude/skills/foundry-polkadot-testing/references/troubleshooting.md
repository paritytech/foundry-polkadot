# Troubleshooting: Comprehensive Guide

<nav>
  <tag name="test-failures" section="Test Failures" />
  <tag name="numeric-issues" section="Numeric Limits" />
  <tag name="state-sync" section="State Synchronization" />
  <tag name="bytecode" section="Bytecode Issues" />
  <tag name="debugging" section="Debugging Techniques" />
</nav>

<atom>
  <type>reference</type>
  <diataxis_quadrant>how-to</diataxis_quadrant>
  <intent>troubleshoot_polkadot_tests</intent>
</atom>

## Overview

This guide covers common issues when testing Solidity contracts against Polkadot's pallet-revive runtime and their solutions.

## Test Failures

### Issue: Tests Pass in REVM but Fail with --polkadot

**Symptoms**:
```bash
# ✅ Passes
forge test --match-test testMyFunction

# ❌ Fails
forge test --polkadot --match-test testMyFunction
```

**Common Causes**:

#### 1. Numeric Overflow

**Problem**: Values exceed Polkadot limits

```solidity
function testLargeTimestamp() public {
    // ❌ Exceeds u64::MAX
    vm.warp(type(uint256).max);
    // Value clamped to u64::MAX with warning
}
```

**Solution**: Use appropriate ranges

```solidity
function testTimestamp() public {
    // ✅ Within u64 range
    vm.warp(1234567890);
}
```

#### 2. State Synchronization Issues

**Problem**: Contract not persistent

```solidity
function testMigration() public {
    Counter counter = new Counter();
    // ❌ Missing: vm.makePersistent(address(counter));

    counter.increment();
    vm.polkadot(false);

    // Fails - counter lost
    assertEq(counter.number(), 1);
}
```

**Solution**: Mark contracts persistent

```solidity
function testMigration() public {
    Counter counter = new Counter();
    vm.makePersistent(address(counter));  // ✅ Add this

    counter.increment();
    vm.polkadot(false);

    assertEq(counter.number(), 1);  // ✅ Passes
}
```

#### 3. Cheatcodes Not Available in Polkadot

**Symptoms**:
```
[FAIL: Cheatcodes are not available in polkadot runtime.]
```

**Root Cause**: The cheatcode precompile (at `HEVM_ADDRESS`) only exists in REVM, not in pallet-revive

**Technical Details**:
- State is synced bidirectionally between REVM and pallet-revive
- The cheatcode precompile address itself is REVM-only
- When contracts in pallet-revive try to call `HEVM_ADDRESS`, it doesn't exist

**Solution**: Use `vm.polkadotSkip()` to keep contracts in REVM where cheatcodes exist

```solidity
function testWithSnapshot() public {
    // Keep contracts in REVM where HEVM_ADDRESS exists
    vm.polkadotSkip();

    Counter counter = new Counter();

    // Now snapshot/revert works
    uint256 snapshot = vm.snapshot();
    counter.increment();
    vm.revertTo(snapshot);

    assertEq(counter.number(), 0);
}
```

**Common cheatcodes that need skip**:
- `vm.snapshot()` / `vm.revertTo()`
- Complex `vm.prank()` scenarios
- `vm.record()` / `vm.accesses()`
- Some advanced cheatcodes

**Best Practice**: Create two versions when possible

```solidity
// ✅ Works in both modes
function testIncrement() public {
    Counter counter = new Counter();
    counter.increment();
    assertEq(counter.number(), 1);
}

// ✅ REVM-only with advanced features
function testIncrementWithSnapshot() public {
    vm.polkadotSkip();  // Skip Polkadot
    // ... advanced cheatcode usage
}
```

#### 4. Gas Estimation Differences

**Problem**: Gas model differs between REVM and pallet-revive

**Solution**: Don't rely on exact gas values

```solidity
// ❌ BAD - Exact gas assertion
function testGas() public {
    uint256 gasStart = gasleft();
    counter.increment();
    uint256 gasUsed = gasStart - gasleft();
    assertEq(gasUsed, 21000);  // Fails in Polkadot
}

// ✅ GOOD - Range assertion
function testGas() public {
    uint256 gasStart = gasleft();
    counter.increment();
    uint256 gasUsed = gasStart - gasleft();
    assertTrue(gasUsed > 0);  // Passes everywhere
}
```

### Issue: PVM Tests Fail but EVM Tests Pass

**Symptoms**:
```bash
# ✅ EVM passes
cargo test --package forge --test it test_my_feature::evm

# ❌ PVM fails
cargo test --package forge --test it test_my_feature::pvm
```

**Common Causes**:

#### 1. PVM Bytecode Not Compiled

**Problem**: Missing `--resolc` compilation

**Solution**: Compile with resolc

```bash
# Build PVM bytecode
forge build --resolc

# Verify PVM bytecode (should start with 0x505)
forge inspect Counter bytecode --resolc | head -c 10

# Test PVM mode
forge test --polkadot=pvm
```

#### 2. Library Incompatibility

**Problem**: PVM doesn't support certain libraries

```solidity
// May not work in PVM
import "openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
```

**Solution**: Test EVM mode only or simplify

```rust
// Test EVM only
#[tokio::test(flavor = "multi_thread")]
async fn test_evm_only() {
    let runtime_mode = ReviveRuntimeMode::Evm;
    // ...
}
```

#### 3. Proxy Pattern Issues

**Problem**: PVM has limited proxy support

**Solution**: Avoid proxy patterns or test EVM only

### Issue: Rust Integration Test Not Finding Solidity Test

**Symptoms**:
```bash
cargo test --package forge --test it test_my_feature
# Error: No tests match filter
```

**Cause**: Incorrect filter path

**Solution**: Use correct glob pattern

```rust
// ❌ WRONG - Missing path
let filter = Filter::new(".*", ".*", "MyTest.t.sol");

// ✅ CORRECT - Full path with glob
let filter = Filter::new(".*", ".*", ".*/revive/MyTest.t.sol");
```

### Issue: Module Not Found Error

**Symptoms**:
```bash
cargo test
# Error: unresolved import `crate::revive::my_feature`
```

**Cause**: Module not registered

**Solution**: Add to `mod.rs`

```rust
// crates/forge/tests/it/revive/mod.rs
pub mod my_feature;  // ← Add this line
```

## Numeric Limits

### Understanding Limits

| Type | REVM | Polkadot | Overflow Behavior |
|------|------|----------|-------------------|
| Balance | `uint256` | `uint128` | Clamped to u128::MAX with warning |
| Block number | `uint256` | `uint64` | Clamped to u64::MAX with warning |
| Timestamp | `uint256` | `uint64` | Clamped to u64::MAX with warning |
| Gas | `uint256` | `uint64` | Clamped to u64::MAX with warning |

### Detecting Overflow Warnings

```bash
# Run with verbosity to see warnings
forge test --polkadot -vv

# Look for:
# Warning: Block number exceeds u64::MAX, clamping to 18446744073709551615
# Warning: Timestamp exceeds u64::MAX, clamping to 18446744073709551615
# Warning: Balance exceeds u128::MAX, clamping to 340282366920938463463374607431768211455
```

### Fixing Overflow Issues

#### Fix 1: Use Appropriate Types in Fuzz Tests

```solidity
// ❌ BAD - u256 may overflow
function testFuzzTimestamp(uint256 timestamp) public {
    vm.warp(timestamp);  // Warning if timestamp > u64::MAX
}

// ✅ GOOD - u64 always fits
function testFuzzTimestamp(uint64 timestamp) public {
    vm.warp(timestamp);  // No warning
}

// ❌ BAD - u256 may overflow for balance
function testFuzzBalance(uint256 amount) public {
    vm.deal(address(0x1), amount);  // Warning if amount > u128::MAX
}

// ✅ GOOD - u128 always fits
function testFuzzBalance(uint128 amount) public {
    vm.deal(address(0x1), amount);  // No warning
}
```

#### Fix 2: Clamp Values Explicitly

```solidity
function testWithLargeValues(uint256 value) public {
    // Clamp to u64::MAX
    uint64 timestamp = value > type(uint64).max
        ? type(uint64).max
        : uint64(value);

    vm.warp(timestamp);  // No warning
}
```

#### Fix 3: Use max_fuzz_int in foundry.toml

```toml
# foundry.toml - limit fuzz values for Polkadot compatibility
[fuzz]
max_fuzz_int = "18446744073709551615"  # u64::MAX for timestamps/block numbers

# Or use u128::MAX for balance tests
max_fuzz_int = "340282366920938463463374607431768211455"  # u128::MAX
```

When set, unsigned integers are clamped to `[0, max_fuzz_int]` and signed integers are clamped to `[-(max_fuzz_int+1), max_fuzz_int]` to match real signed type ranges (e.g., i64 range is `[-2^63, 2^63-1]`).

### Testing Numeric Edge Cases

```solidity
function testMaxU64Timestamp() public {
    // Test at u64::MAX
    vm.warp(type(uint64).max);
    assertEq(block.timestamp, type(uint64).max);
}

function testMaxU128Balance() public {
    // Test at u128::MAX
    address user = address(0x1);
    vm.deal(user, type(uint128).max);
    assertEq(user.balance, type(uint128).max);
}

function testOverflowClamping() public {
    // Test overflow behavior
    vm.warp(type(uint256).max);  // Clamped to u64::MAX
    assertEq(block.timestamp, type(uint64).max);
}
```

## State Synchronization

### Issue: Stale State in Assertions

**Problem**: Assertion sees old state

```solidity
function testStaleState() public {
    Counter counter = new Counter();

    counter.increment();

    // ❌ Might fail if state not synced
    assertEq(counter.number(), 1);
}
```

**Cause**: State not synced back from pallet-revive

**Solution**: This should work automatically (continuous sync), but if issues persist:

```solidity
// Workaround: Explicit state check
function testWithExplicitCheck() public {
    Counter counter = new Counter();

    counter.increment();

    // Force state refresh by reading
    uint256 value = counter.number();
    assertEq(value, 1);
}
```

### Issue: Contract Lost After vm.polkadot(false)

**Problem**: Contract not accessible after switching

```solidity
function testLostContract() public {
    Counter counter = new Counter();
    // ❌ Missing: vm.makePersistent(address(counter));

    counter.increment();

    vm.polkadot(false);

    // Fails - counter doesn't exist in REVM
    counter.number();
}
```

**Solution**: Use `vm.makePersistent`

```solidity
function testPersistentContract() public {
    Counter counter = new Counter();
    vm.makePersistent(address(counter));  // ✅ Mark persistent

    counter.increment();

    vm.polkadot(false);

    assertEq(counter.number(), 1);  // ✅ Works
}
```

### Issue: Storage Not Migrating

**Problem**: Storage values don't persist after migration

**Symptoms**:
```solidity
function testStorageMigration() public {
    Store store = new Store();
    vm.makePersistent(address(store));

    store.setValue(42);
    assertEq(store.getValue(), 42);  // ✅ Passes

    vm.polkadot(false);

    assertEq(store.getValue(), 42);  // ❌ Fails - shows 0
}
```

**Cause**: Storage not properly migrated

**Debug Steps**:

1. **Check if contract is persistent**:
```solidity
assertTrue(vm.isPersistent(address(store)));
```

2. **Verify storage before switch**:
```solidity
uint256 before = store.getValue();
emit log_uint(before);  // Should show 42

vm.polkadot(false);

uint256 after = store.getValue();
emit log_uint(after);  // Should show 42
```

3. **Run with debug logs**:
```bash
env RUST_LOG=warn,revive_strategy=debug \
  cargo test --package forge --test it test_storage_migration -- --nocapture
```

## Bytecode Issues

### Issue: PVM Bytecode Not Found

**Symptoms**:
```bash
forge test --polkadot=pvm
# Error: PVM bytecode not found for contract
```

**Cause**: Missing `--resolc` compilation

**Solution**:

```bash
# 1. Compile with resolc
forge build --resolc

# 2. Verify PVM bytecode exists
forge inspect Counter bytecode --resolc

# 3. Check for 0x505 prefix (PVM marker)
forge inspect Counter bytecode --resolc | grep "^0x505"

# 4. Test PVM mode
forge test --polkadot=pvm -vv
```

### Issue: Bytecode Too Large

**Symptoms**:
```bash
# Error: Contract bytecode too large
```

**Cause**: Contract exceeds size limit

**Solutions**:

1. **Enable optimizer**:
```toml
# foundry.toml
[profile.default]
optimizer = true
optimizer_runs = 200
```

2. **Split contract into smaller pieces**:
```solidity
// Instead of one large contract
contract Large {
    // Too much code
}

// Split into multiple contracts
contract Part1 { }
contract Part2 { }
contract Main {
    Part1 part1;
    Part2 part2;
}
```

3. **Use libraries**:
```solidity
library Utils {
    function helper() internal { }
}

contract Main {
    using Utils for *;
}
```

### Issue: Bytecode Mismatch

**Symptoms**: EVM and PVM behave differently

**Cause**: Bytecode compiled with different compilers

**Solution**: Ensure consistent compilation

```bash
# EVM bytecode (default)
forge build

# PVM bytecode
forge build --resolc

# Both should be present for dual-mode tests
```

## Debugging Techniques

### Technique 1: Compare REVM vs Polkadot

```bash
# Test in REVM
forge test --match-test testMyFunction -vvv > revm.log

# Test in Polkadot
forge test --polkadot --match-test testMyFunction -vvv > polkadot.log

# Compare
diff revm.log polkadot.log
```

**Look for**:
- Different gas usage
- Different return values
- Different revert messages

### Technique 2: Enable Rust Logging

```bash
# Basic logging
env RUST_LOG=warn cargo test --package forge --test it test_my_feature -- --nocapture

# Detailed revive strategy logs
env RUST_LOG=warn,revive_strategy=info cargo test --package forge --test it test_my_feature -- --nocapture

# Debug level
env RUST_LOG=warn,revive_strategy=debug cargo test --package forge --test it test_my_feature -- --nocapture

# Trace everything
env RUST_LOG=trace cargo test --package forge --test it test_my_feature -- --nocapture
```

**Log Output Shows**:
- State migration events
- Storage synchronization
- Contract creation/calls
- Bytecode selection

### Technique 3: Add Solidity Logs

```solidity
import "forge-std/console.sol";

function testWithLogs() public {
    console.log("Starting test");

    Counter counter = new Counter();
    console.log("Counter deployed at:", address(counter));

    counter.increment();
    console.log("After increment:", counter.number());

    vm.polkadot(false);
    console.log("After migration:", counter.number());
}
```

Run with verbosity:
```bash
forge test --polkadot --match-test testWithLogs -vvv
```

### Technique 4: Use Labels

```solidity
function testWithLabels() public {
    Counter counter = new Counter();

    // Add labels for better trace readability
    vm.label(address(counter), "Counter");
    vm.label(address(this), "TestContract");

    counter.increment();

    // Traces show "Counter" instead of 0x123...
}
```

### Technique 5: Breakpoint Debugging

```solidity
function testWithBreakpoints() public {
    Counter counter = new Counter();

    // Checkpoint 1
    assertEq(counter.number(), 0);
    emit log("Checkpoint 1: initialized");

    counter.increment();

    // Checkpoint 2
    assertEq(counter.number(), 1);
    emit log("Checkpoint 2: after increment");

    vm.polkadot(false);

    // Checkpoint 3
    assertEq(counter.number(), 1);
    emit log("Checkpoint 3: after migration");
}
```

### Technique 6: Isolated Test Cases

```solidity
// ❌ BAD - Too many operations
function testEverything() public {
    Counter c1 = new Counter();
    Counter c2 = new Counter();

    c1.increment();
    c2.increment();

    vm.polkadot(false);

    c1.increment();
    c2.decrement();

    // Which operation failed?
}

// ✅ GOOD - One operation per test
function testIncrementC1() public {
    Counter c1 = new Counter();
    c1.increment();
    assertEq(c1.number(), 1);
}

function testMigration() public {
    Counter c1 = new Counter();
    vm.makePersistent(address(c1));
    c1.increment();
    vm.polkadot(false);
    assertEq(c1.number(), 1);
}
```

## Performance Issues

### Issue: Tests Running Slow

**Symptoms**: Tests take longer than expected

**Causes and Solutions**:

#### 1. Too Many Migrations

```solidity
// ❌ BAD - Migrates repeatedly
function testSlowMigrations() public {
    Counter counter = new Counter();
    vm.makePersistent(address(counter));

    for (uint i = 0; i < 100; i++) {
        vm.polkadot(true);
        counter.increment();
        vm.polkadot(false);  // Expensive migration each iteration
    }
}

// ✅ GOOD - Migrate once
function testFastMigrations() public {
    Counter counter = new Counter();
    vm.makePersistent(address(counter));

    for (uint i = 0; i < 100; i++) {
        counter.increment();  // Stay in pallet-revive
    }

    vm.polkadot(false);  // Single migration
}
```

#### 2. Unnecessary Persistence

```solidity
// ❌ BAD - Persists unused contracts
function testUnnecessaryPersistence() public {
    Counter c1 = new Counter();
    Counter c2 = new Counter();
    Counter c3 = new Counter();

    vm.makePersistent(address(c1));
    vm.makePersistent(address(c2));
    vm.makePersistent(address(c3));

    c1.increment();  // Only use c1

    vm.polkadot(false);  // Migrates all 3
}

// ✅ GOOD - Persist only what's needed
function testMinimalPersistence() public {
    Counter c1 = new Counter();
    Counter c2 = new Counter();
    Counter c3 = new Counter();

    vm.makePersistent(address(c1));  // Only c1

    c1.increment();

    vm.polkadot(false);  // Migrates only c1
}
```

#### 3. PVM Mode Overhead

**Solution**: Test EVM mode during development, PVM in CI

```bash
# Fast development iteration
forge test --polkadot  # EVM mode

# Comprehensive CI testing
forge test --polkadot=pvm  # PVM mode
```

### Issue: Compilation Slow

**Symptoms**: `forge build --resolc` takes long

**Solutions**:

1. **Cache artifacts**:
```bash
# Cache is automatic, but ensure it's not disabled
forge build --resolc
```

2. **Incremental builds**:
```bash
# Only rebuild changed files
forge build --resolc --force  # Force full rebuild only when needed
```

3. **Parallel compilation**:
```toml
# foundry.toml
[profile.default]
optimizer = true
optimizer_runs = 200
```

## Common Error Messages

### "Contract not persistent"

**Error**:
```
Error: Contract 0x123... not persistent, cannot migrate
```

**Solution**: Add `vm.makePersistent(address(contract))`

### "Cheatcodes are not available in polkadot runtime"

**Error**:
```
[FAIL: Cheatcodes are not available in polkadot runtime.]
```

**Root Cause**: The cheatcode precompile at `HEVM_ADDRESS` only exists in REVM, not in pallet-revive

**Solution**: Use `vm.polkadotSkip()` to keep contracts in REVM where `HEVM_ADDRESS` exists

```solidity
function testWithSnapshot() public {
    vm.polkadotSkip();  // Keep contracts in REVM where cheatcodes exist

    Counter counter = new Counter();  // Deployed in REVM, not pallet-revive

    // Contract can now call cheatcodes (HEVM_ADDRESS exists in REVM)
    uint256 snapshot = vm.snapshot();
    // ...
}
```

### "PVM bytecode not found"

**Error**:
```
Error: PVM bytecode not found for contract 0x123...
```

**Solution**: Run `forge build --resolc`

### "Block number exceeds u64::MAX"

**Warning**:
```
Warning: Block number exceeds u64::MAX, clamping to 18446744073709551615
```

**Solution**: Use smaller values or u64 type in fuzz tests

### "Balance exceeds u128::MAX"

**Warning**:
```
Warning: Balance exceeds u128::MAX, clamping to 340282366920938463463374607431768211455
```

**Solution**: Use u128 type in fuzz tests

### "No tests match filter"

**Error**:
```
Error: No tests match filter .*/MyTest.t.sol
```

**Solution**: Check Solidity file path and filter pattern

```rust
// Verify file exists
// testdata/default/revive/MyTest.t.sol

// Use correct filter
let filter = Filter::new(".*", ".*", ".*/revive/MyTest.t.sol");
```

## Debugging Checklist

When a test fails:

- [ ] Does it pass in REVM? (`forge test`)
- [ ] Does it pass in Polkadot EVM? (`forge test --polkadot`)
- [ ] Does it pass in Polkadot PVM? (`forge test --polkadot=pvm`)
- [ ] Are you using unsupported cheatcodes? (Add `vm.polkadotSkip()` if needed)
- [ ] Are contracts marked persistent? (`vm.makePersistent`)
- [ ] Are numeric values within limits? (u64, u128)
- [ ] Is PVM bytecode compiled? (`forge build --resolc`)
- [ ] Are there any warnings? (`-vv` flag)
- [ ] Do Rust logs show issues? (`RUST_LOG=debug`)
- [ ] Is the module registered? (`mod.rs`)
- [ ] Is the filter correct? (`.*/revive/...`)

## Getting Help

### Information to Provide

When reporting issues, include:

1. **Command used**:
```bash
forge test --polkadot --match-test testMyFunction -vvv
```

2. **Error output**:
```
[FAIL] testMyFunction()
  Error: Contract not persistent
```

3. **Test code**:
```solidity
function testMyFunction() public {
    // Minimal reproducible example
}
```

4. **Rust logs** (if applicable):
```bash
env RUST_LOG=warn,revive_strategy=info cargo test ... -- --nocapture
```

5. **Environment**:
- Foundry version
- Rust version
- OS

### Useful Commands for Diagnostics

```bash
# Foundry version
forge --version

# Rust version
cargo --version

# Show test output
forge test --polkadot --match-test testFailing -vvvvv

# Rust detailed logs
env RUST_LOG=trace cargo test --package forge --test it test_failing -- --nocapture

# Check bytecode
forge inspect Counter bytecode
forge inspect Counter bytecode --resolc

# List tests
forge test --list

# Show gas report
forge test --polkadot --gas-report
```

## References

- [Running Tests Guide](running-tests.md) - CLI flags and execution modes
- [Writing Tests Guide](writing-tests.md) - Test patterns and best practices
- [Integration Tests Guide](integration-tests.md) - Adding new tests
- [State Migration Guide](state-migration.md) - Understanding state sync
- [CLAUDE.md](../../../CLAUDE.md) - Project documentation

## Related Files

- [crates/revive-strategy/src/cheatcodes/mod.rs](../../../crates/revive-strategy/src/cheatcodes/mod.rs) - State sync implementation
- [testdata/default/revive/](../../../testdata/default/revive/) - Example tests
- [crates/forge/tests/it/revive/](../../../crates/forge/tests/it/revive/) - Integration tests
