//! Polkadot Revive state snapshot management.
//!
//! This module provides functions to save and restore Polkadot Revive contract state
//! snapshots, synchronized with Foundry's `vm.snapshotState()` and `vm.revertToState()`
//! cheatcodes.

use alloy_primitives::U256;
use polkadot_sdk::{
    sp_core::{self, H160},
    sp_io,
    sp_state_machine::InMemoryBackend,
};
use revive_env::ExtBuilder;
use std::collections::HashMap;

std::thread_local! {
    /// Stores Polkadot Revive state snapshots, keyed by Foundry snapshot ID
    pub static REVIVE_SNAPSHOTS: std::cell::RefCell<HashMap<U256, InMemoryBackend<sp_core::Blake2Hasher>>> =
        std::cell::RefCell::new(HashMap::new());

    /// The main Polkadot Revive test externalities storage
    pub static TEST_EXTERNALITIES: std::cell::RefCell<sp_io::TestExternalities> = std::cell::RefCell::new(ExtBuilder::default()
        .balance_genesis_config(vec![(H160::from_low_u64_be(1), 1000)])
        .build());
}

/// Saves a Polkadot Revive state snapshot with the given snapshot ID
pub fn save_revive_snapshot(snapshot_id: U256) {
    TEST_EXTERNALITIES.with_borrow_mut(|test_ext| {
        let backend = test_ext.as_backend();
        REVIVE_SNAPSHOTS.with_borrow_mut(|snapshots| {
            snapshots.insert(snapshot_id, backend);
        });
    });
}

/// Reverts to a Polkadot Revive state snapshot with the given snapshot ID
pub fn revert_revive_snapshot(snapshot_id: U256) -> bool {
    REVIVE_SNAPSHOTS.with_borrow_mut(|snapshots| {
        if let Some(backend) = snapshots.get(&snapshot_id).cloned() {
            let mut test_externalities = ExtBuilder::default().build();
            let mut backend_copy = backend;
            std::mem::swap(&mut test_externalities.backend, &mut backend_copy);
            TEST_EXTERNALITIES.set(test_externalities);
            true
        } else {
            false
        }
    })
}

/// Deletes a Polkadot Revive state snapshot with the given snapshot ID
pub fn delete_revive_snapshot(snapshot_id: U256) -> bool {
    REVIVE_SNAPSHOTS.with_borrow_mut(|snapshots| snapshots.remove(&snapshot_id).is_some())
}

/// Deletes all Polkadot Revive state snapshots
pub fn delete_all_revive_snapshots() {
    REVIVE_SNAPSHOTS.with_borrow_mut(|snapshots| {
        snapshots.clear();
    });
}

/// Executes a closure with mutable access to the test externalities
pub fn execute_with_externalities<R, F: FnOnce(&mut sp_io::TestExternalities) -> R>(f: F) -> R {
    TEST_EXTERNALITIES.with_borrow_mut(f)
}
