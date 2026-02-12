---
name: foundry-polkadot-testing
description: "Test Solidity contracts with forge test --polkadot against pallet-revive runtime using EVM or PVM backends"
license: MIT
compatibility: opencode
triggers:
  - "test polkadot"
  - "forge test --polkadot"
  - "foundry polkadot testing"
  - "pallet-revive testing"
  - "vm.polkadot"
  - "revive integration test"
metadata:
  project_specific: true
  domains:
    - foundry
    - polkadot
    - testing
    - forge
    - pallet-revive
    - integration-testing
---

# Foundry Polkadot Testing Skill

Test Solidity contracts against Polkadot's pallet-revive runtime using `forge test` with `--polkadot` flag.

<nav>
  <tag name="quick-start" section="Quick Start" />
  <tag name="core-concepts" section="Core Concepts" />
  <tag name="workflows" section="Common Workflows" />
  <tag name="constraints" section="Key Constraints" />
  <tag name="validation" section="Validation" />
</nav>

## Quick Decision Matrix

<decision_matrix>
<question>What do you need to do?</question>

<option condition="Run tests against Polkadot runtime">
  <navigate_to>references/running-tests.md</navigate_to>
  <description>Execute tests with --polkadot flag (EVM/PVM modes), understand test execution flow</description>
</option>

<option condition="Write tests that switch between REVM and Polkadot">
  <navigate_to>references/writing-tests.md</navigate_to>
  <description>Use vm.polkadot cheatcode, handle state migration, contract persistence</description>
</option>

<option condition="Add new integration test (Solidity + Rust wrapper)">
  <navigate_to>references/integration-tests.md</navigate_to>
  <description>Create Solidity test contracts, Rust test wrappers, register in test module</description>
</option>

<option condition="Debug test failures or understand architecture">
  <navigate_to>references/troubleshooting.md</navigate_to>
  <description>Common issues: numeric limits, state sync, bytecode selection, PVM compatibility</description>
</option>

<option condition="Understand state synchronization and migration">
  <navigate_to>references/state-migration.md</navigate_to>
  <description>Bidirectional sync (REVM ↔ pallet-revive), migration workflow, persistence</description>
</option>
</decision_matrix>

## Quick Start

**CRITICAL**: ALWAYS use `--polkadot` flag to test against pallet-revive runtime.

```bash
# Run tests in standard REVM (no Polkadot integration)
forge test

# Run tests against Polkadot EVM runtime (PRODUCTION READY) - MUST use this for production
forge test --polkadot

# Run tests against Polkadot PVM runtime (EXPERIMENTAL) - NEVER use in production
forge test --polkadot=pvm

# Run specific test with verbose output - MUST use -vvv for debugging
forge test --polkadot --match-test testMyFunction -vvv

# Run with integer fuzzing limited (set max_fuzz_int in foundry.toml if needed)
forge test --polkadot
```

**SAFETY RULES**:
- ✅ **ALWAYS** test with `--polkadot` before deploying to Polkadot
- ✅ **MUST** use `vm.makePersistent` before switching VMs with `vm.polkadot(false)`
- ❌ **NEVER** use `--polkadot=pvm` for production testing
- ❌ **NEVER** exceed u64::MAX for timestamps/block numbers or u128::MAX for balances
- ❌ **MUST NOT** forget to compile with `--resolc` when using PVM mode

## Core Concepts

**CRITICAL CONSTRAINTS**:
- MUST use DSTest base contract (NOT forge-std/Test)
- MUST call `vm.makePersistent(address)` before using `vm.polkadot(false)`
- MUST NOT use proxy patterns or complex libraries with PVM mode
- MUST compile with `--resolc` before testing PVM mode
- NEVER commit test changes without running full test suite

### Dual Execution Model

| Mode | Flag | Backend | Status | Use Case |
|------|------|---------|--------|----------|
| **REVM** | (none) | Foundry's standard EVM | Stable | Local development, standard Ethereum |
| **Polkadot EVM** | `--polkadot` or `--polkadot=evm` | pallet-revive EVM | Production | Ethereum-compatible Polkadot testing |
| **Polkadot PVM** | `--polkadot=pvm` | PolkaVM (RISC-V) | Experimental | Advanced RISC-V features |

### Test Execution Flow

```
┌─────────────────────────────────────────────────────────────┐
│ Test Contract (runs in REVM)                                │
│  ├─ setUp()                                                 │
│  ├─ test functions                                          │
│  └─ vm.polkadot(true) ← switches to pallet-revive          │
└─────────────────────────────────────────────────────────────┘
                              │
                              ↓
┌─────────────────────────────────────────────────────────────┐
│ CREATE/CALL Interception                                    │
│  ├─ CREATE/CREATE2 → deployed in pallet-revive             │
│  ├─ CALL/STATICCALL → executed in pallet-revive            │
│  └─ State syncs back to REVM after each operation          │
└─────────────────────────────────────────────────────────────┘
```

**MANDATORY EXECUTION RULES**:
- Test contracts MUST ALWAYS run in REVM (NEVER migrate test contracts)
- User contracts deployed with `new` MUST go to pallet-revive when `--polkadot` is active
- State MUST sync bidirectionally after each operation (automatic)
- Cheatcodes MUST remain functional in both modes
- NEVER intercept test contract calls

### State Synchronization

**CRITICAL THREE-PHASE SYNC** (MUST happen in this order):

1. **Initial Migration** (REVM → pallet-revive): MUST trigger when `--polkadot` flag is used
   - MUST migrate persistent accounts: balances, nonces, bytecode, storage
   - Test contract MUST stay in REVM (NEVER migrate)

2. **After Each Call** (pallet-revive → REVM): MUST happen after EVERY contract interaction
   - MUST sync changed storage slots only (NEVER full state)
   - MUST ensure REVM has latest state before assertions

3. **Switching Back** (pallet-revive → REVM): MUST trigger when `vm.polkadot(false)` is called
   - MUST migrate everything back to REVM
   - CRITICAL: Persistent contracts MUST survive migration
   - NEVER lose non-persistent contracts (they MUST be garbage collected)

## Common Workflows

### Run Existing Tests

```bash
# All tests against Polkadot EVM
forge test --polkadot

# Specific test pattern
forge test --polkadot --match-test test_revive

# With debug output
env RUST_LOG=warn,revive_strategy=info \
  cargo test --package forge --test it test_revive_basic_migration -- --nocapture
```

### Write New Test

```solidity
// testdata/default/revive/MyTest.t.sol
pragma solidity ^0.8.18;

import "ds-test/test.sol";
import "cheats/Vm.sol";

contract MyTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function setUp() public {
        // Setup runs in REVM
    }

    function testWithPolkadot() public {
        // Deploys in pallet-revive (when --polkadot flag active)
        Counter counter = new Counter();

        // Mark for migration if switching VMs
        vm.makePersistent(address(counter));

        counter.increment();
        assertEq(counter.number(), 1);
    }
}
```

### Add Integration Test

1. Create Solidity test: `testdata/default/revive/MyTest.t.sol`
2. Create Rust wrapper: `crates/forge/tests/it/revive/my_test.rs`
3. Register module: `crates/forge/tests/it/revive/mod.rs`

See: [Integration Tests Guide](references/integration-tests.md)

## Key Constraints

**STOP**: Read these constraints before writing tests.

### Numeric Limits

**CRITICAL**: Polkadot uses smaller integer types than Ethereum.

| Type | Ethereum | Polkadot | Behavior |
|------|----------|----------|----------|
| Block number | `uint256` | `u64` | Values > u64::MAX clamped with warning |
| Timestamp | `uint256` | `u64` | Values > u64::MAX clamped with warning |
| Balance | `uint256` | `u128` | Values > u128::MAX clamped with warning |

**MANDATORY RULES**:
- MUST use `uint64` for timestamps/block numbers in fuzz tests
- MUST use `uint128` for balances in fuzz tests
- NEVER exceed u64::MAX (18446744073709551615) for time values
- NEVER exceed u128::MAX for balance values
- ALWAYS check for overflow warnings with `-vv` flag

**Fuzzing**: Set `max_fuzz_int` in foundry.toml to limit fuzz values for Polkadot compatibility

### PVM Limitations

- Experimental feature
- May not work with libraries or proxy patterns
- Requires `--resolc` compilation for PVM bytecode

### Unsupported Features

- `forge clone` requires an Etherscan API key
- `forge coverage --resolc` fails with compiler error (standard and `--polkadot` work)
- Gas model not fully aligned with production Polkadot

## Architecture Overview

### Critical Implementation Files

**State Synchronization**:
- [crates/revive-strategy/src/cheatcodes/mod.rs](../../crates/revive-strategy/src/cheatcodes/mod.rs)
  - `select_revive()` (line 649): REVM → pallet-revive migration
  - `select_evm()` (line 903): pallet-revive → REVM migration
  - `apply_revm_storage_diff()` (line 1560): Post-call sync
  - `revive_try_create()` (line 985): CREATE interception
  - `revive_try_call()` (line 1212): CALL interception

**Test Infrastructure**:
- [crates/forge/tests/it/revive/](../../crates/forge/tests/it/revive/): Rust test wrappers
- [testdata/default/revive/](../../testdata/default/revive/): Solidity test contracts

## Validation

### Before Committing

```bash
# Run full test suite
cargo test --workspace

# Run Polkadot integration tests
cargo test --package forge --test it test_revive_

# Run specific integration test
cargo test --package forge --test it test_revive_basic_migration

# Format and lint
cargo fmt --all
cargo clippy --workspace
```

### Test Naming Convention

- Rust files: `test_revive_*.rs`
- Solidity files: `*.t.sol`
- Test functions: `testCamelCase()` or `test_snake_case()`

## References

- [Running Tests: Comprehensive Guide](references/running-tests.md) - CLI flags, execution modes, debugging
- [Writing Tests: Comprehensive Guide](references/writing-tests.md) - vm.polkadot cheatcode, patterns, examples
- [Integration Tests: Comprehensive Guide](references/integration-tests.md) - Adding new test pairs (Solidity + Rust)
- [State Migration: Comprehensive Guide](references/state-migration.md) - Bidirectional sync, persistence, architecture
- [Troubleshooting: Comprehensive Guide](references/troubleshooting.md) - Common issues, debugging tips, solutions

## Examples

See existing tests:
- [testdata/default/revive/EvmToReviveMigration.t.sol](../../testdata/default/revive/EvmToReviveMigration.t.sol) - Migration examples
- [testdata/default/revive/MockCall.t.sol](../../testdata/default/revive/MockCall.t.sol) - Mock call patterns
- [testdata/default/revive/Store.t.sol](../../testdata/default/revive/Store.t.sol) - Storage manipulation
- [crates/forge/tests/it/revive/cheat_mock_calls.rs](../../crates/forge/tests/it/revive/cheat_mock_calls.rs) - Rust wrapper pattern
