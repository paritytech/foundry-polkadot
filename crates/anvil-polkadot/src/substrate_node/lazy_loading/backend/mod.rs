mod block_import_operation;
mod blockchain;
mod forked_lazy_backend;

pub use block_import_operation::BlockImportOperation;
pub use blockchain::Blockchain;
pub use forked_lazy_backend::ForkedLazyBackend;

#[cfg(test)]
mod tests;

use parking_lot::RwLock;
use polkadot_sdk::{
    sc_client_api::{
        TrieCacheContext, UsageInfo,
        backend::{self, AuxStore},
    },
    sp_blockchain,
    sp_core::{H256, offchain::storage::InMemOffchainStorage},
    sp_runtime::{
        Justification, StateVersion,
        traits::{Block as BlockT, Header as HeaderT, NumberFor, One, Saturating, Zero},
    },
};
use serde::de::DeserializeOwned;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::substrate_node::lazy_loading::rpc_client::RPCClient;

pub struct Backend<Block: BlockT + DeserializeOwned> {
    fork_config: Option<(Arc<dyn RPCClient<Block>>, Block::Header)>,
    states: RwLock<HashMap<Block::Hash, ForkedLazyBackend<Block>>>,
    pub(crate) blockchain: Blockchain<Block>,
    import_lock: RwLock<()>,
    pinned_blocks: RwLock<HashMap<Block::Hash, i64>>,
}

impl<Block: BlockT + DeserializeOwned> Backend<Block> {
    fn new(fork_config: Option<(Arc<dyn RPCClient<Block>>, Block::Header)>) -> Self {
        let rpc_client = fork_config.as_ref().map(|(rpc, _)| rpc.clone());
        Self {
            fork_config,
            states: Default::default(),
            blockchain: Blockchain::new(rpc_client),
            import_lock: Default::default(),
            pinned_blocks: Default::default(),
        }
    }

    #[inline]
    pub fn rpc(&self) -> Option<&dyn RPCClient<Block>> {
        self.fork_config.as_ref().map(|(rpc, _)| rpc.as_ref())
    }

    #[inline]
    fn fork_checkpoint(&self) -> Option<&Block::Header> {
        self.fork_config.as_ref().map(|(_, checkpoint)| checkpoint)
    }
}

impl<Block: BlockT + DeserializeOwned> AuxStore for Backend<Block> {
    fn insert_aux<
        'a,
        'b: 'a,
        'c: 'a,
        I: IntoIterator<Item = &'a (&'c [u8], &'c [u8])>,
        D: IntoIterator<Item = &'a &'b [u8]>,
    >(
        &self,
        _insert: I,
        _delete: D,
    ) -> sp_blockchain::Result<()> {
        unimplemented!("`insert_aux` is not supported in lazy loading mode.")
    }

    fn get_aux(&self, _key: &[u8]) -> sp_blockchain::Result<Option<Vec<u8>>> {
        unimplemented!("`get_aux` is not supported in lazy loading mode.")
    }
}

impl<Block: BlockT + DeserializeOwned> backend::Backend<Block> for Backend<Block> {
    type BlockImportOperation = BlockImportOperation<Block>;
    type Blockchain = Blockchain<Block>;
    type State = ForkedLazyBackend<Block>;
    type OffchainStorage = InMemOffchainStorage;

    fn begin_operation(&self) -> sp_blockchain::Result<Self::BlockImportOperation> {
        let old_state = self.state_at(Default::default(), TrieCacheContext::Trusted)?;
        Ok(BlockImportOperation {
            pending_block: None,
            old_state,
            new_state: None,
            aux: Default::default(),
            storage_updates: Default::default(),
            child_storage_updates: Default::default(),
            finalized_blocks: Default::default(),
            set_head: None,
        })
    }

    fn begin_state_operation(
        &self,
        operation: &mut Self::BlockImportOperation,
        block: Block::Hash,
    ) -> sp_blockchain::Result<()> {
        operation.old_state = self.state_at(block, TrieCacheContext::Trusted)?;
        Ok(())
    }

    fn commit_operation(&self, operation: Self::BlockImportOperation) -> sp_blockchain::Result<()> {
        for (block, justification) in operation.finalized_blocks {
            self.blockchain.finalize_header(block, justification)?;
        }

        if let Some(pending_block) = operation.pending_block {
            let old_state = &operation.old_state;
            let (header, body, justification) = pending_block.block.into_inner();
            let hash = header.hash();

            let storage_updates = operation.storage_updates.clone();
            let child_storage_updates = operation.child_storage_updates.clone();

            let mut removed_keys = old_state.removed_keys.read().clone();
            for (key, value) in &storage_updates {
                if value.is_some() {
                    removed_keys.remove(key);
                } else {
                    removed_keys.insert(key.clone());
                }
            }

            for (child_key, child_data) in &child_storage_updates {
                for (key, value) in child_data {
                    let composite_key = make_composite_child_key(child_key, key);
                    if value.is_some() {
                        removed_keys.remove(&composite_key);
                    } else {
                        removed_keys.insert(composite_key);
                    }
                }
            }

            let new_removed_keys = Arc::new(RwLock::new(removed_keys));

            let mut db_clone = old_state.db.read().clone();
            {
                let mut entries = vec![(None, storage_updates.clone())];
                if !child_storage_updates.is_empty() {
                    entries.extend(child_storage_updates.iter().map(|(key, data)| {
                        (Some(polkadot_sdk::sp_storage::ChildInfo::new_default(key)), data.clone())
                    }));
                }
                db_clone.insert(entries, StateVersion::V1);
            }
            let new_db = Arc::new(RwLock::new(db_clone));
            let fork_block = self.fork_checkpoint().map(|checkpoint| checkpoint.hash());
            let rpc_client = self.fork_config.as_ref().map(|(rpc, _)| rpc.clone());
            let new_state = ForkedLazyBackend {
                rpc_client,
                block_hash: Some(hash),
                fork_block,
                db: new_db,
                removed_keys: new_removed_keys,
                before_fork: false,
            };
            self.states.write().insert(hash, new_state);

            self.blockchain.insert(hash, header, justification, body, pending_block.state)?;
        }

        if !operation.aux.is_empty() {
            self.blockchain.write_aux(operation.aux);
        }

        if let Some(set_head) = operation.set_head {
            self.blockchain.set_head(set_head)?;
        }

        Ok(())
    }

    fn finalize_block(
        &self,
        hash: Block::Hash,
        justification: Option<Justification>,
    ) -> sp_blockchain::Result<()> {
        self.blockchain.finalize_header(hash, justification)
    }

    fn append_justification(
        &self,
        hash: Block::Hash,
        justification: Justification,
    ) -> sp_blockchain::Result<()> {
        self.blockchain.append_justification(hash, justification)
    }

    fn blockchain(&self) -> &Self::Blockchain {
        &self.blockchain
    }

    fn usage_info(&self) -> Option<UsageInfo> {
        None
    }

    fn offchain_storage(&self) -> Option<Self::OffchainStorage> {
        None
    }

    fn state_at(
        &self,
        hash: Block::Hash,
        _trie_cache_context: TrieCacheContext,
    ) -> sp_blockchain::Result<Self::State> {
        if hash == Default::default() {
            let (fork_block, before_fork) = match self.fork_checkpoint() {
                Some(checkpoint) => (Some(checkpoint.hash()), true),
                None => (None, false),
            };

            let rpc_client = self.fork_config.as_ref().map(|(rpc, _)| rpc.clone());
            return Ok(ForkedLazyBackend::<Block> {
                rpc_client,
                block_hash: Some(hash),
                fork_block,
                db: Default::default(),
                removed_keys: Default::default(),
                before_fork,
            });
        }

        let (backend, should_write) = self
            .states
            .read()
            .get(&hash)
            .cloned()
            .map(|state| Ok((state, false)))
            .unwrap_or_else(|| {
                self.rpc()
                    .and_then(|rpc| rpc.header(Some(hash)).ok())
                    .flatten()
                    .ok_or(sp_blockchain::Error::UnknownBlock(format!(
                        "Failed to fetch block header: {hash:?}"
                    )))
                    .map(|header| {
                        let rpc_client = self.fork_config.as_ref().map(|(rpc, _)| rpc.clone());
                        let state = match self.fork_checkpoint() {
                            Some(checkpoint) => {
                                if header.number().gt(checkpoint.number()) {
                                    let parent = self
                                        .state_at(*header.parent_hash(), TrieCacheContext::Trusted)
                                        .ok();

                                    ForkedLazyBackend::<Block> {
                                        rpc_client: rpc_client.clone(),
                                        block_hash: Some(hash),
                                        fork_block: Some(checkpoint.hash()),
                                        db: parent.clone().map_or(Default::default(), |p| p.db),
                                        removed_keys: parent
                                            .map_or(Default::default(), |p| p.removed_keys),
                                        before_fork: false,
                                    }
                                } else {
                                    ForkedLazyBackend::<Block> {
                                        rpc_client: rpc_client.clone(),
                                        block_hash: Some(hash),
                                        fork_block: Some(checkpoint.hash()),
                                        db: Default::default(),
                                        removed_keys: Default::default(),
                                        before_fork: true,
                                    }
                                }
                            }
                            None => {
                                let parent = self
                                    .state_at(*header.parent_hash(), TrieCacheContext::Trusted)
                                    .ok();

                                ForkedLazyBackend::<Block> {
                                    rpc_client: rpc_client.clone(),
                                    block_hash: Some(hash),
                                    fork_block: None,
                                    db: parent.clone().map_or(Default::default(), |p| p.db),
                                    removed_keys: parent
                                        .map_or(Default::default(), |p| p.removed_keys),
                                    before_fork: false,
                                }
                            }
                        };

                        (state, true)
                    })
            })?;

        if should_write {
            self.states.write().insert(hash, backend.clone());
        }

        Ok(backend)
    }

    fn revert(
        &self,
        n: NumberFor<Block>,
        revert_finalized: bool,
    ) -> sp_blockchain::Result<(NumberFor<Block>, HashSet<Block::Hash>)> {
        let mut storage = self.blockchain.storage.write();

        if storage.blocks.is_empty() || n.is_zero() {
            return Ok((Zero::zero(), HashSet::new()));
        }

        let mut states = self.states.write();
        let pinned = self.pinned_blocks.read();

        let mut target = n;
        let original_finalized_number = storage.finalized_number;

        if !revert_finalized {
            let revertible = storage.best_number.saturating_sub(storage.finalized_number);
            if target > revertible {
                target = revertible;
            }
        }

        let mut reverted = NumberFor::<Block>::zero();
        let mut reverted_up_to_finalized = HashSet::new();

        let mut current_hash = storage.best_hash;

        while reverted < target {
            // Stop if we've reached genesis or if there are no blocks in storage
            if storage.blocks.is_empty() || current_hash == storage.genesis_hash {
                break;
            }

            if let Some(count) = pinned.get(&current_hash)
                && *count > 0
            {
                break;
            }

            let Some(block) = storage.blocks.get(&current_hash) else {
                break;
            };

            let header = block.header().clone();
            let number = *header.number();
            let parent_hash = header.parent_hash();

            // If this is the genesis block, parent doesn't become a leaf
            // Otherwise, check if any other block has the same parent
            let parent_becomes_leaf = if current_hash == storage.genesis_hash {
                false
            } else {
                !storage.blocks.iter().any(|(other_hash, stored)| {
                    *other_hash != current_hash && stored.header().parent_hash() == parent_hash
                })
            };

            let hash_to_remove = current_hash;

            storage.blocks.remove(&hash_to_remove);
            if let Some(entry) = storage.hashes.get(&number)
                && *entry == hash_to_remove
            {
                storage.hashes.remove(&number);
            }
            states.remove(&hash_to_remove);

            storage.leaves.remove(
                hash_to_remove,
                number,
                parent_becomes_leaf.then_some(*parent_hash),
            );

            if number <= original_finalized_number {
                reverted_up_to_finalized.insert(hash_to_remove);
            }

            reverted = reverted.saturating_add(One::one());

            current_hash = *parent_hash;

            storage.best_hash = current_hash;
            storage.best_number = number.saturating_sub(One::one());
        }

        let best_hash_after = storage.best_hash;
        let best_number_after = storage.best_number;

        let _ = storage.leaves.revert(best_hash_after, best_number_after);

        storage.hashes.insert(best_number_after, best_hash_after);

        if storage.finalized_number > best_number_after {
            storage.finalized_number = best_number_after;
        }

        // Get the genesis block number to use as lower bound
        let genesis_number = storage
            .blocks
            .get(&storage.genesis_hash)
            .map(|block| *block.header().number())
            .unwrap_or(Zero::zero());

        // Decrement finalized_number until we find a valid block, but don't go below genesis
        while storage.finalized_number > genesis_number
            && !storage.hashes.contains_key(&storage.finalized_number)
        {
            storage.finalized_number = storage.finalized_number.saturating_sub(One::one());
        }

        if let Some(hash) = storage.hashes.get(&storage.finalized_number).copied() {
            storage.finalized_hash = hash;
        } else {
            storage.finalized_hash = storage.genesis_hash;
        }

        Ok((reverted, reverted_up_to_finalized))
    }

    fn remove_leaf_block(&self, _hash: Block::Hash) -> sp_blockchain::Result<()> {
        // Not used
        Ok(())
    }

    fn get_import_lock(&self) -> &RwLock<()> {
        &self.import_lock
    }

    fn requires_full_sync(&self) -> bool {
        false
    }

    fn pin_block(&self, hash: <Block as BlockT>::Hash) -> sp_blockchain::Result<()> {
        let mut blocks = self.pinned_blocks.write();
        *blocks.entry(hash).or_default() += 1;
        Ok(())
    }

    fn unpin_block(&self, hash: <Block as BlockT>::Hash) {
        let mut blocks = self.pinned_blocks.write();
        blocks.entry(hash).and_modify(|counter| *counter -= 1).or_insert(-1);
    }
}

impl<Block: BlockT + DeserializeOwned> backend::LocalBackend<Block> for Backend<Block> {}

pub fn new_backend<Block>(
    fork_config: Option<(Arc<dyn RPCClient<Block>>, Block)>,
) -> Result<Arc<Backend<Block>>, polkadot_sdk::sc_service::Error>
where
    Block: BlockT + DeserializeOwned,
    Block::Hash: From<H256>,
{
    let fork_config = fork_config.map(|(rpc, block)| (rpc, block.header().clone()));
    let backend = Arc::new(Backend::new(fork_config));
    Ok(backend)
}

/// Creates a composite key for child storage by combining child_storage_key + key.
/// This ensures keys from different child storages don't collide.
#[inline]
pub fn make_composite_child_key(child_storage_key: &[u8], key: &[u8]) -> Vec<u8> {
    let mut composite = Vec::with_capacity(child_storage_key.len() + key.len());
    composite.extend_from_slice(child_storage_key);
    composite.extend_from_slice(key);
    composite
}
