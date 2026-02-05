# State Migration: Comprehensive Guide

<nav>
  <tag name="overview" section="Overview" />
  <tag name="architecture" section="Architecture" />
  <tag name="initial-migration" section="Initial Migration" />
  <tag name="continuous-sync" section="Continuous Sync" />
  <tag name="switching-back" section="Switching Back" />
  <tag name="implementation" section="Implementation Details" />
</nav>

<atom>
  <type>reference</type>
  <diataxis_quadrant>explanation</diataxis_quadrant>
  <intent>understand_state_migration</intent>
</atom>

## Overview

State migration is the bidirectional synchronization of contract state between Foundry's REVM and Polkadot's pallet-revive runtime.

## Three-Phase Synchronization

```
┌─────────────────────────────────────────────────────────────┐
│ Phase 1: Initial Migration (REVM → pallet-revive)           │
│  ├─ Triggered by: --polkadot flag at test startup          │
│  ├─ Migrates: persistent accounts, balances, code, storage │
│  └─ Function: select_revive() (line 649)                   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ↓
┌─────────────────────────────────────────────────────────────┐
│ Phase 2: Continuous Sync (pallet-revive → REVM)             │
│  ├─ Triggered by: After each contract call/create          │
│  ├─ Syncs: Changed storage slots only                      │
│  └─ Function: apply_revm_storage_diff() (line 1560)        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ↓
┌─────────────────────────────────────────────────────────────┐
│ Phase 3: Switching Back (pallet-revive → REVM)              │
│  ├─ Triggered by: vm.polkadot(false)                       │
│  ├─ Migrates: persistent contracts back to REVM            │
│  └─ Function: select_evm() (line 903)                      │
└─────────────────────────────────────────────────────────────┘
```

## Architecture

### Dual VM System

```
┌─────────────────────────────────────────────────────────────┐
│ Foundry REVM                                                 │
│  ├─ Test contracts (always here)                           │
│  ├─ Cheatcodes (vm.*)                                       │
│  ├─ Assertions (assertEq, assertTrue, etc.)                │
│  └─ DSTest helpers                                          │
└─────────────────────────────────────────────────────────────┘
                              │
                    Interception Layer
                              │
┌─────────────────────────────────────────────────────────────┐
│ pallet-revive Runtime                                        │
│  ├─ User contracts (when --polkadot active)                │
│  ├─ Contract calls (CALL, STATICCALL, DELEGATECALL)        │
│  ├─ Contract deploys (CREATE, CREATE2)                     │
│  └─ State changes                                           │
└─────────────────────────────────────────────────────────────┘
```

### What Runs Where

**REVM (Always)**:
- Test contract itself
- `setUp()` function
- Test functions (`test*()`)
- Cheatcode calls (`vm.*`)
- Assertion logic

**pallet-revive (When `--polkadot` active)**:
- Contracts deployed with `new`
- Function calls on deployed contracts
- State mutations

### Interception Points

**CREATE/CREATE2 Interception**:
```rust
// crates/revive-strategy/src/cheatcodes/mod.rs:985
pub fn revive_try_create(&mut self, inputs: &mut CreateInputs) -> Option<CreateOutcome> {
    // Intercepts contract deployment
    // Deploys in pallet-revive instead of REVM
}
```

**CALL/STATICCALL/DELEGATECALL Interception**:
```rust
// crates/revive-strategy/src/cheatcodes/mod.rs:1212
pub fn revive_try_call(&mut self, inputs: &mut CallInputs) -> Option<CallOutcome> {
    // Intercepts contract calls
    // Executes in pallet-revive instead of REVM
}
```

## Phase 1: Initial Migration (REVM → pallet-revive)

### When It Happens

**Trigger**: `--polkadot` flag at test startup

**Function**: `select_revive()` (line 649)

### What Gets Migrated

**For Each Persistent Account**:

1. **Balance**: Account ETH balance
2. **Nonce**: Transaction count
3. **Bytecode**: Contract code (EVM or PVM based on mode)
4. **Storage**: All storage slots

### Implementation

```rust
// crates/revive-strategy/src/cheatcodes/mod.rs:649
pub fn select_revive(&mut self, revive_backend: &str) -> Result {
    // 1. Get persistent accounts from REVM
    let persistent_accounts = self.get_persistent_accounts();

    // 2. For each account, migrate:
    for account in persistent_accounts {
        // Balance
        let balance = self.revm.balance(account);

        // Nonce
        let nonce = self.revm.nonce(account);

        // Bytecode (EVM or PVM)
        let bytecode = if pvm_mode {
            self.get_pvm_bytecode(account)
        } else {
            self.get_evm_bytecode(account)
        };

        // Storage (all slots)
        let storage = self.migrate_contract_storage(account);

        // 3. Write to pallet-revive
        self.revive.set_account(account, balance, nonce, bytecode, storage);
    }
}
```

### Bytecode Selection

```rust
// Bytecode selection based on runtime mode
match runtime_mode {
    ReviveRuntimeMode::Evm => {
        // Use EVM bytecode (standard Solidity compilation)
        bytecode = account.info.code.as_ref().original_bytes()
    }
    ReviveRuntimeMode::Pvm => {
        // Use PVM bytecode (requires --resolc compilation)
        bytecode = self.get_pvm_bytecode_from_artifacts(address)
    }
}
```

**Important**: PVM bytecode starts with `0x505` prefix.

### Example

```solidity
contract MigrationTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testInitialMigration() public {
        // When --polkadot is active, this triggers initial migration
        Counter counter = new Counter();

        // Counter is now in pallet-revive with:
        // - Balance: 0
        // - Nonce: 1 (from creation)
        // - Bytecode: EVM or PVM based on mode
        // - Storage: initialized state
    }
}
```

## Phase 2: Continuous Sync (pallet-revive → REVM)

### When It Happens

**Trigger**: After every contract call or create in pallet-revive

**Function**: `apply_revm_storage_diff()` (line 1560)

### Why It's Needed

1. **Assertions in REVM**: Test assertions run in REVM, need latest state
2. **Cheatcodes**: Some cheatcodes inspect state, need accurate data
3. **Multi-contract**: Contracts may call each other, state must sync

### What Gets Synced

**Only changed storage slots**:
- Not full account state
- Only storage slots modified by the call
- Efficient incremental sync

### Implementation

```rust
// crates/revive-strategy/src/cheatcodes/mod.rs:1560
fn apply_revm_storage_diff(&mut self, address: Address, storage_diff: Vec<(U256, U256)>) {
    // For each changed storage slot:
    for (slot, new_value) in storage_diff {
        // Update REVM journaled state
        self.revm.journaled_state
            .state
            .get_mut(&address)
            .unwrap()
            .storage
            .insert(slot, new_value);
    }
}
```

### Example

```solidity
function testContinuousSync() public {
    Counter counter = new Counter();

    // 1. Call executed in pallet-revive
    counter.increment();  // storage[0] = 1

    // 2. State synced back to REVM (storage[0] = 1)

    // 3. Assertion reads from REVM (sees storage[0] = 1)
    assertEq(counter.number(), 1);  // ✅ Passes
}
```

### Performance Optimization

**Why only changed slots?**

```rust
// ❌ BAD: Full state sync (expensive)
fn sync_full_state() {
    for slot in all_storage_slots {
        sync_slot(slot);
    }
}

// ✅ GOOD: Incremental sync (efficient)
fn sync_changed_slots(diff: Vec<(U256, U256)>) {
    for (slot, value) in diff {
        sync_slot(slot, value);
    }
}
```

## Phase 3: Switching Back (pallet-revive → REVM)

### When It Happens

**Trigger**: `vm.polkadot(false)` cheatcode call

**Function**: `select_evm()` (line 903)

### What Gets Migrated

**For Each Persistent Contract**:

1. **Balance**: Current ETH balance
2. **Nonce**: Current transaction count
3. **Storage**: All storage slots (read from pallet-revive)

**Not Migrated**:
- Bytecode (already in REVM)
- Non-persistent contracts (lost)

### Implementation

```rust
// crates/revive-strategy/src/cheatcodes/mod.rs:903
pub fn select_evm(&mut self) -> Result {
    // 1. Get persistent contracts
    let persistent = self.get_persistent_accounts();

    // 2. For each persistent contract:
    for address in persistent {
        // Read from pallet-revive
        let balance = self.revive.balance(address);
        let nonce = self.revive.nonce(address);
        let storage = self.revive.get_all_storage(address);

        // Write to REVM
        self.revm.journaled_state.state.get_mut(&address).unwrap().info.balance = balance;
        self.revm.journaled_state.state.get_mut(&address).unwrap().info.nonce = nonce;

        for (slot, value) in storage {
            self.revm.journaled_state.state.get_mut(&address).unwrap().storage.insert(slot, value);
        }
    }

    // 3. Switch execution back to REVM
    self.active_runtime = RuntimeMode::Revm;
}
```

### Example

```solidity
function testSwitchingBack() public {
    Counter counter = new Counter();
    vm.makePersistent(address(counter));

    // In pallet-revive
    counter.increment();  // storage[0] = 1
    assertEq(counter.number(), 1);

    // Switch to REVM
    vm.polkadot(false);

    // Now in REVM, state migrated
    assertEq(counter.number(), 1);  // ✅ Still 1

    // Further operations in REVM
    counter.increment();  // Now storage[0] = 2 in REVM
    assertEq(counter.number(), 2);
}
```

## Persistence Deep Dive

### vm.makePersistent

**Purpose**: Mark contracts for migration when switching VMs

```solidity
function makePersistent(address account) external;
```

### Internal Tracking

```rust
// Persistent accounts stored in REVM
struct Cheatcodes {
    persistent_accounts: HashSet<Address>,
    // ...
}

impl Cheatcodes {
    fn make_persistent(&mut self, address: Address) {
        self.persistent_accounts.insert(address);
    }

    fn is_persistent(&self, address: Address) -> bool {
        self.persistent_accounts.contains(&address)
    }
}
```

### What Happens Without Persistence

```solidity
// ❌ Contract lost when switching
function testNoPersistence() public {
    Counter counter = new Counter();
    // Missing: vm.makePersistent(address(counter));

    counter.increment();

    vm.polkadot(false);  // counter is lost!

    // This will fail - counter doesn't exist in REVM
    assertEq(counter.number(), 1);  // ❌ Fails
}

// ✅ Contract migrates
function testWithPersistence() public {
    Counter counter = new Counter();
    vm.makePersistent(address(counter));  // ← Mark persistent

    counter.increment();

    vm.polkadot(false);  // counter migrates

    assertEq(counter.number(), 1);  // ✅ Passes
}
```

## Storage Migration Details

### Full Storage Scan

```rust
// crates/revive-strategy/src/cheatcodes/mod.rs:853
fn migrate_contract_storage(&mut self, address: Address) -> Vec<(U256, U256)> {
    let mut storage = Vec::new();

    // Get all storage slots from REVM
    let account = self.revm.journaled_state.state.get(&address).unwrap();

    for (slot, value) in &account.storage {
        if !value.is_zero() {  // Skip zero values (default)
            storage.push((*slot, *value));
        }
    }

    storage
}
```

### Storage Slot Format

**REVM Storage**:
```rust
HashMap<U256, U256>  // slot -> value
```

**pallet-revive Storage**:
```rust
BTreeMap<H256, H256>  // slot -> value
```

**Conversion**:
```rust
fn convert_storage(revm_storage: HashMap<U256, U256>) -> BTreeMap<H256, H256> {
    revm_storage
        .into_iter()
        .map(|(slot, value)| {
            let slot_h256 = H256::from(slot.to_be_bytes());
            let value_h256 = H256::from(value.to_be_bytes());
            (slot_h256, value_h256)
        })
        .collect()
}
```

## Implementation Details

### Key Files

**Main Implementation**:
- [crates/revive-strategy/src/cheatcodes/mod.rs](../../../crates/revive-strategy/src/cheatcodes/mod.rs)

**Critical Functions**:

| Function | Line | Purpose |
|----------|------|---------|
| `select_revive()` | 649 | Initial migration REVM → pallet-revive |
| `select_evm()` | 903 | Switch back pallet-revive → REVM |
| `migrate_contract_storage()` | 853 | Migrate storage slots |
| `apply_revm_storage_diff()` | 1560 | Sync storage after call |
| `revive_try_create()` | 985 | Intercept CREATE opcodes |
| `revive_try_call()` | 1212 | Intercept CALL opcodes |

### Data Structures

**Account Info**:
```rust
struct AccountInfo {
    balance: U256,
    nonce: u64,
    code_hash: B256,
    code: Option<Bytecode>,
}
```

**Storage Entry**:
```rust
struct StorageSlot {
    previous_or_original_value: U256,
    present_value: U256,
}
```

## Migration Patterns

### Pattern 1: Simple Migration

```solidity
function testSimpleMigration() public {
    Counter counter = new Counter();
    vm.makePersistent(address(counter));

    counter.increment();
    vm.polkadot(false);

    assertEq(counter.number(), 1);
}
```

### Pattern 2: Multiple Migrations

```solidity
function testMultipleMigrations() public {
    Counter counter = new Counter();
    vm.makePersistent(address(counter));

    // Round 1: Polkadot → REVM
    counter.increment();  // 1 in pallet-revive
    vm.polkadot(false);
    assertEq(counter.number(), 1);  // 1 in REVM

    // Round 2: REVM → Polkadot
    vm.polkadot(true);
    counter.increment();  // 2 in pallet-revive
    vm.polkadot(false);
    assertEq(counter.number(), 2);  // 2 in REVM
}
```

### Pattern 3: Multi-Contract Migration

```solidity
function testMultiContract() public {
    Counter counter1 = new Counter();
    Counter counter2 = new Counter();

    vm.makePersistent(address(counter1));
    vm.makePersistent(address(counter2));

    counter1.increment();
    counter2.increment();

    vm.polkadot(false);

    assertEq(counter1.number(), 1);
    assertEq(counter2.number(), 1);
}
```

### Pattern 4: Complex Storage

```solidity
function testComplexStorage() public {
    Store store = new Store();
    vm.makePersistent(address(store));

    // Set various storage types
    store.setUint(42);
    store.setAddress(address(0x1234));
    store.setMapping(1, 100);
    store.setMapping(2, 200);
    store.setArray([1, 2, 3, 4, 5]);
    store.setStruct(Data({ value: 42, active: true }));

    vm.polkadot(false);

    // All storage migrates
    assertEq(store.getUint(), 42);
    assertEq(store.getAddress(), address(0x1234));
    assertEq(store.getMapping(1), 100);
    assertEq(store.getArray(0), 1);
    assertEq(store.getStruct().value, 42);
}
```

## Performance Considerations

### Initial Migration Cost

**Factors**:
- Number of persistent accounts
- Size of contract bytecode
- Number of storage slots

**Optimization**: Only persist contracts that need migration

```solidity
// ❌ BAD: Persist everything
function testAll() public {
    Counter c1 = new Counter();
    Counter c2 = new Counter();
    Counter c3 = new Counter();

    vm.makePersistent(address(c1));
    vm.makePersistent(address(c2));
    vm.makePersistent(address(c3));

    // Only use c1
    c1.increment();
    vm.polkadot(false);
}

// ✅ GOOD: Persist only what's needed
function testOne() public {
    Counter c1 = new Counter();
    Counter c2 = new Counter();  // Not persistent
    Counter c3 = new Counter();  // Not persistent

    vm.makePersistent(address(c1));  // Only c1

    c1.increment();
    vm.polkadot(false);
}
```

### Continuous Sync Cost

**Incremental updates**: Only changed slots sync

**Cost**: O(changed_slots) not O(all_slots)

### Switching Back Cost

**Full read**: Reads all storage from pallet-revive

**Cost**: O(all_slots) for persistent contracts

## Common Issues

### Issue: Contract Lost After Switch

```solidity
// ❌ Forgot makePersistent
Counter counter = new Counter();
vm.polkadot(false);
// counter is lost
```

**Solution**: Always use `vm.makePersistent`

### Issue: Stale State in Assertions

**Cause**: State not synced back from pallet-revive

**Solution**: Continuous sync handles this automatically (Phase 2)

### Issue: Bytecode Mismatch

**Cause**: PVM mode without `--resolc` compilation

**Solution**:
```bash
forge build --resolc
forge test --polkadot=pvm
```

## References

- [Writing Tests Guide](writing-tests.md) - Using vm.polkadot and vm.makePersistent
- [Running Tests Guide](running-tests.md) - CLI flags and modes
- [Integration Tests Guide](integration-tests.md) - Adding new tests
- [Troubleshooting Guide](troubleshooting.md) - Common issues

## Related Code

- [crates/revive-strategy/src/cheatcodes/mod.rs](../../../crates/revive-strategy/src/cheatcodes/mod.rs) - Main implementation
- [CLAUDE.md](../../../CLAUDE.md) - Project documentation with architecture overview
