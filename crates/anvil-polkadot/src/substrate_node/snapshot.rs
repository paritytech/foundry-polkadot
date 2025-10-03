use crate::substrate_node::service::Backend;
use polkadot_sdk::{
    sc_client_api::Backend as BackendT,
    sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata, Result},
};
use std::{collections::BTreeMap, num::NonZeroU64, sync::Arc};
use substrate_runtime::OpaqueBlock;

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub best_number: u64,
}

pub struct SnapshotManager<C> {
    client: Arc<C>,
    backend: Arc<Backend>,
    next_snapshot_id: u64,
    snapshots: BTreeMap<u64, Snapshot>,
}

impl<C> SnapshotManager<C> {
    pub fn new(client: Arc<C>, backend: Arc<Backend>, genesis_block_number: u64) -> Self {
        let snapshot = Snapshot { best_number: genesis_block_number };
        let mut map = BTreeMap::new();
        map.insert(0, snapshot);

        Self {
            client,
            backend,
            // Start with 1 to mimic Ganache
            next_snapshot_id: 1,
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
    /// Create a snapshot id corresponding to the best block number.
    pub fn snapshot(&mut self) -> u64 {
        self.next_snapshot_id += 1;
        let snapshot = Snapshot { best_number: self.client.info().best_number.into() };
        self.snapshots.insert(self.next_snapshot_id - 1, snapshot);
        // Safe since there is always a snapshot representing genesis.
        self.next_snapshot_id - 1
    }

    /// Revert the chain to the block number represented by the snapshot `id`.
    pub fn revert(&mut self, id: &NonZeroU64) -> Result<bool> {
        let maybe_snapshot = self.snapshots.remove(&id.get());
        let Some(snap) = maybe_snapshot else { return Ok(false) };

        let current_best_number: u64 = self.client.info().best_number.into();
        let number_of_blocks_to_revert = current_best_number - snap.best_number;

        self.backend.revert(
            number_of_blocks_to_revert.try_into().expect("to not surpass u32 bounds"),
            true,
        )?;

        self.snapshots.retain(|&k, _| k < id.get());

        Ok(true)
    }

    pub fn rollback(&self) -> Result<bool> {
        let current_best_number = self.client.info().best_number;
        self.backend.revert(current_best_number, true)?;

        Ok(true)
    }
}
