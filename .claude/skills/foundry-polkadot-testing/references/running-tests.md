# Running Tests: Comprehensive Guide

<nav>
  <tag name="cli-flags" section="CLI Flags and Options" />
  <tag name="execution-modes" section="Execution Modes" />
  <tag name="test-selection" section="Test Selection" />
  <tag name="debugging" section="Debugging and Logging" />
  <tag name="ci-cd" section="CI/CD Integration" />
</nav>

<atom>
  <type>reference</type>
  <diataxis_quadrant>how-to</diataxis_quadrant>
  <intent>run_polkadot_tests</intent>
</atom>

## Overview

This guide covers running tests against Polkadot's pallet-revive runtime using `forge test` with the `--polkadot` flag.

## CLI Flags and Options

### Basic Usage

```bash
# Standard REVM (no Polkadot integration)
forge test

# Polkadot EVM runtime (PRODUCTION READY)
forge test --polkadot

# Explicit EVM mode
forge test --polkadot=evm

# Polkadot PVM runtime (EXPERIMENTAL)
forge test --polkadot=pvm
```

### Verbosity Levels

```bash
# Minimal output (default)
forge test --polkadot

# Show test results (-v)
forge test --polkadot -v

# Show failing test logs (-vv)
forge test --polkadot -vv

# Show all test logs (-vvv)
forge test --polkadot -vvv

# Show all logs + traces (-vvvv)
forge test --polkadot -vvvv

# Maximum verbosity (-vvvvv)
forge test --polkadot -vvvvv
```

### Test Selection

```bash
# Run specific test by name
forge test --polkadot --match-test testMyFunction

# Run tests matching pattern
forge test --polkadot --match-test "test_.*_migration"

# Run specific contract's tests
forge test --polkadot --match-contract MyTestContract

# Run tests in specific file
forge test --polkadot --match-path "testdata/default/revive/Store.t.sol"

# Exclude tests
forge test --polkadot --no-match-test testSkipThis
```

### Fuzzing Options

```bash
# Number of fuzz runs (default: 256)
forge test --polkadot --fuzz-runs 1000

# Maximum fuzz test rejections
forge test --polkadot --fuzz-max-test-rejects 65536
```

**Tip**: Set `max_fuzz_int` in foundry.toml to limit fuzz values for Polkadot compatibility:
```toml
[fuzz]
# Limit to u128::MAX for balance compatibility
max_fuzz_int = "340282366920938463463374607431768211455"
```

### Gas Reporting

```bash
# Enable gas reporting
forge test --polkadot --gas-report

# Save gas report to file
forge test --polkadot --gas-report --gas-report-file gas-report.txt
```

**Note**: Gas model is not fully aligned with production Polkadot.

## Execution Modes

### Mode Comparison

| Mode | Flag | Backend | Bytecode | Status | Use Case |
|------|------|---------|----------|--------|----------|
| **REVM** | (none) | Foundry REVM | EVM | Stable | Standard Ethereum development |
| **Polkadot EVM** | `--polkadot` or `--polkadot=evm` | pallet-revive EVM | EVM | Production | Ethereum-compatible Polkadot |
| **Polkadot PVM** | `--polkadot=pvm` | PolkaVM (RISC-V) | PVM | Experimental | RISC-V experimentation |

### REVM Mode (Default)

**When to use**:
- Standard Ethereum contract development
- No Polkadot-specific features needed
- Maximum compatibility

**Characteristics**:
- Runs entirely in Foundry's REVM
- No pallet-revive integration
- Standard EVM semantics
- Full `uint256` support

```bash
# Standard REVM mode
forge test

# All forge-std features work
forge test -vvv
```

### Polkadot EVM Mode (Production)

**When to use**:
- Testing contracts for Polkadot deployment
- Verifying Ethereum compatibility
- Production-ready testing

**Characteristics**:
- Ethereum-compatible
- Stable and deterministic
- Uses EVM bytecode
- Numeric limits: u64 (block numbers/timestamps), u128 (balances)

```bash
# Polkadot EVM mode
forge test --polkadot

# Equivalent
forge test --polkadot=evm
```

**Numeric Behavior**:
```solidity
// Values exceeding limits are clamped with warnings
uint256 largeValue = type(uint256).max;  // Clamped to u128::MAX for balances
block.timestamp = 2**65;  // Clamped to u64::MAX with warning
```

### Polkadot PVM Mode (Experimental)

**When to use**:
- Research and experimentation
- Testing RISC-V specific features
- Advanced users only

**Characteristics**:
- Experimental - may not work with all contracts
- Uses PolkaVM (RISC-V) backend
- Requires `--resolc` compilation
- Limited library/proxy support

```bash
# Polkadot PVM mode
forge test --polkadot=pvm

# Build PVM bytecode first
forge build --resolc
forge test --polkadot=pvm
```

**Limitations**:
- May not work with complex libraries
- Proxy patterns may fail
- Less battle-tested than EVM mode

## Test Execution Flow

### Lifecycle

```
1. Test Setup (REVM)
   └─ setUp() runs in Foundry REVM

2. Test Execution (REVM or Polkadot)
   ├─ Without --polkadot: stays in REVM
   └─ With --polkadot: switches to pallet-revive

3. Contract Deployment (Polkadot)
   ├─ CREATE/CREATE2 intercepted
   └─ Contract deployed in pallet-revive

4. Contract Calls (Polkadot)
   ├─ CALL/STATICCALL/DELEGATECALL intercepted
   ├─ Executed in pallet-revive
   └─ State syncs back to REVM

5. Test Completion
   └─ Assertions checked in REVM
```

### What Runs Where

**REVM (Always)**:
- Test contract itself
- DSTest assertion helpers
- Cheatcodes (`vm.*`)
- `setUp()` function

**pallet-revive (When `--polkadot` active)**:
- User contracts deployed with `new`
- Contract calls via interfaces
- State changes

**Example**:
```solidity
contract MyTest is DSTest {  // ← Runs in REVM
    Vm constant vm = Vm(HEVM_ADDRESS);  // ← Runs in REVM

    Counter counter;  // ← Storage in REVM

    function setUp() public {  // ← Runs in REVM
        // When --polkadot is active, this deploys in pallet-revive
        counter = new Counter();
    }

    function testIncrement() public {  // ← Runs in REVM
        counter.increment();  // ← Executes in pallet-revive
        assertEq(counter.number(), 1);  // ← Assertion in REVM
    }
}
```

## Rust Integration Tests

### Running Rust Wrappers

Rust integration tests wrap Solidity tests for both EVM and PVM modes:

```bash
# Run all revive integration tests
cargo test --package forge --test it test_revive_

# Run specific test
cargo test --package forge --test it test_revive_basic_migration

# Run with logging
env RUST_LOG=warn cargo test --package forge --test it test_revive_basic_migration -- --nocapture

# Run with detailed revive logs
env RUST_LOG=warn,revive_strategy=info \
  cargo test --package forge --test it test_revive_basic_migration -- --nocapture

# Run with trace-level logging (very verbose)
env RUST_LOG=trace cargo test --package forge --test it test_revive_basic_migration -- --nocapture
```

### Test Structure

Rust tests use `rstest` to parameterize EVM/PVM modes:

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

This single test runs twice:
1. `test_revive_basic_migration::pvm` - PVM mode
2. `test_revive_basic_migration::evm` - EVM mode

## Debugging and Logging

### Forge Debug Output

```bash
# Show failed test traces
forge test --polkadot -vv

# Show all test traces
forge test --polkadot -vvv

# Show setup traces
forge test --polkadot -vvvv

# Maximum detail
forge test --polkadot -vvvvv
```

### Rust Logging

Control Rust-side logging with `RUST_LOG`:

```bash
# Warn level (errors + warnings only)
env RUST_LOG=warn cargo test --package forge --test it test_revive_basic_migration -- --nocapture

# Info level (+ info messages)
env RUST_LOG=info cargo test --package forge --test it test_revive_basic_migration -- --nocapture

# Debug level (+ debug messages)
env RUST_LOG=debug cargo test --package forge --test it test_revive_basic_migration -- --nocapture

# Trace level (everything)
env RUST_LOG=trace cargo test --package forge --test it test_revive_basic_migration -- --nocapture
```

### Selective Module Logging

```bash
# Revive strategy only
env RUST_LOG=warn,revive_strategy=info cargo test ...

# Multiple modules
env RUST_LOG=warn,revive_strategy=info,foundry_evm=debug cargo test ...

# Trace specific module
env RUST_LOG=warn,revive_strategy=trace cargo test ...
```

### Common Debug Patterns

**Issue: Test passes in REVM but fails with --polkadot**

```bash
# Run both modes for comparison
forge test --match-test testMyFunction -vvv
forge test --polkadot --match-test testMyFunction -vvv

# Check state sync with Rust logs
env RUST_LOG=warn,revive_strategy=info \
  cargo test --package forge --test it test_my_function -- --nocapture
```

**Issue: Numeric overflow warnings**

```bash
# Enable warnings
forge test --polkadot -vv

# Look for messages like:
# "Warning: Block number exceeds u64::MAX, clamping to 18446744073709551615"
```

**Issue: PVM bytecode not found**

```bash
# Verify PVM compilation
forge build --resolc
forge inspect Counter bytecode --resolc | head -c 10
# Should output: 0x505...

# Test with PVM
forge test --polkadot=pvm -vvv
```

## Test Selection Strategies

### By Function Name

```bash
# Exact match
forge test --polkadot --match-test testIncrement

# Regex pattern
forge test --polkadot --match-test "test.*migration"

# Multiple tests (run separately)
forge test --polkadot --match-test testA
forge test --polkadot --match-test testB
```

### By Contract

```bash
# Single contract
forge test --polkadot --match-contract CounterTest

# Pattern matching
forge test --polkadot --match-contract ".*Migration.*"
```

### By File Path

```bash
# Specific file
forge test --polkadot --match-path "testdata/default/revive/Store.t.sol"

# All revive tests
forge test --polkadot --match-path "testdata/default/revive/**"

# Exclude files
forge test --polkadot --no-match-path "testdata/default/revive/Experimental.t.sol"
```

### Combined Filters

```bash
# Contract + test name
forge test --polkadot \
  --match-contract CounterTest \
  --match-test testIncrement

# Path + test pattern
forge test --polkadot \
  --match-path "testdata/default/revive/**" \
  --match-test "test_.*_migration"
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Polkadot Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true

      - name: Install Foundry
        uses: foundry-rs/foundry-toolchain@v1

      - name: Run REVM tests
        run: forge test

      - name: Run Polkadot EVM tests
        run: forge test --polkadot

      - name: Run Rust integration tests
        run: cargo test --package forge --test it test_revive_

      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Run clippy
        run: cargo clippy --workspace -- -D warnings
```

### Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

echo "Running tests..."

# Quick smoke test
forge test --polkadot || exit 1

# Format check
cargo fmt --all -- --check || exit 1

# Clippy
cargo clippy --workspace -- -D warnings || exit 1

echo "All checks passed!"
```

## Performance Considerations

### Test Speed

| Mode | Relative Speed | Use Case |
|------|----------------|----------|
| REVM | 1x (fastest) | Development iteration |
| Polkadot EVM | 2-3x slower | Pre-commit validation |
| Polkadot PVM | 3-5x slower | Pre-merge validation |

### Optimization Strategies

**1. Use REVM for rapid iteration**:
```bash
# Fast feedback loop
forge test --match-test testMyFunction -vv
```

**2. Run Polkadot tests before commit**:
```bash
# Comprehensive validation
forge test --polkadot
```

**3. Parallelize in CI**:
```yaml
strategy:
  matrix:
    mode: [revm, polkadot-evm, polkadot-pvm]
steps:
  - run: forge test ${{ matrix.mode }}
```

**4. Cache dependencies**:
```yaml
- uses: actions/cache@v3
  with:
    path: |
      ~/.cargo
      target/
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
```

## Common Patterns

### Full Test Suite

```bash
# All tests in all modes
forge test                    # REVM
forge test --polkadot        # Polkadot EVM
forge test --polkadot=pvm    # Polkadot PVM (if applicable)

# Rust integration tests
cargo test --workspace
cargo test --package forge --test it test_revive_
```

### Quick Validation

```bash
# Fast smoke test
forge test --match-test "test_basic_*"

# Polkadot validation
forge test --polkadot --match-path "testdata/default/revive/**"
```

### Pre-merge Checklist

```bash
# 1. All tests pass
cargo test --workspace
forge test --polkadot

# 2. Formatting
cargo fmt --all

# 3. Linting
cargo clippy --workspace

# 4. Integration tests
cargo test --package forge --test it test_revive_
```

## Troubleshooting

### Tests Pass in REVM but Fail in Polkadot

**Likely causes**:
1. Numeric overflow (block number, timestamp, balance > limits)
2. Gas estimation differences
3. State sync issues

**Debugging**:
```bash
# Compare outputs
forge test --match-test testFailing -vvv > revm.log
forge test --polkadot --match-test testFailing -vvv > polkadot.log
diff revm.log polkadot.log

# Check for warnings
forge test --polkadot -vv | grep -i "warning\|clamp"
```

### PVM Tests Fail but EVM Tests Pass

**Likely causes**:
1. PVM bytecode not compiled
2. Library incompatibility
3. Proxy pattern issues

**Debugging**:
```bash
# Verify PVM compilation
forge build --resolc
forge inspect MyContract bytecode --resolc

# Check for 0x505 prefix (PVM bytecode marker)
forge inspect MyContract bytecode --resolc | grep "^0x505"
```

### Slow Test Execution

**Solutions**:
1. Use `--match-test` to run specific tests
2. Run REVM tests during development
3. Run Polkadot tests in CI/CD
4. Parallelize Rust tests: `cargo test -- --test-threads=4`

### Flaky Tests

**Likely causes**:
1. Non-deterministic behavior (timestamps, block numbers)
2. Race conditions in async code
3. Insufficient gas

**Solutions**:
```solidity
// Fix timestamps
vm.warp(1234567890);  // Set deterministic timestamp

// Fix block numbers
vm.roll(100);  // Set deterministic block number

// Increase gas
vm.txGasPrice(1 gwei);
```

## Examples

### Basic Test Run

```bash
# Run all tests
forge test --polkadot

# Output:
# [⠢] Compiling...
# [⠆] Compiling 1 files with 0.8.18
# [⠰] Solc 0.8.18 finished in 2.34s
# Compiler run successful!
#
# Running 5 tests for testdata/default/revive/Counter.t.sol:CounterTest
# [PASS] testIncrement() (gas: 12345)
# [PASS] testDecrement() (gas: 12345)
# [PASS] testSetNumber() (gas: 12345)
# [PASS] testReset() (gas: 12345)
# [PASS] testMultiple() (gas: 12345)
# Test result: ok. 5 passed; 0 failed; finished in 1.23s
```

### Verbose Test Run

```bash
# Run with maximum verbosity
forge test --polkadot --match-test testIncrement -vvvvv

# Output shows:
# - Compilation details
# - Setup execution
# - Transaction traces
# - State changes
# - Gas usage
# - Assertion results
```

### Rust Integration Test

```bash
# Run with logging
env RUST_LOG=warn,revive_strategy=info \
  cargo test --package forge --test it test_revive_basic_migration -- --nocapture

# Output:
# running 2 tests
# test revive::cheat_migrations::test_revive_basic_migration::evm ... ok
# test revive::cheat_migrations::test_revive_basic_migration::pvm ... ok
#
# test result: ok. 2 passed; 0 failed; finished in 12.34s
```

## References

- [Writing Tests Guide](writing-tests.md) - How to write tests with vm.polkadot
- [Integration Tests Guide](integration-tests.md) - Adding Solidity + Rust test pairs
- [State Migration Guide](state-migration.md) - Understanding state synchronization
- [Troubleshooting Guide](troubleshooting.md) - Common issues and solutions

## Related CLI Commands

```bash
# Build contracts
forge build                    # EVM bytecode
forge build --resolc          # PVM bytecode

# Inspect bytecode
forge inspect Counter bytecode
forge inspect Counter bytecode --resolc

# Clean build artifacts
forge clean
cargo clean

# Format code
forge fmt                     # Solidity
cargo fmt --all              # Rust
```
