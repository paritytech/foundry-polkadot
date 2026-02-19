use crate::substrate_node::service::{Backend, Client};
use alloy_primitives::{B256, U256};
use polkadot_sdk::{
    polkadot_sdk_frame::runtime::types_common::OpaqueBlock,
    sc_client_api::Backend as BackendT,
    sp_blockchain::{HeaderBackend, Info, Result},
};
use std::{collections::BTreeMap, sync::Arc};

// The snapshot contains the block number and the block hash
type Snapshot = (u64, B256);

pub struct RevertInfo {
    pub info: Info<OpaqueBlock>,
    pub reverted: u64,
}

pub struct RevertManager {
    client: Arc<Client>,
    backend: Arc<Backend>,
    next_snapshot_id: U256,
    snapshots: BTreeMap<U256, Snapshot>,
}

impl RevertManager {
    pub fn new(client: Arc<Client>, backend: Arc<Backend>) -> Self {
        Self { client, backend, next_snapshot_id: U256::ZERO, snapshots: BTreeMap::new() }
    }
}

impl RevertManager {
    /// Create a snapshot id corresponding to the best block number.
    pub fn snapshot(&mut self) -> U256 {
        let current_snapshot_id = self.next_snapshot_id;
        self.next_snapshot_id += U256::ONE;
        let block_number = self.client.info().best_number.into();
        let block_hash = B256::from_slice(self.client.info().best_hash.as_ref());
        self.snapshots.insert(current_snapshot_id, (block_number, block_hash));
        current_snapshot_id
    }

    /// Revert the chain to the block number represented by the snapshot `id`.
    pub fn revert(&mut self, snapshot_id: U256) -> Result<Option<RevertInfo>> {
        let maybe_snapshot = self.snapshots.remove(&snapshot_id);
        let Some((snapshot_block_number, _)) = maybe_snapshot else {
            return Ok(None);
        };

        let current_best_number: u64 = self.client.info().best_number.into();
        let number_of_blocks_to_revert = current_best_number - snapshot_block_number;

        let (reverted, _) =
            self.backend.revert(number_of_blocks_to_revert.try_into().unwrap_or(u32::MAX), true)?;

        self.snapshots.retain(|_, (snap_to_remove, _)| *snap_to_remove < snapshot_block_number);

        Ok(Some(RevertInfo { reverted: reverted.into(), info: self.client.info() }))
    }

    /// Revert from best block to a parent represented by current block height minus depth.
    pub fn rollback(&self, depth: Option<u64>) -> Result<RevertInfo> {
        let (reverted, _) =
            self.backend.revert(depth.unwrap_or(1).try_into().unwrap_or(u32::MAX), true)?;
        Ok(RevertInfo { reverted: reverted.into(), info: self.client.info() })
    }

    /// Will revert to genesis.
    pub fn reset_to_genesis(&self) -> Result<RevertInfo> {
        let current_block_number = self.client.info().best_number;
        let (reverted, _) = self.backend.revert(current_block_number, true)?;

        // The chain info can refer to a genesis block with a number different than 0, based on how
        // the node was started, so we will query the state once more to return accurate info.
        Ok(RevertInfo { reverted: reverted.into(), info: self.client.info() })
    }

    pub fn list_snapshots(&self) -> BTreeMap<U256, (u64, B256)> {
        self.snapshots.clone()
    }
}
