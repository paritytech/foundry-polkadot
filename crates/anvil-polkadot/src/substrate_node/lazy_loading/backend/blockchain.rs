use crate::substrate_node::lazy_loading::{LAZY_LOADING_LOG_TARGET, rpc_client::RPCClient};
use parking_lot::RwLock;
use polkadot_sdk::{
    sc_client_api::{
        backend::{self, NewBlockState},
        leaves::LeafSet,
    },
    sp_blockchain::{self, BlockStatus, CachedHeaderMetadata, HeaderBackend, HeaderMetadata},
    sp_runtime::{
        Justification, Justifications,
        generic::BlockId,
        traits::{Block as BlockT, Header as HeaderT, NumberFor, Zero},
    },
};
use serde::de::DeserializeOwned;
use std::{collections::HashMap, sync::Arc};

#[derive(PartialEq, Eq, Clone)]
pub(crate) enum StoredBlock<B: BlockT> {
    Header(B::Header, Option<Justifications>),
    Full(B, Option<Justifications>),
}

impl<B: BlockT> StoredBlock<B> {
    pub(crate) fn new(
        header: B::Header,
        body: Option<Vec<B::Extrinsic>>,
        just: Option<Justifications>,
    ) -> Self {
        match body {
            Some(body) => Self::Full(B::new(header, body), just),
            None => Self::Header(header, just),
        }
    }

    pub(crate) fn header(&self) -> &B::Header {
        match *self {
            Self::Header(ref h, _) => h,
            Self::Full(ref b, _) => b.header(),
        }
    }

    pub(crate) fn justifications(&self) -> Option<&Justifications> {
        match *self {
            Self::Header(_, ref j) | Self::Full(_, ref j) => j.as_ref(),
        }
    }

    pub(crate) fn extrinsics(&self) -> Option<&[B::Extrinsic]> {
        match *self {
            Self::Header(_, _) => None,
            Self::Full(ref b, _) => Some(b.extrinsics()),
        }
    }

    pub(crate) fn into_inner(
        self,
    ) -> (B::Header, Option<Vec<B::Extrinsic>>, Option<Justifications>) {
        match self {
            Self::Header(header, just) => (header, None, just),
            Self::Full(block, just) => {
                let (header, body) = block.deconstruct();
                (header, Some(body), just)
            }
        }
    }
}

#[derive(Clone)]
pub(crate) struct BlockchainStorage<Block: BlockT> {
    pub(crate) blocks: HashMap<Block::Hash, StoredBlock<Block>>,
    pub(crate) hashes: HashMap<NumberFor<Block>, Block::Hash>,
    pub(crate) best_hash: Block::Hash,
    pub(crate) best_number: NumberFor<Block>,
    pub(crate) finalized_hash: Block::Hash,
    pub(crate) finalized_number: NumberFor<Block>,
    pub(crate) genesis_hash: Block::Hash,
    pub(crate) leaves: LeafSet<Block::Hash, NumberFor<Block>>,
    pub(crate) aux: HashMap<Vec<u8>, Vec<u8>>,
}

/// In-memory blockchain. Supports concurrent reads.
#[derive(Clone)]
pub struct Blockchain<Block: BlockT + DeserializeOwned> {
    rpc_client: Option<Arc<dyn RPCClient<Block>>>,
    pub(crate) storage: Arc<RwLock<BlockchainStorage<Block>>>,
}

impl<Block: BlockT + DeserializeOwned> Blockchain<Block> {
    /// Create new in-memory blockchain storage.
    pub(crate) fn new(rpc_client: Option<Arc<dyn RPCClient<Block>>>) -> Self {
        let storage = Arc::new(RwLock::new(BlockchainStorage {
            blocks: HashMap::new(),
            hashes: HashMap::new(),
            best_hash: Default::default(),
            best_number: Zero::zero(),
            finalized_hash: Default::default(),
            finalized_number: Zero::zero(),
            genesis_hash: Default::default(),
            leaves: LeafSet::new(),
            aux: HashMap::new(),
        }));
        Self { rpc_client, storage }
    }

    #[inline]
    fn rpc(&self) -> Option<&dyn RPCClient<Block>> {
        self.rpc_client.as_deref()
    }

    /// Get header hash of given block.
    pub fn id(&self, id: BlockId<Block>) -> Option<Block::Hash> {
        match id {
            BlockId::Hash(h) => Some(h),
            BlockId::Number(n) => {
                let block_hash = self.storage.read().hashes.get(&n).copied();

                match block_hash {
                    None => {
                        let block_hash =
                            self.rpc().and_then(|rpc| rpc.block_hash(Some(n)).ok().flatten());
                        if let Some(h) = block_hash {
                            self.storage.write().hashes.insert(n, h);
                        }
                        block_hash
                    }
                    block_hash => block_hash,
                }
            }
        }
    }

    /// Insert a block header and associated data.
    pub fn insert(
        &self,
        hash: Block::Hash,
        header: <Block as BlockT>::Header,
        justifications: Option<Justifications>,
        body: Option<Vec<<Block as BlockT>::Extrinsic>>,
        new_state: NewBlockState,
    ) -> sp_blockchain::Result<()> {
        let number = *header.number();

        if new_state.is_best() {
            self.apply_head(&header)?;
        }

        let mut storage = self.storage.write();

        // Always insert the block into blocks and hashes storage
        storage.blocks.insert(hash, StoredBlock::new(header.clone(), body, justifications));
        storage.hashes.insert(number, hash);

        // Set genesis_hash only for the first block inserted
        if storage.blocks.len() == 1 {
            storage.genesis_hash = hash;
        }

        // Update leaves for all blocks including genesis.
        // For genesis when forking, the parent_hash points to the previous block on the remote
        // chain. That parent won't be in our leaf set, so this effectively adds genesis as
        // a new leaf.
        storage.leaves.import(hash, number, *header.parent_hash());

        // Finalize block only if explicitly requested via new_state
        if let NewBlockState::Final = new_state {
            storage.finalized_hash = hash;
            storage.finalized_number = number;
        }

        Ok(())
    }

    /// Set an existing block as head.
    pub fn set_head(&self, hash: Block::Hash) -> sp_blockchain::Result<()> {
        let header = self
            .header(hash)?
            .ok_or_else(|| sp_blockchain::Error::UnknownBlock(format!("{hash:?}")))?;

        self.apply_head(&header)
    }

    fn apply_head(&self, header: &<Block as BlockT>::Header) -> sp_blockchain::Result<()> {
        let mut storage = self.storage.write();

        let hash = header.hash();
        let number = header.number();

        storage.best_hash = hash;
        storage.best_number = *number;
        storage.hashes.insert(*number, hash);

        Ok(())
    }

    pub(crate) fn finalize_header(
        &self,
        block: Block::Hash,
        justification: Option<Justification>,
    ) -> sp_blockchain::Result<()> {
        let mut storage = self.storage.write();
        storage.finalized_hash = block;

        if justification.is_some() {
            let block = storage
                .blocks
                .get_mut(&block)
                .expect("hash was fetched from a block in the db; qed");

            let block_justifications = match block {
                StoredBlock::Header(_, j) | StoredBlock::Full(_, j) => j,
            };

            *block_justifications = justification.map(Justifications::from);
        }

        Ok(())
    }

    pub(crate) fn append_justification(
        &self,
        hash: Block::Hash,
        justification: Justification,
    ) -> sp_blockchain::Result<()> {
        let mut storage = self.storage.write();

        let block =
            storage.blocks.get_mut(&hash).expect("hash was fetched from a block in the db; qed");

        let block_justifications = match block {
            StoredBlock::Header(_, j) | StoredBlock::Full(_, j) => j,
        };

        if let Some(stored_justifications) = block_justifications {
            if !stored_justifications.append(justification) {
                return Err(sp_blockchain::Error::BadJustification(
                    "Duplicate consensus engine ID".into(),
                ));
            }
        } else {
            *block_justifications = Some(Justifications::from(justification));
        };

        Ok(())
    }

    pub(crate) fn write_aux(&self, ops: Vec<(Vec<u8>, Option<Vec<u8>>)>) {
        let mut storage = self.storage.write();
        for (k, v) in ops {
            match v {
                Some(v) => storage.aux.insert(k, v),
                None => storage.aux.remove(&k),
            };
        }
    }
}

impl<Block: BlockT + DeserializeOwned> HeaderBackend<Block> for Blockchain<Block> {
    fn header(
        &self,
        hash: Block::Hash,
    ) -> sp_blockchain::Result<Option<<Block as BlockT>::Header>> {
        // First, try to get the header from local storage
        if let Some(header) = self.storage.read().blocks.get(&hash).map(|b| b.header().clone()) {
            return Ok(Some(header));
        }

        // If not found in local storage, fetch from RPC client
        let header = if let Some(rpc) = self.rpc() {
            match rpc.block(Some(hash)) {
                Ok(Some(full)) => {
                    let block = full.block.clone();
                    self.storage
                        .write()
                        .blocks
                        .insert(hash, StoredBlock::Full(block.clone(), full.justifications));
                    Some(block.header().clone())
                }
                Ok(None) => {
                    // Block not found on remote chain - this is expected for locally-built blocks
                    tracing::debug!(
                        target: LAZY_LOADING_LOG_TARGET,
                        "Block {:?} not found in local storage or remote RPC",
                        hash
                    );
                    None
                }
                Err(e) => {
                    tracing::warn!(
                        target: LAZY_LOADING_LOG_TARGET,
                        "Failed to fetch block {:?} from RPC: {}",
                        hash,
                        e
                    );
                    None
                }
            }
        } else {
            // No RPC configured - block simply doesn't exist locally
            tracing::debug!(
                target: LAZY_LOADING_LOG_TARGET,
                "Block {:?} not found in local storage (no RPC configured)",
                hash
            );
            None
        };

        Ok(header)
    }

    fn info(&self) -> sp_blockchain::Info<Block> {
        let storage = self.storage.read();
        let finalized_state = if storage.blocks.len() <= 1 {
            None
        } else {
            Some((storage.finalized_hash, storage.finalized_number))
        };

        sp_blockchain::Info {
            best_hash: storage.best_hash,
            best_number: storage.best_number,
            genesis_hash: storage.genesis_hash,
            finalized_hash: storage.finalized_hash,
            finalized_number: storage.finalized_number,
            finalized_state,
            number_leaves: storage.leaves.count(),
            block_gap: None,
        }
    }

    fn status(&self, hash: Block::Hash) -> sp_blockchain::Result<BlockStatus> {
        // Check local storage first
        if self.storage.read().blocks.contains_key(&hash) {
            return Ok(BlockStatus::InChain);
        }

        // If not in local storage, check RPC
        if let Some(rpc) = self.rpc() {
            match rpc.header(Some(hash)) {
                Ok(Some(_)) => Ok(BlockStatus::InChain),
                Ok(None) => Ok(BlockStatus::Unknown),
                Err(_) => Ok(BlockStatus::Unknown),
            }
        } else {
            Ok(BlockStatus::Unknown)
        }
    }

    fn number(&self, hash: Block::Hash) -> sp_blockchain::Result<Option<NumberFor<Block>>> {
        if let Some(b) = self.storage.read().blocks.get(&hash) {
            return Ok(Some(*b.header().number()));
        }
        match self.rpc() {
            Some(rpc) => match rpc.block(Some(hash)) {
                Ok(Some(block)) => Ok(Some(*block.block.header().number())),
                err => Err(sp_blockchain::Error::UnknownBlock(format!(
                    "Failed to fetch block number from RPC: {err:?}"
                ))),
            },
            None => Err(sp_blockchain::Error::UnknownBlock(
                "RPC not configured to resolve block number".into(),
            )),
        }
    }

    fn hash(
        &self,
        number: <<Block as BlockT>::Header as HeaderT>::Number,
    ) -> sp_blockchain::Result<Option<Block::Hash>> {
        Ok(self.id(BlockId::Number(number)))
    }
}

impl<Block: BlockT + DeserializeOwned> HeaderMetadata<Block> for Blockchain<Block> {
    type Error = sp_blockchain::Error;

    fn header_metadata(
        &self,
        hash: Block::Hash,
    ) -> Result<CachedHeaderMetadata<Block>, Self::Error> {
        self.header(hash)?.map(|header| CachedHeaderMetadata::from(&header)).ok_or_else(|| {
            sp_blockchain::Error::UnknownBlock(format!("header not found: {hash:?}"))
        })
    }

    fn insert_header_metadata(&self, _hash: Block::Hash, _metadata: CachedHeaderMetadata<Block>) {
        // No need to implement.
        unimplemented!("insert_header_metadata")
    }
    fn remove_header_metadata(&self, _hash: Block::Hash) {
        // No need to implement.
        unimplemented!("remove_header_metadata")
    }
}

impl<Block: BlockT + DeserializeOwned> sp_blockchain::Backend<Block> for Blockchain<Block> {
    fn body(
        &self,
        hash: Block::Hash,
    ) -> sp_blockchain::Result<Option<Vec<<Block as BlockT>::Extrinsic>>> {
        if let Some(xs) =
            self.storage.read().blocks.get(&hash).and_then(|b| b.extrinsics().map(|x| x.to_vec()))
        {
            return Ok(Some(xs));
        }
        let extrinsics = self.rpc().and_then(|rpc| {
            rpc.block(Some(hash)).ok().flatten().map(|b| b.block.extrinsics().to_vec())
        });
        Ok(extrinsics)
    }

    fn justifications(&self, hash: Block::Hash) -> sp_blockchain::Result<Option<Justifications>> {
        // Check local storage first
        if let Some(justifications) =
            self.storage.read().blocks.get(&hash).and_then(|b| b.justifications().cloned())
        {
            return Ok(Some(justifications));
        }

        // If not in local storage, fetch from RPC
        let justifications = self.rpc().and_then(|rpc| {
            rpc.block(Some(hash)).ok().flatten().and_then(|full| full.justifications)
        });

        Ok(justifications)
    }

    fn last_finalized(&self) -> sp_blockchain::Result<Block::Hash> {
        let last_finalized = self.storage.read().finalized_hash;

        Ok(last_finalized)
    }

    fn leaves(&self) -> sp_blockchain::Result<Vec<Block::Hash>> {
        let leaves = self.storage.read().leaves.hashes();

        Ok(leaves)
    }

    fn children(&self, parent_hash: Block::Hash) -> sp_blockchain::Result<Vec<Block::Hash>> {
        // Find all blocks whose parent_hash matches the given hash
        let storage = self.storage.read();
        let children: Vec<Block::Hash> = storage
            .blocks
            .iter()
            .filter_map(|(hash, block)| {
                if *block.header().parent_hash() == parent_hash { Some(*hash) } else { None }
            })
            .collect();
        Ok(children)
    }

    fn indexed_transaction(&self, _hash: Block::Hash) -> sp_blockchain::Result<Option<Vec<u8>>> {
        // Indexed transactions are not supported in the lazy-loading backend
        Ok(None)
    }

    fn block_indexed_body(
        &self,
        _hash: Block::Hash,
    ) -> sp_blockchain::Result<Option<Vec<Vec<u8>>>> {
        // Indexed block bodies are not supported in the lazy-loading backend
        Ok(None)
    }
}

impl<Block: BlockT + DeserializeOwned> backend::AuxStore for Blockchain<Block> {
    fn insert_aux<
        'a,
        'b: 'a,
        'c: 'a,
        I: IntoIterator<Item = &'a (&'c [u8], &'c [u8])>,
        D: IntoIterator<Item = &'a &'b [u8]>,
    >(
        &self,
        insert: I,
        delete: D,
    ) -> sp_blockchain::Result<()> {
        let mut storage = self.storage.write();
        for (k, v) in insert {
            storage.aux.insert(k.to_vec(), v.to_vec());
        }
        for k in delete {
            storage.aux.remove(*k);
        }
        Ok(())
    }

    fn get_aux(&self, key: &[u8]) -> sp_blockchain::Result<Option<Vec<u8>>> {
        Ok(self.storage.read().aux.get(key).cloned())
    }
}
