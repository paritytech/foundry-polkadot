use crate::substrate_node::service::{Backend, Client, TransactionPoolHandle};
use alloy_primitives::{B256, U256};
use indexmap::IndexMap;
use polkadot_sdk::{
    polkadot_sdk_frame::runtime::types_common::OpaqueBlock,
    sc_client_api::{Backend as BackendT, BlockBackend},
    sc_service::TransactionPool,
    sc_transaction_pool_api::{ChainEvent, MaintainedTransactionPool},
    sp_blockchain::{Error, HeaderBackend, Info, Result, tree_route},
    sp_runtime::transaction_validity::{
        InvalidTransaction as SpInvalidTransaction,
        TransactionValidityError as SpTransactionValidityError,
    },
};
use std::{collections::BTreeMap, sync::Arc};
use subxt::utils::H256;

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
    transaction_pool: Arc<TransactionPoolHandle>,
    genesis_block_number: u32,
}

impl RevertManager {
    pub fn new(
        client: Arc<Client>,
        backend: Arc<Backend>,
        transaction_pool: Arc<TransactionPoolHandle>,
        genesis_block_number: u32,
    ) -> Self {
        Self {
            client,
            backend,
            next_snapshot_id: U256::ZERO,
            snapshots: BTreeMap::new(),
            transaction_pool,
            genesis_block_number,
        }
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
    pub async fn revert(&mut self, snapshot_id: U256) -> Result<Option<RevertInfo>> {
        let maybe_snapshot = self.snapshots.remove(&snapshot_id);
        let Some((snapshot_block_number, snapshot_block_hash)) = maybe_snapshot else {
            return Ok(None);
        };

        let current_best_number: u64 = self.client.info().best_number.into();
        let number_of_blocks_to_revert = current_best_number - snapshot_block_number;

        let current_best_hash = self.client.info().best_hash;
        let snapshot_block_hash = H256::from_slice(snapshot_block_hash.as_ref());

        self.revert_and_clean(current_best_hash, snapshot_block_hash).await?;

        let (reverted, _) =
            self.backend.revert(number_of_blocks_to_revert.try_into().unwrap_or(u32::MAX), true)?;

        self.snapshots.retain(|_, (snap_to_remove, _)| *snap_to_remove < snapshot_block_number);

        Ok(Some(RevertInfo { reverted: reverted.into(), info: self.client.info() }))
    }

    /// Revert from best block to a parent represented by current block height minus depth.
    pub async fn rollback(&self, depth: Option<u64>) -> Result<RevertInfo> {
        let depth = depth.unwrap_or(1).try_into().unwrap_or(u32::MAX);
        let current_info = self.client.info();
        let current_best_hash = current_info.best_hash;
        let target_number = current_info
            .best_number
            .checked_sub(depth)
            .unwrap_or(self.genesis_block_number) // If underflow, clamp to genesis
            .max(self.genesis_block_number);
        let Some(target_hash) = self.client.hash(target_number).ok().flatten() else {
            return Err(Error::UnknownBlock(format!(
                "Hash not found for block number: {}",
                target_number
            )));
        };
        self.revert_and_clean(current_best_hash, target_hash).await?;

        let (reverted, _) = self.backend.revert(depth, true)?;
        Ok(RevertInfo { reverted: reverted.into(), info: self.client.info() })
    }

    /// Will revert to genesis.
    pub async fn reset_to_genesis(&self) -> Result<RevertInfo> {
        let current_block_number = self.client.info().best_number;

        let current_best_hash = self.client.info().best_hash;
        let Some(genesis_hash) = self.client.hash(self.genesis_block_number).ok().flatten() else {
            return Err(Error::UnknownBlock(format!(
                "Genesis hash not found for genesis block number: {}",
                self.genesis_block_number
            )));
        };
        self.revert_and_clean(current_best_hash, genesis_hash).await?;

        let (reverted, _) = self.backend.revert(current_block_number, true)?;

        // The chain info can refer to a genesis block with a number different than 0, based on how
        // the node was started, so we will query the state once more to return accurate info.
        Ok(RevertInfo { reverted: reverted.into(), info: self.client.info() })
    }

    pub fn list_snapshots(&self) -> BTreeMap<U256, (u64, B256)> {
        self.snapshots.clone()
    }

    async fn revert_and_clean(
        &self,
        current_best_hash: H256,
        snapshot_block_hash: H256,
    ) -> Result<()> {
        let t = tree_route(self.backend.blockchain(), current_best_hash, snapshot_block_hash)?;
        let retracted_hashes =
            t.retracted().iter().map(|hash_and_number| hash_and_number.hash).collect::<Vec<_>>();

        // Collect ALL transaction hashes from blocks being reverted
        let mut txs_to_remove = Vec::new();
        for block_hash in &retracted_hashes {
            if let Ok(Some(signed_block)) = self.client.block(*block_hash) {
                for ext in signed_block.block.extrinsics {
                    let tx_hash = self.transaction_pool.hash_of(&ext);
                    txs_to_remove.push(tx_hash);
                }
            }
        }

        self.transaction_pool
            .maintain(ChainEvent::Finalized {
                hash: snapshot_block_hash,
                tree_route: retracted_hashes.into(),
            })
            .await;

        let invalid_txs: IndexMap<_, _> = txs_to_remove
            .into_iter()
            .map(|hash| {
                (hash, Some(SpTransactionValidityError::Invalid(SpInvalidTransaction::Stale)))
            })
            .collect();

        self.transaction_pool.report_invalid(Some(snapshot_block_hash), invalid_txs).await;
        Ok(())
    }
}
