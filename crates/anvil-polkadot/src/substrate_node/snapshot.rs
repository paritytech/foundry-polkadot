use crate::substrate_node::service::{Backend, Client};
use alloy_primitives::{B256, U256};
use parking_lot::Mutex;
use polkadot_sdk::{
    polkadot_sdk_frame::runtime::types_common::OpaqueBlock,
    sc_client_api::Backend as BackendT,
    sp_blockchain::{HeaderBackend, Info, Result},
};
use std::{collections::BTreeMap, sync::Arc};

type Snapshot = u64;
type Hash = B256;

pub struct RevertInfo {
    pub info: Info<OpaqueBlock>,
    pub reverted: u64,
}

pub struct SnapshotManager {
    client: Arc<Client>,
    backend: Arc<Backend>,
    next_snapshot_id: U256,
    snapshots: Arc<Mutex<BTreeMap<U256, (Snapshot, Hash)>>>,
}

impl SnapshotManager {
    pub fn new(client: Arc<Client>, backend: Arc<Backend>) -> Self {
        Self {
            client,
            backend,
            next_snapshot_id: U256::ZERO,
            snapshots: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
}

impl SnapshotManager {
    /// Create a snapshot id corresponding to the best block number.
    pub fn snapshot(&mut self) -> U256 {
        let current_snapshot_id = self.next_snapshot_id;
        self.next_snapshot_id += U256::ONE;
        let snapshot = self.client.info().best_number.into();
        let hash = B256::from_slice(self.client.info().best_hash.as_ref());
        self.snapshots.lock().insert(current_snapshot_id, (snapshot, hash));
        current_snapshot_id
    }

    /// Revert the chain to the block number represented by the snapshot `id`.
    pub fn revert(&mut self, snapshot_id: U256) -> Result<Option<RevertInfo>> {
        let maybe_snapshot_with_hash = self.snapshots.lock().remove(&snapshot_id);
        let Some((snap, _)) = maybe_snapshot_with_hash else {
            return Ok(None);
        };

        let current_best_number: u64 = self.client.info().best_number.into();
        let number_of_blocks_to_revert = current_best_number - snap;

        let (reverted, _) =
            self.backend.revert(number_of_blocks_to_revert.try_into().unwrap_or(u32::MAX), true)?;

        self.snapshots.lock().retain(|_, (snap_to_remove, _)| *snap_to_remove < snap);

        Ok(Some(RevertInfo { reverted: reverted.into(), info: self.client.info() }))
    }

    /// Revert from best block to a parent represented by current block height minus depth.
    pub fn rollback(&self, depth: Option<u64>) -> Result<RevertInfo> {
        let (reverted, _) =
            self.backend.revert(depth.unwrap_or(1).try_into().unwrap_or(u32::MAX), true)?;
        Ok(RevertInfo { reverted: reverted.into(), info: self.client.info() })
    }

    pub fn list_snapshots(&self) -> BTreeMap<U256, (u64, B256)> {
        self.snapshots.lock().clone()
    }
}
