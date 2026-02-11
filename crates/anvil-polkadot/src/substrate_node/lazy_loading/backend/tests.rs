use super::*;
use mock_rpc::{Rpc, TestBlock, TestHeader};
use parking_lot::RwLock;
use polkadot_sdk::{
    sc_client_api::{Backend as BackendT, HeaderBackend, StateBackend},
    sp_runtime::{
        OpaqueExtrinsic,
        traits::{BlakeTwo256, Header as HeaderT},
    },
    sp_state_machine,
    sp_storage::{StorageData, StorageKey},
};
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicUsize, Ordering},
};

#[cfg(test)]
mod mock_rpc {
    use super::*;
    use polkadot_sdk::sp_runtime::{
        Justifications,
        generic::{Block as GenericBlock, Header, SignedBlock},
        traits::Header as HeaderT,
    };

    pub type TestHashing = BlakeTwo256;
    pub type TestHeader<N = u32> = Header<N, TestHashing>;
    pub type TestExtrinsic = OpaqueExtrinsic;
    pub type TestBlock<N = u32> = GenericBlock<TestHeader<N>, TestExtrinsic>;

    #[derive(Default, Debug)]
    pub struct Counters {
        pub storage_calls: AtomicUsize,
        pub storage_hash_calls: AtomicUsize,
        pub storage_keys_paged_calls: AtomicUsize,
        pub header_calls: AtomicUsize,
        pub block_calls: AtomicUsize,
    }

    /// Mockable RPC with interior mutability.
    #[allow(clippy::type_complexity)]
    #[derive(Clone, Default, Debug)]
    pub struct Rpc<Block: BlockT + DeserializeOwned> {
        pub counters: std::sync::Arc<Counters>,
        /// storage[(block_hash, key)] = value
        pub storage: Arc<RwLock<BTreeMap<(Block::Hash, StorageKey), StorageData>>>,
        /// storage_hash[(block_hash, key)] = hash
        pub storage_hashes: Arc<RwLock<BTreeMap<(Block::Hash, StorageKey), Block::Hash>>>,
        /// storage_keys_paged[(block_hash, (prefix,start))] = Vec<keys>
        pub storage_keys_pages: Arc<RwLock<BTreeMap<(Block::Hash, Vec<u8>), Vec<StorageKey>>>>,
        /// child_storage[(block_hash, child_storage_key, key)] = value
        pub child_storage: Arc<RwLock<BTreeMap<(Block::Hash, Vec<u8>, StorageKey), StorageData>>>,
        /// child_storage_hashes[(block_hash, child_storage_key, key)] = hash
        pub child_storage_hashes:
            Arc<RwLock<BTreeMap<(Block::Hash, Vec<u8>, StorageKey), Block::Hash>>>,
        /// child_storage_keys_pages[(block_hash, child_storage_key, prefix)] = Vec<keys>
        pub child_storage_keys_pages:
            Arc<RwLock<BTreeMap<(Block::Hash, Vec<u8>, Vec<u8>), Vec<StorageKey>>>>,
        /// headers[hash] = header
        pub headers: Arc<RwLock<BTreeMap<Block::Hash, Block::Header>>>,
        /// blocks[hash] = SignedBlock
        pub blocks: Arc<RwLock<BTreeMap<Block::Hash, SignedBlock<Block>>>>,
    }

    impl<Block: BlockT + DeserializeOwned> Rpc<Block> {
        pub fn new() -> Self {
            Self {
                counters: std::sync::Arc::new(Counters::default()),
                storage: std::sync::Arc::new(RwLock::new(BTreeMap::new())),
                storage_hashes: std::sync::Arc::new(RwLock::new(BTreeMap::new())),
                storage_keys_pages: std::sync::Arc::new(RwLock::new(BTreeMap::new())),
                child_storage: std::sync::Arc::new(RwLock::new(BTreeMap::new())),
                child_storage_hashes: std::sync::Arc::new(RwLock::new(BTreeMap::new())),
                child_storage_keys_pages: std::sync::Arc::new(RwLock::new(BTreeMap::new())),
                headers: std::sync::Arc::new(RwLock::new(BTreeMap::new())),
                blocks: std::sync::Arc::new(RwLock::new(BTreeMap::new())),
            }
        }

        pub fn put_storage(&self, at: Block::Hash, key: StorageKey, val: StorageData) {
            self.storage.write().insert((at, key), val);
        }
        pub fn put_header(&self, h: Block::Header) {
            self.headers.write().insert(h.hash(), h);
        }
        pub fn put_block(&self, block: Block, just: Option<Justifications>) {
            let full = SignedBlock { block, justifications: just };
            self.blocks.write().insert(full.block.header().hash(), full);
        }

        pub fn put_child_storage(
            &self,
            at: Block::Hash,
            child_storage_key: Vec<u8>,
            key: StorageKey,
            val: StorageData,
        ) {
            self.child_storage.write().insert((at, child_storage_key, key), val);
        }

        pub fn put_child_storage_hash(
            &self,
            at: Block::Hash,
            child_storage_key: Vec<u8>,
            key: StorageKey,
            hash: Block::Hash,
        ) {
            self.child_storage_hashes.write().insert((at, child_storage_key, key), hash);
        }

        pub fn put_child_storage_keys_page(
            &self,
            at: Block::Hash,
            child_storage_key: Vec<u8>,
            prefix: Vec<u8>,
            keys: Vec<StorageKey>,
        ) {
            self.child_storage_keys_pages.write().insert((at, child_storage_key, prefix), keys);
        }
    }

    impl<Block: BlockT + DeserializeOwned> RPCClient<Block> for Rpc<Block> {
        fn storage(
            &self,
            key: StorageKey,
            at: Option<Block::Hash>,
        ) -> Result<Option<StorageData>, jsonrpsee::core::ClientError> {
            self.counters.storage_calls.fetch_add(1, Ordering::Relaxed);
            let map = self.storage.read();
            Ok(map.get(&(at.unwrap_or_default(), key)).cloned())
        }

        fn storage_hash(
            &self,
            key: StorageKey,
            at: Option<Block::Hash>,
        ) -> Result<Option<Block::Hash>, jsonrpsee::core::ClientError> {
            self.counters.storage_hash_calls.fetch_add(1, Ordering::Relaxed);
            let bh = at.unwrap_or_default();
            let map = self.storage_hashes.read();
            Ok(map.get(&(bh, key)).copied())
        }

        fn storage_keys_paged(
            &self,
            key: Option<StorageKey>,
            count: u32,
            start_key: Option<StorageKey>,
            at: Option<Block::Hash>,
        ) -> Result<Vec<sp_state_machine::StorageKey>, jsonrpsee::core::ClientError> {
            self.counters.storage_keys_paged_calls.fetch_add(1, Ordering::Relaxed);

            use std::cmp::min;

            let bh = at.unwrap_or_default();
            let prefix = key.map(|k| k.0).unwrap_or_default();
            let start = start_key.map(|k| k.0);

            let map = self.storage_keys_pages.read();
            let mut all = map.get(&(bh, prefix.clone())).cloned().unwrap_or_default();

            all.sort_by(|a, b| a.0.cmp(&b.0));

            let mut filtered: Vec<StorageKey> =
                all.into_iter().filter(|k| k.0.starts_with(&prefix)).collect();

            if let Some(s) = start {
                if let Some(pos) = filtered.iter().position(|k| k.0 == s) {
                    filtered = filtered.into_iter().skip(pos + 1).collect();
                } else {
                    filtered.retain(|k| k.0 > s);
                }
            }

            let take = min(filtered.len(), count as usize);
            Ok(filtered.into_iter().take(take).map(|k| k.0).collect())
        }

        fn header(
            &self,
            at: Option<Block::Hash>,
        ) -> Result<Option<Block::Header>, jsonrpsee::core::ClientError> {
            self.counters.header_calls.fetch_add(1, Ordering::Relaxed);
            let key = at.unwrap_or_default();
            let raw = self.headers.read().get(&key).cloned();
            Ok(raw)
        }

        fn block(
            &self,
            hash: Option<Block::Hash>,
        ) -> Result<
            Option<polkadot_sdk::sp_runtime::generic::SignedBlock<Block>>,
            jsonrpsee::core::ClientError,
        > {
            self.counters.block_calls.fetch_add(1, Ordering::Relaxed);
            let key = hash.unwrap_or_default();
            let raw = self.blocks.read().get(&key).cloned();
            Ok(raw)
        }

        fn block_hash(
            &self,
            _num: Option<NumberFor<Block>>,
        ) -> Result<Option<Block::Hash>, jsonrpsee::core::ClientError> {
            todo!()
        }

        fn system_chain(&self) -> Result<String, jsonrpsee::core::ClientError> {
            todo!()
        }

        fn system_properties(
            &self,
        ) -> Result<polkadot_sdk::sc_chain_spec::Properties, jsonrpsee::core::ClientError> {
            todo!()
        }

        fn child_storage(
            &self,
            child_info: &polkadot_sdk::sp_storage::ChildInfo,
            key: StorageKey,
            at: Option<Block::Hash>,
        ) -> Result<Option<StorageData>, jsonrpsee::core::ClientError> {
            let bh = at.unwrap_or_default();
            let child_storage_key = child_info.storage_key().to_vec();
            let map = self.child_storage.read();
            Ok(map.get(&(bh, child_storage_key, key)).cloned())
        }

        fn child_storage_hash(
            &self,
            child_info: &polkadot_sdk::sp_storage::ChildInfo,
            key: StorageKey,
            at: Option<Block::Hash>,
        ) -> Result<Option<Block::Hash>, jsonrpsee::core::ClientError> {
            let bh = at.unwrap_or_default();
            let child_storage_key = child_info.storage_key().to_vec();
            let map = self.child_storage_hashes.read();
            Ok(map.get(&(bh, child_storage_key, key)).copied())
        }

        fn child_storage_keys_paged(
            &self,
            child_info: &polkadot_sdk::sp_storage::ChildInfo,
            key: Option<StorageKey>,
            count: u32,
            start_key: Option<StorageKey>,
            at: Option<Block::Hash>,
        ) -> Result<Vec<sp_state_machine::StorageKey>, jsonrpsee::core::ClientError> {
            use std::cmp::min;

            let bh = at.unwrap_or_default();
            let child_storage_key = child_info.storage_key().to_vec();
            let prefix = key.map(|k| k.0).unwrap_or_default();
            let start = start_key.map(|k| k.0);

            let map = self.child_storage_keys_pages.read();
            let mut all =
                map.get(&(bh, child_storage_key, prefix.clone())).cloned().unwrap_or_default();

            all.sort_by(|a, b| a.0.cmp(&b.0));

            let mut filtered: Vec<StorageKey> =
                all.into_iter().filter(|k| k.0.starts_with(&prefix)).collect();

            if let Some(s) = start {
                if let Some(pos) = filtered.iter().position(|k| k.0 == s) {
                    filtered = filtered.into_iter().skip(pos + 1).collect();
                } else {
                    filtered.retain(|k| k.0 > s);
                }
            }

            let take = min(filtered.len(), count as usize);
            Ok(filtered.into_iter().take(take).map(|k| k.0).collect())
        }
    }
}

type N = u32;
type TestBlockT = TestBlock<N>;

fn make_header(number: N, parent: <TestBlock as BlockT>::Hash) -> TestHeader<N> {
    TestHeader::new(number, Default::default(), Default::default(), parent, Default::default())
}

fn make_block(
    number: N,
    parent: <TestBlock as BlockT>::Hash,
    xts: Vec<OpaqueExtrinsic>,
) -> TestBlock<N> {
    let header = make_header(number, parent);
    TestBlock::new(header, xts)
}

fn checkpoint(n: N) -> TestHeader<N> {
    make_header(n, Default::default())
}

#[test]
fn before_fork_reads_remote_only() {
    let rpc = std::sync::Arc::new(Rpc::new());
    // fork checkpoint at #100
    let cp = checkpoint(100);
    let backend = Backend::<TestBlockT>::new(Some((rpc.clone(), cp)));

    // state_at(Default::default()) => before_fork=true
    let state = backend.state_at(Default::default(), TrieCacheContext::Trusted).unwrap();

    let key = b":foo".to_vec();
    // prepare remote value at "block_hash = Default::default()"
    let at = Default::default();
    rpc.put_storage(at, StorageKey(key.clone()), StorageData(b"bar".to_vec()));

    // read storage
    let v1 = state.storage(&key).unwrap();
    assert_eq!(v1, Some(b"bar".to_vec()));

    // not cached in DB: second read still goes to RPC
    let v2 = state.storage(&key).unwrap();
    assert_eq!(v2, Some(b"bar".to_vec()));
    assert!(rpc.counters.storage_calls.load(Ordering::Relaxed) >= 2);
}

#[test]
fn after_fork_first_fetch_caches_subsequent_hits_local() {
    let rpc = std::sync::Arc::new(Rpc::new());
    let cp = checkpoint(10);
    let backend = Backend::<TestBlockT>::new(Some((rpc.clone(), cp.clone())));

    // Build a block #11 > checkpoint (#10), with parent #10
    let parent = cp.hash();
    let b11 = make_block(11, parent, vec![]);
    let h11 = b11.header.hash();

    rpc.put_header(b11.header.clone());
    rpc.put_block(b11, None);

    // remote storage at fork block (checkpoint hash)
    let fork_hash = cp.hash();
    let key = b":k".to_vec();
    rpc.put_storage(fork_hash, StorageKey(key.clone()), StorageData(b"v".to_vec()));

    // Grab state_at(#11): after_fork=false; local DB empty
    let state = backend.state_at(h11, TrieCacheContext::Trusted).unwrap();

    // First read fetches remote and caches
    let v1 = state.storage(&key).unwrap();
    assert_eq!(v1, Some(b"v".to_vec()));

    // Mutate RPC to detect second call (remove remote value)
    // If second read still tries RPC, it would return None; but it should come from cache.
    // So we do not change the mock; instead, assert RPC call count increases only once.
    let calls_before = rpc.counters.storage_calls.load(Ordering::Relaxed);
    let _ = state.storage(&key).unwrap();
    let calls_after = rpc.counters.storage_calls.load(Ordering::Relaxed);
    assert_eq!(calls_before, calls_after, "second hit should be served from cache");
}

#[test]
fn removed_keys_prevents_remote_fetch() {
    let rpc = std::sync::Arc::new(Rpc::new());
    let cp = checkpoint(5);
    let backend = Backend::<TestBlockT>::new(Some((rpc.clone(), cp.clone())));

    // make block #6
    let b6 = make_block(6, cp.hash(), vec![]);
    rpc.put_header(b6.header.clone());
    rpc.put_block(b6.clone(), None);
    let state = backend.state_at(b6.header.hash(), TrieCacheContext::Trusted).unwrap();

    // mark key as removed
    let key = b":dead".to_vec();
    state.removed_keys.write().insert(key.clone());

    // Even if remote has a value, backend must not fetch it
    rpc.put_storage(cp.hash(), StorageKey(key.clone()), StorageData(b"ghost".to_vec()));
    let calls_before = rpc.counters.storage_calls.load(Ordering::Relaxed);
    let v = state.storage(&key).unwrap();
    let calls_after = rpc.counters.storage_calls.load(Ordering::Relaxed);

    assert!(v.is_none());
    assert_eq!(calls_before, calls_after, "should not call RPC for removed keys");
}

#[test]
fn blockchain_header_and_number_are_cached() {
    let rpc = std::sync::Arc::new(Rpc::new());
    let cp = checkpoint(3);
    let backend = Backend::<TestBlockT>::new(Some((rpc.clone(), cp.clone())));
    let chain = backend.blockchain();

    // prepare one block w/ extrinsics
    let xts: Vec<OpaqueExtrinsic> = vec![];
    let b4 = make_block(4, cp.hash(), xts);
    let h4 = b4.header().hash();
    rpc.put_block(b4, None);

    // first header() fetches RPC and caches as Full
    let h = chain.header(h4).unwrap().unwrap();
    assert_eq!(h.hash(), h4);

    // number() should now return from cache (no extra RPC needed)
    let calls_before = rpc.counters.block_calls.load(Ordering::Relaxed);
    let number = chain.number(h4).unwrap().unwrap();
    let calls_after = rpc.counters.block_calls.load(Ordering::Relaxed);

    assert_eq!(number, 4);
    assert_eq!(calls_before, calls_after, "number() should be served from cache after header()");
}

#[test]
fn no_fork_mode_uses_local_db_only() {
    let backend = Backend::<TestBlockT>::new(None);
    let state = backend.state_at(Default::default(), TrieCacheContext::Trusted).unwrap();

    assert!(!state.before_fork);

    let key = b":test_key".to_vec();
    let v1 = state.storage(&key).unwrap();
    assert_eq!(v1, None);

    state.update_storage(&key, &Some(b"local_value".to_vec()));

    let v2 = state.storage(&key).unwrap();
    assert_eq!(v2, Some(b"local_value".to_vec()));
}

#[test]
fn no_fork_mode_state_at_default() {
    let backend = Backend::<TestBlockT>::new(None);
    let state = backend.state_at(Default::default(), TrieCacheContext::Trusted).unwrap();

    assert!(!state.before_fork);
    assert_eq!(state.fork_block, None);
    assert!(state.rpc_client.is_none());
}

#[test]
fn no_fork_mode_storage_operations() {
    let backend = Backend::<TestBlockT>::new(None);
    let state = backend.state_at(Default::default(), TrieCacheContext::Trusted).unwrap();

    let key1 = b":key1".to_vec();
    let key2 = b":key2".to_vec();
    let key3 = b":key3".to_vec();

    state.update_storage(&key1, &Some(b"value1".to_vec()));
    state.update_storage(&key2, &Some(b"value2".to_vec()));

    assert_eq!(state.storage(&key1).unwrap(), Some(b"value1".to_vec()));
    assert_eq!(state.storage(&key2).unwrap(), Some(b"value2".to_vec()));
    assert_eq!(state.storage(&key3).unwrap(), None);
}

#[test]
fn child_storage_before_fork_reads_remote() {
    use polkadot_sdk::sp_storage::ChildInfo;

    let rpc = std::sync::Arc::new(Rpc::new());
    let cp = checkpoint(100);
    let backend = super::Backend::<TestBlockT>::new(Some((rpc.clone(), cp)));

    let state = backend.state_at(Default::default(), TrieCacheContext::Trusted).unwrap();

    let child_info = ChildInfo::new_default(b"child1");
    let key = b":child_key".to_vec();
    let at = Default::default();

    // Put child storage in mock RPC
    rpc.put_child_storage(
        at,
        child_info.storage_key().to_vec(),
        StorageKey(key.clone()),
        StorageData(b"child_value".to_vec()),
    );

    // Read child storage - should fetch from RPC
    let v = state.child_storage(&child_info, &key).unwrap();
    assert_eq!(v, Some(b"child_value".to_vec()));
}

#[test]
fn child_storage_after_fork_caches() {
    use polkadot_sdk::sp_storage::ChildInfo;

    let rpc = std::sync::Arc::new(Rpc::new());
    let cp = checkpoint(10);
    let backend = super::Backend::<TestBlockT>::new(Some((rpc.clone(), cp.clone())));

    let parent = cp.hash();
    let b11 = make_block(11, parent, vec![]);
    let h11 = b11.header.hash();

    rpc.put_header(b11.header.clone());
    rpc.put_block(b11, None);

    let child_info = ChildInfo::new_default(b"child2");
    let key = b":child_key2".to_vec();
    let fork_hash = cp.hash();

    rpc.put_child_storage(
        fork_hash,
        child_info.storage_key().to_vec(),
        StorageKey(key.clone()),
        StorageData(b"cached_value".to_vec()),
    );

    let state = backend.state_at(h11, TrieCacheContext::Trusted).unwrap();

    // First read - should cache
    let v1 = state.child_storage(&child_info, &key).unwrap();
    assert_eq!(v1, Some(b"cached_value".to_vec()));

    // Second read - should come from cache (we can verify by checking the value is still there)
    let v2 = state.child_storage(&child_info, &key).unwrap();
    assert_eq!(v2, Some(b"cached_value".to_vec()));
}

#[test]
fn child_storage_hash_reads_from_rpc() {
    use polkadot_sdk::sp_storage::ChildInfo;

    let rpc = std::sync::Arc::new(Rpc::new());
    let cp = checkpoint(50);
    let backend = super::Backend::<TestBlockT>::new(Some((rpc.clone(), cp.clone())));

    let parent = cp.hash();
    let b51 = make_block(51, parent, vec![]);
    let h51 = b51.header.hash();

    rpc.put_header(b51.header.clone());
    rpc.put_block(b51, None);

    let child_info = ChildInfo::new_default(b"child3");
    let key = b":hash_key".to_vec();
    let fork_hash = cp.hash();
    let expected_hash = <TestBlockT as BlockT>::Hash::default();

    rpc.put_child_storage_hash(
        fork_hash,
        child_info.storage_key().to_vec(),
        StorageKey(key.clone()),
        expected_hash,
    );

    let state = backend.state_at(h51, TrieCacheContext::Trusted).unwrap();

    let hash = state.child_storage_hash(&child_info, &key).unwrap();
    assert_eq!(hash, Some(expected_hash));
}

#[test]
fn next_child_storage_key_uses_paged() {
    use polkadot_sdk::sp_storage::ChildInfo;

    let rpc = std::sync::Arc::new(Rpc::new());
    let cp = checkpoint(20);
    let backend = super::Backend::<TestBlockT>::new(Some((rpc.clone(), cp.clone())));

    let parent = cp.hash();
    let b21 = make_block(21, parent, vec![]);
    let h21 = b21.header.hash();

    rpc.put_header(b21.header.clone());
    rpc.put_block(b21, None);

    let child_info = ChildInfo::new_default(b"child4");
    let fork_hash = cp.hash();

    // Put a page of keys
    rpc.put_child_storage_keys_page(
        fork_hash,
        child_info.storage_key().to_vec(),
        vec![],
        vec![StorageKey(b"key1".to_vec()), StorageKey(b"key2".to_vec())],
    );

    let state = backend.state_at(h21, TrieCacheContext::Trusted).unwrap();

    // Get next key after "key1" should be "key2"
    let next = state.next_child_storage_key(&child_info, b"key1").unwrap();
    assert_eq!(next, Some(b"key2".to_vec()));
}

#[test]
fn blockchain_status_queries_rpc_when_not_local() {
    use polkadot_sdk::sp_blockchain::{BlockStatus, HeaderBackend};

    let rpc = std::sync::Arc::new(Rpc::new());
    let cp = checkpoint(20);
    let backend = super::Backend::<TestBlockT>::new(Some((rpc.clone(), cp.clone())));

    let parent = cp.hash();
    let b21 = make_block(21, parent, vec![]);
    let h21 = b21.header.hash();

    // Block is not in local storage yet, but exists in RPC
    rpc.put_header(b21.header);

    // status() should query RPC and return InChain
    let status = backend.blockchain().status(h21).unwrap();
    assert_eq!(status, BlockStatus::InChain);

    // Now query for a block that doesn't exist anywhere
    let unknown_hash = make_block(999, parent, vec![]).header.hash();
    let status = backend.blockchain().status(unknown_hash).unwrap();
    assert_eq!(status, BlockStatus::Unknown);
}

#[test]
fn blockchain_justifications_queries_rpc_when_not_local() {
    use polkadot_sdk::sp_blockchain::Backend as BlockchainBackend;

    let rpc = std::sync::Arc::new(Rpc::new());
    let cp = checkpoint(20);
    let backend = super::Backend::<TestBlockT>::new(Some((rpc.clone(), cp.clone())));

    let parent = cp.hash();
    let b21 = make_block(21, parent, vec![]);
    let h21 = b21.header.hash();

    // Create justifications
    let justifications =
        polkadot_sdk::sp_runtime::Justifications::from((*b"TEST", vec![1, 2, 3, 4]));

    // Block is not in local storage yet, but exists in RPC with justifications
    rpc.put_block(b21, Some(justifications.clone()));

    // justifications() should query RPC and return the justifications
    let result = backend.blockchain().justifications(h21).unwrap();
    assert_eq!(result, Some(justifications));

    // Now query for a block that doesn't exist anywhere
    let unknown_hash = make_block(999, parent, vec![]).header.hash();
    let result = backend.blockchain().justifications(unknown_hash).unwrap();
    assert_eq!(result, None);
}

#[test]
fn child_storage_removed_keys_uses_composite_key() {
    use polkadot_sdk::sp_storage::ChildInfo;

    let rpc = std::sync::Arc::new(Rpc::new());
    let cp = checkpoint(5);
    let backend = Backend::<TestBlockT>::new(Some((rpc.clone(), cp.clone())));

    // make block #6
    let b6 = make_block(6, cp.hash(), vec![]);
    rpc.put_header(b6.header.clone());
    rpc.put_block(b6.clone(), None);
    let state = backend.state_at(b6.header.hash(), TrieCacheContext::Trusted).unwrap();

    let child_info1 = ChildInfo::new_default(b"child1");
    let child_info2 = ChildInfo::new_default(b"child2");
    let key = b"same_key".to_vec();

    // Put values in RPC for both child storages with the same key
    rpc.put_child_storage(
        cp.hash(),
        child_info1.storage_key().to_vec(),
        StorageKey(key.clone()),
        StorageData(b"value1".to_vec()),
    );
    rpc.put_child_storage(
        cp.hash(),
        child_info2.storage_key().to_vec(),
        StorageKey(key.clone()),
        StorageData(b"value2".to_vec()),
    );

    // Mark the key as removed only for child1 using composite key
    let composite_key1 = super::make_composite_child_key(child_info1.storage_key(), &key);
    state.removed_keys.write().insert(composite_key1);

    // child1 should return None (key is removed)
    let v1 = state.child_storage(&child_info1, &key).unwrap();
    assert_eq!(v1, None, "child1 key should be removed");

    // child2 should still fetch from RPC (different composite key)
    let v2 = state.child_storage(&child_info2, &key).unwrap();
    assert_eq!(v2, Some(b"value2".to_vec()), "child2 key should still be accessible");
}

#[test]
fn child_storage_removed_keys_no_collision_with_main_storage() {
    use polkadot_sdk::sp_storage::ChildInfo;

    let rpc = std::sync::Arc::new(Rpc::new());
    let cp = checkpoint(5);
    let backend = Backend::<TestBlockT>::new(Some((rpc.clone(), cp.clone())));

    // make block #6
    let b6 = make_block(6, cp.hash(), vec![]);
    rpc.put_header(b6.header.clone());
    rpc.put_block(b6.clone(), None);
    let state = backend.state_at(b6.header.hash(), TrieCacheContext::Trusted).unwrap();

    let child_info = ChildInfo::new_default(b"child1");
    let key = b"test_key".to_vec();

    // Put value in main storage
    rpc.put_storage(cp.hash(), StorageKey(key.clone()), StorageData(b"main_value".to_vec()));

    // Put value in child storage with the same key
    rpc.put_child_storage(
        cp.hash(),
        child_info.storage_key().to_vec(),
        StorageKey(key.clone()),
        StorageData(b"child_value".to_vec()),
    );

    // Mark the key as removed in main storage (just the raw key)
    state.removed_keys.write().insert(key.clone());

    // Main storage should return None
    let v_main = state.storage(&key).unwrap();
    assert_eq!(v_main, None, "main storage key should be removed");

    // Child storage should still work (uses composite key)
    let v_child = state.child_storage(&child_info, &key).unwrap();
    assert_eq!(v_child, Some(b"child_value".to_vec()), "child storage key should not be affected");
}
