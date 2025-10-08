use crate::substrate_node::service::Backend;
use alloy_primitives::U256;
use polkadot_sdk::{
    sc_client_api::Backend as BackendT,
    sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata, Result},
};
use std::{collections::BTreeMap, sync::Arc};
use substrate_runtime::OpaqueBlock;

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub best_number: u64,
}

pub struct SnapshotManager<C> {
    client: Arc<C>,
    backend: Arc<Backend>,
    next_snapshot_id: U256,
    snapshots: BTreeMap<U256, Snapshot>,
}

impl<C> SnapshotManager<C> {
    pub fn new(client: Arc<C>, backend: Arc<Backend>, genesis_block_number: u64) -> Self {
        let snapshot = Snapshot { best_number: genesis_block_number };
        let mut map = BTreeMap::new();
        map.insert(U256::ZERO, snapshot);

        Self {
            client,
            backend,
            // Start with 1 to mimic Ganache
            next_snapshot_id: U256::ONE,
            snapshots: map,
        }
    }
}

impl<C> SnapshotManager<C>
where
    C: HeaderBackend<OpaqueBlock>
        + HeaderMetadata<OpaqueBlock, Error = BlockChainError>
        + Send
        + Sync
        + 'static,
{
    /// Get the block number associated to a snapshot id.
    pub fn block_number_for(&mut self, snapshot_id: U256) -> Option<u64> {
        self.snapshots.get(&snapshot_id).map(|snap| snap.best_number)
    }

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
    pub fn revert(&mut self, snapshot_id: U256) -> Result<bool> {
        // Remove the snapshot when reverting. We do not want to keep it around
        // since reverting to an existing snapshot could mean going back to future,
        // which is not supported.
        let maybe_snapshot = self.snapshots.remove(&snapshot_id);
        let Some(snap) = maybe_snapshot else { return Ok(false) };

        let current_best_number: u64 = self.client.info().best_number.into();
        let number_of_blocks_to_revert = current_best_number - snap.best_number;

        self.backend.revert(
            number_of_blocks_to_revert.try_into().expect("to not surpass u32 bounds"),
            true,
        )?;

        self.snapshots.retain(|&k, snap_to_remove| {
            k < snapshot_id || snap_to_remove.best_number >= snap.best_number
        });

        Ok(true)
    }

    pub fn rollback(&self) -> Result<bool> {
        let current_best_number = self.client.info().best_number;
        self.backend.revert(current_best_number, true)?;

        Ok(true)
    }
}
