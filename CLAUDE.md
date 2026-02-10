# Foundry-Polkadot Context

This document provides essential architectural context for the foundry-polkadot integration.

**For testing workflows**, use the **`foundry-polkadot-testing` skill**: `.claude/skills/foundry-polkadot-testing/`

## Project Overview

Foundry-Polkadot is an integration built on **pallet-revive** that allows Solidity developers to test contracts against **Polkadot's EVM** and **PolkaVM (RISC-V)** backends directly from `forge test`.

## 🚨 Critical Instructions

1. **Compilation**: Use `--resolc` flag for Polkadot/PVM compilation. Without it, standard Solc/EVM is used.
2. **Testing**: The `forge test` command supports `--polkadot` flag for Polkadot runtime testing. Use standard `forge-std/Test.sol` for your tests.
3. **DSTest vs Test**: Use `forge-std/Test.sol` for normal development. DSTest is only for foundry-polkadot's internal integration tests.
4. **For detailed testing workflows**: Use the `foundry-polkadot-testing` skill.

## 🛡️ Safety Boundaries

- ✅ **Safe**: `cargo build`, `cargo test`, `cargo fmt`, `cargo clippy`
- ✅ **Safe**: `forge build`, `forge test`, `forge init`, `forge fmt`
- ⚠️ **Ask First**: Modifying existing migrations, changing gas models, altering state sync logic
- ❌ **Never**: Commit changes to test fixtures without running full test suite
- ❌ **Never**: Modify pallet-revive integration without understanding state synchronization

## 🗺️ Directory Map

| Path | Purpose | Key Commands |
|------|---------|--------------|
| `crates/forge/` | Forge CLI and test runner | `cargo test --package forge` |
| `crates/evm/evm/` | EVM execution and Polkadot integration | `cargo build --package foundry-evm` |
| `crates/evm/fuzz/` | Fuzzing strategies and proptest | `cargo test --package foundry-evm-fuzz` |
| `crates/config/` | Configuration (FuzzConfig, InvariantConfig) | `cargo build --package foundry-config` |
| `crates/revive-strategy/` | Polkadot runtime integration strategy | `cargo build --package revive-strategy` |
| `crates/revive-utils/` | Utilities for Revive integration | `cargo build --package revive-utils` |
| `testdata/default/revive/` | Polkadot integration test contracts | Run via `forge test` integration tests |
| `.claude/skills/foundry-polkadot-testing/` | Testing workflows skill | Use for testing guidance |

## 🏛️ Core Concepts

### 1. Revive Architecture

Revive supports two distinct execution backends:

| Bytecode | VM | Status | Description |
|----------|-----|--------|-------------|
| **EVM** | EVM Interpreter | Production | Fully compatible with Ethereum |
| **PVM** | PolkaVM (RISC-V) | Experimental | Limited Ethereum compatibility |

### 2. Dual Compilation Model

Contracts are compiled to **both** EVM and PVM bytecode simultaneously:
- Standard Solc compilation produces EVM bytecode
- Resolc compilation (via `--resolc` flag) produces PVM bytecode (starts with `0x505`)

### 3. Integration Architecture

**Tests always start in Foundry REVM**, then switch to pallet-revive based on configuration:

- **Test contracts** run inside Foundry's REVM (never migrated)
- **CREATE/CREATE2** opcodes are intercepted → contract deployed in pallet-revive runtime
- **CALL/STATICCALL/DELEGATECALL** opcodes are intercepted → call executed in pallet-revive runtime
- **Cheatcode precompile** (HEVM_ADDRESS) only exists in REVM - deployed contracts in pallet-revive cannot call cheatcodes

### 4. State Synchronization (Bidirectional)

The integration uses **bidirectional state synchronization** between Foundry REVM and pallet-revive:

**Initial Migration (REVM → pallet-revive)**:
- When `--polkadot` flag is used, `select_revive()` is called at test startup
- Migrates all persistent accounts: balances, nonces, bytecode, storage
- Bytecode selection based on runtime mode (EVM or PVM)
- Test contract itself remains in REVM

**After Each Call (pallet-revive → REVM)**:
- After every contract call/create in pallet-revive, state syncs BACK to REVM
- `apply_revm_storage_diff()` syncs only changed storage slots
- Ensures REVM always has the latest state from pallet-revive

**Switching Back (pallet-revive → REVM)**:
- When `vm.polkadot(false)` is called, `select_evm()` migrates everything back
- Reads balances, nonces from pallet-revive
- Writes back to REVM journaled state
- Contracts marked with `vm.makePersistent` survive the migration

**For detailed state migration information**, see: `.claude/skills/foundry-polkadot-testing/references/state-migration.md`

## 🚀 Quick Command Reference

### Common Commands

```bash
# Standard REVM testing
forge test

# Polkadot EVM testing (PRODUCTION)
forge test --polkadot

# Polkadot PVM testing (EXPERIMENTAL)
forge test --polkadot=pvm

# Build with Resolc (PVM bytecode)
forge build --resolc

# Inspect PVM bytecode (starts with 0x505)
forge inspect Counter bytecode --resolc
```

### Cast Commands

```bash
# Check balance on Polkadot testnet
cast balance 0x... --rpc-url https://testnet-passet-hub-eth-rpc.polkadot.io

# Call contract
cast call 0x... "get()" --rpc-url https://testnet-passet-hub-eth-rpc.polkadot.io

# Send transaction
cast send 0x... "set(uint256)" 42 --rpc-url https://testnet-passet-hub-eth-rpc.polkadot.io --private-key 0x...
```

**For detailed CLI usage**, see: `.claude/skills/foundry-polkadot-testing/references/running-tests.md`

## ⚠️ Known Limitations

1. **Gas Model**: Not fully aligned with Polkadot's production gas model
2. **Numeric Types**: Ethereum uses `u256`, Polkadot uses `u64` for block numbers/timestamps and `u128` for balances - values exceeding these limits will be clamped with warnings
3. **PVM Integration**: Experimental - may not work with libraries or proxy patterns
4. **Unsupported Commands**: `forge clone`, `forge coverage`, `forge snapshot` don't work
5. **Cheatcodes**: HEVM_ADDRESS only exists in REVM - contracts in pallet-revive cannot call cheatcodes. Use `vm.polkadotSkip()` to keep contracts in REVM

**For troubleshooting**, see: `.claude/skills/foundry-polkadot-testing/references/troubleshooting.md`

## 🔧 Key Cheatcodes

### vm.polkadot

```solidity
interface Vm {
    /// Switch INTO or OUT OF Polkadot runtime
    function polkadot(bool enable) external;
    function polkadot(bool enable, string memory backend) external;

    /// Skip Polkadot execution - run only in REVM
    function polkadotSkip() external;
}
```

**Usage**:
```solidity
// Enable Polkadot runtime
vm.polkadot(true);

// Disable Polkadot runtime (switch back to REVM)
vm.polkadot(false);

// Keep contracts in REVM where HEVM_ADDRESS exists
vm.polkadotSkip();
```

### vm.makePersistent

```solidity
// Required for contracts to survive VM switches
Counter counter = new Counter();
vm.makePersistent(address(counter));
vm.polkadot(false);  // counter migrates to REVM
```

**For detailed cheatcode usage**, see: `.claude/skills/foundry-polkadot-testing/references/writing-tests.md`

## 🎯 Execution Matrix

| CLI Flag | Default Environment | `vm.polkadot(false)` | `vm.polkadot(true, "evm")` | `vm.polkadot(true, "pvm")` |
|----------|---------------------|----------------------|----------------------------|----------------------------|
| `forge test` | Foundry REVM | Foundry REVM | ❌ Invalid | ❌ Invalid |
| `forge test --polkadot` | Polkadot EVM | Foundry REVM | Polkadot EVM | Polkadot PVM |
| `forge test --polkadot=pvm` | Polkadot PVM | Foundry REVM | Polkadot EVM | Polkadot PVM |

## ✅ Verification Matrix

| Check | Command | Notes |
|-------|---------|-------|
| Build | `cargo build --workspace` | Builds all packages |
| Test | `cargo test --workspace` | Runs all Rust tests |
| Format | `cargo fmt --all` | Formats Rust code |
| Lint | `cargo clippy --workspace` | Lints with clippy |
| Forge Tests | `cargo test --package forge --test it test_revive_` | Runs Polkadot integration tests |
| Specific Test | `env RUST_LOG=warn cargo test --package forge --test it test_revive_basic_migration -- --nocapture` | Run with logging |

## 📋 Supported Forge Commands

### ✅ Working Commands
- `init`, `build`, `test`
- `bind`, `bind-json`
- `cache clean`, `cache ls`, `clean`
- `compiler resolve`, `config`
- `create` (contract deployment)
- `doc`, `flatten`, `fmt`
- `geiger`, `generate test`, `generate-fig-spec`
- `inspect`
- `install`, `update`, `remove`, `remappings`
- `selectors upload`, `selectors list`, `selectors find`, `selectors cache`
- `tree`

### ❌ Not Working Commands
- `clone`, `coverage`, `snapshot`

## 📚 Key Files for Understanding

### Documentation

- [README.forge.test.md](README.forge.test.md) - Comprehensive testing integration guide
- [README.forge.md](README.forge.md) - Supported forge commands with examples
- [README.cast.md](README.cast.md) - Supported cast commands with examples
- [.claude/skills/foundry-polkadot-testing/](/.claude/skills/foundry-polkadot-testing/) - **Testing workflows skill**

### Critical Implementation Files

**State Synchronization & Migration**:

[crates/revive-strategy/src/cheatcodes/mod.rs](crates/revive-strategy/src/cheatcodes/mod.rs) - Core integration logic
- `select_revive()` (line 649) - Migrates REVM → pallet-revive at test startup
- `select_evm()` (line 903) - Migrates pallet-revive → REVM when switching back
- `apply_revm_storage_diff()` (line 1560) - Syncs storage after each call
- `migrate_contract_storage()` (line 853) - Storage migration helper
- `revive_try_create()` (line 985) - Intercepts CREATE opcodes
- `revive_try_call()` (line 1212) - Intercepts CALL opcodes

**Test Infrastructure**:
- [crates/forge/tests/it/revive/](crates/forge/tests/it/revive/) - Rust test wrappers
- [testdata/default/revive/](testdata/default/revive/) - Solidity test contracts

## 🔑 Configuration

### foundry.toml

```toml
[profile.default]
src = "src"
out = "out"
libs = ["lib"]

# Enable Resolc compilation
[profile.default.polkadot]
resolc_compile = true

# Fuzz configuration
[fuzz]
runs = 256
max_test_rejects = 65536
# Optional: limit fuzz integers to simulate smaller types (e.g., u128 for Polkadot balances)
# Unsigned: [0, max_fuzz_int], Signed: [-(max_fuzz_int+1), max_fuzz_int]
# max_fuzz_int = "340282366920938463463374607431768211455"  # u128::MAX

# Invariant configuration
[invariant]
runs = 256
depth = 500
```

## 🎓 Recommendations

- **Production**: Use **EVM mode** (`--polkadot=evm`) - stable, deterministic, Ethereum-compatible
- **Research**: Use **PVM mode** (`--polkadot=pvm`) - experimental RISC-V capabilities
- **Fuzzing**: Set `max_fuzz_int` in config to limit integers if needed for Polkadot compatibility
- **Migration**: Always use `vm.makePersistent` for contracts that need to survive VM switches

## 📖 State That Migrates

When using `vm.makePersistent`, the following state migrates between runtimes:
- Account **balances**
- Account **nonces**
- Contract **bytecode**
- Contract **storage** (including immutables)
- Block **timestamp** and other environment variables

---

## 🧪 Testing Workflows

**For comprehensive testing guidance**, use the **`foundry-polkadot-testing` skill**:

- **Running tests**: `.claude/skills/foundry-polkadot-testing/references/running-tests.md`
- **Writing tests**: `.claude/skills/foundry-polkadot-testing/references/writing-tests.md`
- **Adding integration tests**: `.claude/skills/foundry-polkadot-testing/references/integration-tests.md`
- **State migration**: `.claude/skills/foundry-polkadot-testing/references/state-migration.md`
- **Troubleshooting**: `.claude/skills/foundry-polkadot-testing/references/troubleshooting.md`

Or invoke the skill with phrases like:
- "How do I run tests with forge test --polkadot?"
- "How do I write a test that uses vm.polkadot?"
- "How do I add a new integration test?"
- "Why is my test failing in Polkadot?"
