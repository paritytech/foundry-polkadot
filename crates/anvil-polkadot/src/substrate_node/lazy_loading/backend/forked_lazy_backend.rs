use super::make_composite_child_key;
use crate::substrate_node::lazy_loading::rpc_client::RPCClient;
use parking_lot::RwLock;
use polkadot_sdk::{
    sc_client_api::StorageKey,
    sp_core,
    sp_runtime::{
        StateVersion,
        traits::{Block as BlockT, HashingFor},
    },
    sp_state_machine::{
        self, BackendTransaction, InMemoryBackend, IterArgs, StorageCollection, StorageValue,
        TrieBackend, backend::AsTrieBackend,
    },
    sp_storage::ChildInfo,
    sp_trie::{self, PrefixedMemoryDB},
};
use serde::de::DeserializeOwned;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

/// DB-backed patricia trie state, transaction type is an overlay of changes to commit.
pub type DbState<B> = TrieBackend<Arc<dyn sp_state_machine::Storage<HashingFor<B>>>, HashingFor<B>>;

/// Simple wrapper around the InMemoryBackend's RawIter that delegates all operations.
pub struct RawIter<Block: BlockT + DeserializeOwned> {
    inner: <InMemoryBackend<HashingFor<Block>> as sp_state_machine::Backend<HashingFor<Block>>>::RawIter,
}

impl<Block: BlockT + DeserializeOwned> sp_state_machine::StorageIterator<HashingFor<Block>>
    for RawIter<Block>
{
    type Backend = ForkedLazyBackend<Block>;
    type Error = String;

    fn next_key(
        &mut self,
        backend: &Self::Backend,
    ) -> Option<Result<sp_state_machine::StorageKey, Self::Error>> {
        let db = backend.db.read();
        self.inner.next_key(&*db)
    }

    fn next_pair(
        &mut self,
        backend: &Self::Backend,
    ) -> Option<Result<(sp_state_machine::StorageKey, sp_state_machine::StorageValue), Self::Error>>
    {
        let db = backend.db.read();
        self.inner.next_pair(&*db)
    }

    fn was_complete(&self) -> bool {
        self.inner.was_complete()
    }
}

#[derive(Debug, Clone)]
pub struct ForkedLazyBackend<Block: BlockT + DeserializeOwned> {
    pub(crate) rpc_client: Option<Arc<dyn RPCClient<Block>>>,
    pub(crate) block_hash: Option<Block::Hash>,
    pub(crate) fork_block: Option<Block::Hash>,
    pub(crate) db: Arc<RwLock<InMemoryBackend<HashingFor<Block>>>>,

    /// Keys explicitly deleted after fork. Prevents RPC fallback from returning stale values
    /// for deleted keys (distinguishes "not cached locally" from "deleted").
    pub(crate) removed_keys: Arc<RwLock<HashSet<Vec<u8>>>>,

    pub(crate) before_fork: bool,
}

impl<Block: BlockT + DeserializeOwned> ForkedLazyBackend<Block> {
    pub(crate) fn update_storage(&self, key: &[u8], value: &Option<Vec<u8>>) {
        if let Some(val) = value {
            let mut entries: HashMap<Option<ChildInfo>, StorageCollection> = Default::default();
            entries.insert(None, vec![(key.to_vec(), Some(val.clone()))]);

            self.db.write().insert(entries, StateVersion::V1);
        }
    }

    pub(crate) fn update_child_storage(
        &self,
        child_info: &ChildInfo,
        key: &[u8],
        value: &Option<Vec<u8>>,
    ) {
        if let Some(val) = value {
            let mut entries: HashMap<Option<ChildInfo>, StorageCollection> = Default::default();
            entries.insert(Some(child_info.clone()), vec![(key.to_vec(), Some(val.clone()))]);

            self.db.write().insert(entries, StateVersion::V1);
        }
    }

    #[inline]
    pub(crate) fn rpc(&self) -> Option<&dyn RPCClient<Block>> {
        self.rpc_client.as_deref()
    }
}

impl<Block: BlockT + DeserializeOwned> sp_state_machine::Backend<HashingFor<Block>>
    for ForkedLazyBackend<Block>
{
    type Error = <DbState<Block> as sp_state_machine::Backend<HashingFor<Block>>>::Error;
    type TrieBackendStorage = PrefixedMemoryDB<HashingFor<Block>>;
    type RawIter = RawIter<Block>;

    fn storage(&self, key: &[u8]) -> Result<Option<StorageValue>, Self::Error> {
        let remote_fetch = |block: Option<Block::Hash>| -> Option<Vec<u8>> {
            self.rpc()
                .and_then(|rpc| rpc.storage(StorageKey(key.to_vec()), block).ok())
                .flatten()
                .map(|v| v.0)
        };

        // When before_fork, try RPC first, then fall back to local DB
        if self.before_fork {
            if self.rpc().is_some() {
                return Ok(remote_fetch(self.block_hash));
            } else {
                // No RPC client, try to read from local DB
                let readable_db = self.db.read();
                return Ok(readable_db.storage(key).ok().flatten());
            }
        }

        let readable_db = self.db.read();
        let maybe_storage = readable_db.storage(key);
        let value = match maybe_storage {
            Ok(Some(data)) => Some(data),
            _ if !self.removed_keys.read().contains(key) => {
                let result =
                    if self.rpc().is_some() { remote_fetch(self.fork_block) } else { None };

                drop(readable_db);
                self.update_storage(key, &result);

                result
            }
            _ => None,
        };

        Ok(value)
    }

    fn storage_hash(
        &self,
        key: &[u8],
    ) -> Result<Option<<HashingFor<Block> as sp_core::Hasher>::Out>, Self::Error> {
        let remote_fetch = |block: Option<Block::Hash>| -> Result<
            Option<<HashingFor<Block> as sp_core::Hasher>::Out>,
            Self::Error,
        > {
            match self.rpc() {
                Some(rpc) => rpc
                    .storage_hash(StorageKey(key.to_vec()), block)
                    .map_err(|e| format!("Failed to fetch storage hash from RPC: {e:?}")),
                None => Ok(None),
            }
        };

        // When before_fork, try RPC first, then fall back to local DB
        if self.before_fork {
            if self.rpc().is_some() {
                return remote_fetch(self.block_hash);
            } else {
                // No RPC client, try to read from local DB
                return Ok(self.db.read().storage_hash(key).ok().flatten());
            }
        }

        let storage_hash = self.db.read().storage_hash(key);
        match storage_hash {
            Ok(Some(hash)) => Ok(Some(hash)),
            _ if !self.removed_keys.read().contains(key) => {
                if self.rpc().is_some() {
                    remote_fetch(self.fork_block)
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    fn closest_merkle_value(
        &self,
        _key: &[u8],
    ) -> Result<
        Option<sp_trie::MerkleValue<<HashingFor<Block> as sp_core::Hasher>::Out>>,
        Self::Error,
    > {
        unimplemented!("closest_merkle_value: unsupported feature for lazy loading")
    }

    fn child_closest_merkle_value(
        &self,
        _child_info: &ChildInfo,
        _key: &[u8],
    ) -> Result<
        Option<sp_trie::MerkleValue<<HashingFor<Block> as sp_core::Hasher>::Out>>,
        Self::Error,
    > {
        unimplemented!("child_closest_merkle_value: unsupported feature for lazy loading")
    }

    fn child_storage(
        &self,
        child_info: &ChildInfo,
        key: &[u8],
    ) -> Result<Option<StorageValue>, Self::Error> {
        let remote_fetch = |block: Option<Block::Hash>| -> Option<Vec<u8>> {
            self.rpc()
                .and_then(|rpc| rpc.child_storage(child_info, StorageKey(key.to_vec()), block).ok())
                .flatten()
                .map(|v| v.0)
        };

        // When before_fork, try RPC first, then fall back to local DB
        if self.before_fork {
            if self.rpc().is_some() {
                return Ok(remote_fetch(self.block_hash));
            } else {
                // No RPC client, try to read from local DB
                let readable_db = self.db.read();
                return Ok(readable_db.child_storage(child_info, key).ok().flatten());
            }
        }

        let readable_db = self.db.read();
        let maybe_storage = readable_db.child_storage(child_info, key);

        match maybe_storage {
            Ok(Some(value)) => Ok(Some(value)),
            Ok(None) => {
                let composite_key = make_composite_child_key(child_info.storage_key(), key);
                if self.removed_keys.read().contains(&composite_key) {
                    return Ok(None);
                }

                if let Some(remote_value) = remote_fetch(self.fork_block) {
                    drop(readable_db);
                    self.update_child_storage(child_info, key, &Some(remote_value.clone()));
                    Ok(Some(remote_value))
                } else {
                    Ok(None)
                }
            }
            Err(e) => Err(e),
        }
    }

    fn child_storage_hash(
        &self,
        child_info: &ChildInfo,
        key: &[u8],
    ) -> Result<Option<<HashingFor<Block> as sp_core::Hasher>::Out>, Self::Error> {
        let remote_fetch = |block: Option<Block::Hash>| -> Option<Block::Hash> {
            self.rpc()
                .and_then(|rpc| {
                    rpc.child_storage_hash(child_info, StorageKey(key.to_vec()), block).ok()
                })
                .flatten()
        };

        // When before_fork, try RPC first, then fall back to local DB
        if self.before_fork {
            if self.rpc().is_some() {
                return Ok(remote_fetch(self.block_hash));
            } else {
                let readable_db = self.db.read();
                return Ok(readable_db.child_storage_hash(child_info, key).ok().flatten());
            }
        }

        let readable_db = self.db.read();
        let maybe_hash = readable_db.child_storage_hash(child_info, key);

        match maybe_hash {
            Ok(Some(hash)) => Ok(Some(hash)),
            Ok(None) => {
                let composite_key = make_composite_child_key(child_info.storage_key(), key);
                if self.removed_keys.read().contains(&composite_key) {
                    return Ok(None);
                }

                Ok(remote_fetch(self.fork_block))
            }
            Err(e) => Err(e),
        }
    }

    fn next_storage_key(
        &self,
        key: &[u8],
    ) -> Result<Option<sp_state_machine::StorageKey>, Self::Error> {
        let remote_fetch = |block: Option<Block::Hash>| {
            let start_key = Some(StorageKey(key.to_vec()));
            self.rpc()
                .and_then(|rpc| rpc.storage_keys_paged(start_key.clone(), 2, None, block).ok())
                .and_then(|keys| keys.last().cloned())
        };

        // When before_fork, try RPC first, then fall back to local DB
        let maybe_next_key = if self.before_fork {
            if self.rpc().is_some() {
                remote_fetch(self.block_hash)
            } else {
                self.db.read().next_storage_key(key).ok().flatten()
            }
        } else {
            let next_storage_key = self.db.read().next_storage_key(key);
            match next_storage_key {
                Ok(Some(next_key)) => Some(next_key),
                _ if !self.removed_keys.read().contains(key) => {
                    if self.rpc().is_some() {
                        remote_fetch(self.fork_block)
                    } else {
                        None
                    }
                }
                // Otherwise, there's no next key
                _ => None,
            }
        }
        .filter(|next_key| next_key != key);

        Ok(maybe_next_key)
    }

    fn next_child_storage_key(
        &self,
        child_info: &ChildInfo,
        key: &[u8],
    ) -> Result<Option<sp_state_machine::StorageKey>, Self::Error> {
        let remote_fetch = |block: Option<Block::Hash>| {
            let start_key = Some(StorageKey(key.to_vec()));
            self.rpc()
                .and_then(|rpc| {
                    rpc.child_storage_keys_paged(child_info, None, 2, start_key.clone(), block).ok()
                })
                .and_then(|keys| keys.last().cloned())
        };

        // When before_fork, try RPC first, then fall back to local DB
        let maybe_next_key = if self.before_fork {
            if self.rpc().is_some() {
                remote_fetch(self.block_hash)
            } else {
                self.db.read().next_child_storage_key(child_info, key).ok().flatten()
            }
        } else {
            let next_child_key = self.db.read().next_child_storage_key(child_info, key);
            match next_child_key {
                Ok(Some(next_key)) => Some(next_key),
                // Otherwise, check removed_keys and try remote fetch if not removed
                _ => {
                    let composite_key = make_composite_child_key(child_info.storage_key(), key);
                    if !self.removed_keys.read().contains(&composite_key) {
                        if self.rpc().is_some() { remote_fetch(self.fork_block) } else { None }
                    } else {
                        None
                    }
                }
            }
        }
        .filter(|next_key| next_key != key);

        Ok(maybe_next_key)
    }

    fn storage_root<'a>(
        &self,
        delta: impl Iterator<Item = (&'a [u8], Option<&'a [u8]>)>,
        state_version: StateVersion,
    ) -> (<HashingFor<Block> as sp_core::Hasher>::Out, BackendTransaction<HashingFor<Block>>)
    where
        <HashingFor<Block> as sp_core::Hasher>::Out: Ord,
    {
        self.db.read().storage_root(delta, state_version)
    }

    fn child_storage_root<'a>(
        &self,
        child_info: &ChildInfo,
        delta: impl Iterator<Item = (&'a [u8], Option<&'a [u8]>)>,
        state_version: StateVersion,
    ) -> (<HashingFor<Block> as sp_core::Hasher>::Out, bool, BackendTransaction<HashingFor<Block>>)
    where
        <HashingFor<Block> as sp_core::Hasher>::Out: Ord,
    {
        self.db.read().child_storage_root(child_info, delta, state_version)
    }

    fn raw_iter(&self, args: IterArgs<'_>) -> Result<Self::RawIter, Self::Error> {
        let inner = self.db.read().raw_iter(args)?;
        Ok(RawIter { inner })
    }

    fn register_overlay_stats(&self, stats: &sp_state_machine::StateMachineStats) {
        self.db.read().register_overlay_stats(stats)
    }

    fn usage_info(&self) -> sp_state_machine::UsageInfo {
        self.db.read().usage_info()
    }
}

impl<B: BlockT + DeserializeOwned> AsTrieBackend<HashingFor<B>> for ForkedLazyBackend<B> {
    type TrieBackendStorage = PrefixedMemoryDB<HashingFor<B>>;

    fn as_trie_backend(
        &self,
    ) -> &sp_state_machine::TrieBackend<Self::TrieBackendStorage, HashingFor<B>> {
        unimplemented!("`as_trie_backend` is not supported in lazy loading mode.")
    }
}
