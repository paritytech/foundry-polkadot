# Foundry–Polkadot Integration: Unified Testing for EVM & PVM

## Introduction

This document introduces the **Foundry–Polkadot integration**, built on top of **pallet-revive**. This integration allows Solidity developers to test contracts against **Polkadot’s EVM** and the **PolkaVM (RISC-V)** backends directly from `forge test`.

It covers:
- The **Revive dual-execution architecture**
- Foundry’s **dual-bytecode compilation model**
- The **CLI interface** for execution mode selection
- The **`vm.polkadot`** cheatcode for dynamic runtime switching
- Guidelines for production and experimental testing

> **Note on Terminology**:
**"Polkadot runtime"** refers to the execution environment activated by the `--polkadot` flag. Internally, this environment is powered by **pallet-revive**, which supports two **backends**: EVM and PolkaVM.

---

## 1. Background: Revive Architecture

Revive is a Substrate pallet designed to support **two distinct execution backends**:

| Bytecode | VM  | Description |
|----------|-----|-------------|
| **EVM** | EVM Interpreter | Fully compatible with Ethereum. |
| **PVM** | PolkaVM (RISC-V) | Limited Ethereum compatibility. |

Developers can compile a single Solidity source into **both** bytecode formats. The type of bytecode deployed determines the backend used for its execution.

---

## 2. The Dual Compilation Model

To support multiple backends, a **dual compilation model** is implemented within Foundry.

When tests are run in this environment, contracts are compiled to both EVM and PVM bytecode simultaneously. This enables the test runner to switch execution environments between the standard Foundry EVM and the Polkadot runtime during test execution.

---

## 3. Integration Architecture

Foundry executes tests inside a local **REVM** (EVM), where cheatcodes like `vm.store` and `vm.prank` manipulate the internal database.

In **foundry-polkadot**, the test contract (implementing `Test` or `DSTest`) continues to run inside Foundry's REVM. However, specific operations are **intercepted** to support Polkadot execution:

| Opcode | Action | Behavior |
|--------|--------|----------|
| `CREATE` / `CREATE2` | **Intercept** | The contract is deployed inside the **Polkadot runtime** (pallet-revive) instead of the Foundry REVM. |
| `CALL` / `STATICCALL` / `DELEGATECALL` | **Intercept** | The call is executed inside the **Polkadot runtime** (pallet-revive), skipping the Foundry REVM execution. |

### Architecture Rationale
1. **Compatibility**: Test logic and assertions run in the standard Foundry environment.
2. **Execution Environment**: Contract execution occurs inside the Polkadot runtime logic (pallet-revive).
3. **Cheatcodes**: Standard Foundry cheatcodes remain functional for setup and assertions.

---

## 4. State Synchronization & Storage

Hybrid testing requires state management between environments. This integration uses a model where the Foundry REVM and the Polkadot runtime synchronize state via access logs.

### Diff-Based State Bridging
Instead of migrating the entire database, a **diff-based state bridging** mechanism is used:

1. **Polkadot Runtime Execution**: Execution in Polkadot mode (pallet-revive) produces storage diffs.
2. **Sync to REVM**: These diffs are applied to the Foundry REVM.
3. **Sync to Polkadot**: Setup performed in REVM (via cheatcodes) is applied directly to the Polkadot runtime state. 

This ensures that cheatcodes like `vm.store` function correctly and state changes in the contract are reflected in test assertions.

---

## 5. Unified CLI Interface

The CLI controls the runtime using the `--polkadot` flag, following standard Foundry conventions.

```bash
# Standard Foundry behavior (Local Foundry REVM)
forge test

# Polkadot EVM runtime (Default for Polkadot mode)
forge test --polkadot

# Explicitly select Polkadot EVM
forge test --polkadot=evm

# Experimental: Polkadot PVM runtime
forge test --polkadot=pvm
```

**Design Principles:**
- **No Redundancy**: Compiler flags are handled automatically.
- **Consistency**: Usage mirrors standard Foundry commands.
- **Configuration**: Compiler settings remain in `foundry.toml`, while runtime selection is handled via CLI.

---

## 6. Dynamic Switching: The `vm.polkadot` Cheatcode

The CLI sets the default mode for the test run. The `vm.polkadot` cheatcode allows toggling environments dynamically within a specific test.

```solidity
interface Vm {
    /// Switch INTO or OUT OF Polkadot runtime.
    /// backend: "evm", or "pvm"
    function polkadot(bool enable, string memory backend) external;

    /// Auto-detect backend from CLI flags.
    function polkadot(bool enable) external;
}
```

### Usage Examples

**Switch to Polkadot runtime (default backend):**
```solidity
vm.polkadot(true);
// Subsequent calls run in Polkadot runtime
```

**Return to standard REVM:**
```solidity
vm.polkadot(false);
// Subsequent calls run in local Foundry REVM
```

> **Note**: When switching VMs, contracts must be registered for migration using `vm.makePersistent` if they need to persist across execution boundaries.

---

## 7. Execution Matrix

The following tables describe how CLI flags interact with cheatcodes in different scenarios.

### Scenario 1: Standard Foundry REVM
**Command:** `forge test`
**Context:** The test runs entirely in the local Foundry EVM. Polkadot features are disabled.

| Cheatcode Action | Resulting Environment |
| :--- | :--- |
| None | **Foundry REVM** |
| `vm.polkadot(true)` | ❌ *Invalid (Mode not enabled)* |

### Scenario 2: Polkadot EVM
**Command:** `forge test --polkadot=evm` or `forge test --polkadot`
**Context:** The test runs in Polkadot mode, deploying **EVM bytecode** by default. You can switch back to local Foundry REVM or to PVM bytecode deployment.

| Cheatcode Action | Resulting Environment |
| :--- | :--- |
| None | **Polkadot EVM** |
| `vm.polkadot(false)` | **Foundry REVM** |
| `vm.polkadot(true, "pvm")` | **Polkadot PVM** |

### Scenario 3: Polkadot PVM
**Command:** `forge test --polkadot=pvm`
**Context:** The test runs in Polkadot mode, deploying **PVM bytecode** by default. You can switch back to local REVM or to EVM bytecode deployment.

| Cheatcode Action | Resulting Environment |
| :--- | :--- |
| None | **Polkadot PVM** |
| `vm.polkadot(false)` | **Foundry REVM** |
| `vm.polkadot(true, "evm")` | **Polkadot EVM** |

---

## 8. Example: Writing a Polkadot Test

Below is an example of a test that bridges the environments.

```solidity
// Simple contract to be tested
contract Simple {
    function get() public pure returns (uint256) {
        return 6;
    }
}

contract FooTest is Test {
    function testSimple() public {
        // 1. Deploy: Intercepted -> Deployed to Polkadot runtime
        Simple testContract = new Simple(); 
        
        // 2. Call: Intercepted -> Executed in Polkadot runtime
        uint256 number = testContract.get();
        
        // 3. Assert: Executed in local REVM
        assertEq(6, number);
    }
}
```

**Execution Flow with `forge test --polkadot` (Default: EVM Backend):**
1. The test starts in Foundry's EVM.
2. `new Simple()` is identified as a `CREATE` opcode. The system intercepts it and deploys the contract into the **Polkadot runtime** (using EVM bytecode).
3. `testContract.get()` is identified as a `CALL`. It is executed in the **Polkadot runtime**.
4. The return value `6` is passed back to the test runner.
5. `assertEq` runs in Foundry REVM to verify the result.

---

## 9. Recommendations & Limitations

### Recommendations
- **Production**: Use **EVM mode** (`--polkadot=evm`). It is stable, deterministic, and compatible with Ethereum.
- **Research**: Use **PVM mode** (`--polkadot=pvm`) to explore RISC-V capabilities. This mode is experimental.

### Known Limitations
Tests on standard open-source projects have shown a **90-100% pass rate** using the Polkadot EVM backend. However, the following limitations exist:

1. **Gas Model**: The gas metering in `foundry-polkadot` is not fully aligned with Polkadot's production gas model. Tests relying on precise gas checks may fail.
2. **Balance Types**: Ethereum uses `u256` for balances, while Polkadot uses `u128`. Tests involving amounts exceeding `u128::MAX` will fail in the Polkadot runtime.
3. **PVM Integration Maturity**: The PVM backend is experimental. Tests may not work, for example when using libraries or proxy patterns.
