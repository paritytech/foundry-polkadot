use crate::substrate_node::service::{Backend, FullClient};
use alloy_primitives::U256;
use polkadot_sdk::{
    sc_client_api::Backend as BackendT,
    sp_blockchain::{HeaderBackend, Info, Result},
};
use std::{collections::BTreeMap, sync::Arc};
use substrate_runtime::OpaqueBlock;

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub best_number: u64,
}

pub struct RevertInfo {
    pub info: Info<OpaqueBlock>,
    pub reverted: u64,
}

pub struct SnapshotManager {
    client: Arc<FullClient>,
    backend: Arc<Backend>,
    next_snapshot_id: U256,
    snapshots: BTreeMap<U256, Snapshot>,
}

impl SnapshotManager {
    pub fn new(client: Arc<FullClient>, backend: Arc<Backend>) -> Self {
        Self { client, backend, next_snapshot_id: U256::ZERO, snapshots: BTreeMap::new() }
    }
}

impl SnapshotManager {
    /// Create a snapshot id corresponding to the best block number.
    pub fn snapshot(&mut self) -> U256 {
        let one = U256::ONE;
        self.next_snapshot_id += one;
        let snapshot = Snapshot { best_number: self.client.info().best_number.into() };
        self.snapshots.insert(self.next_snapshot_id - one, snapshot);
        // Safe since there is always a snapshot representing genesis.
        self.next_snapshot_id - one
    }

    /// Revert the chain to the block number represented by the snapshot `id`.
    pub fn revert(&mut self, snapshot_id: U256) -> Result<Option<RevertInfo>> {
        // Remove the snapshot when reverting. We do not want to keep it around
        // since reverting to an existing snapshot could mean going back to future,
        // which is not supported.
        let maybe_snapshot = self.snapshots.remove(&snapshot_id);
        let Some(snap) = maybe_snapshot else {
            return Ok(None);
        };

        let current_best_number: u64 = self.client.info().best_number.into();
        let number_of_blocks_to_revert = current_best_number - snap.best_number;

        let (reverted, _) = self.backend.revert(
            number_of_blocks_to_revert.try_into().expect("to not surpass u32 bounds"),
            true,
        )?;

        self.snapshots.retain(|_, snap_to_remove| snap_to_remove.best_number < snap.best_number);

        Ok(Some(RevertInfo { reverted: reverted.into(), info: self.client.info() }))
    }

    pub fn rollback(&self, depth: Option<u64>) -> Result<RevertInfo> {
        let (reverted, _) = self
            .backend
            .revert(depth.unwrap_or(1).try_into().expect("to not surpass u32 bounds"), true)?;
        Ok(RevertInfo { reverted: reverted.into(), info: self.client.info() })
    }
}
