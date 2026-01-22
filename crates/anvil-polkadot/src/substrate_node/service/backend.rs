use crate::substrate_node::{
    lazy_loading::backend::Blockchain,
    service::{
        Backend,
        storage::{CodeInfo, ReviveAccountInfo, SystemAccountInfo, well_known_keys},
    },
};
use alloy_primitives::{Address, Bytes};
use codec::{Decode, Encode};
use lru::LruCache;
use parking_lot::Mutex;
use polkadot_sdk::{
    parachains_common::{AccountId, Hash, opaque::Block},
    sc_client_api::{Backend as BackendT, StateBackend, TrieCacheContext},
    sp_blockchain,
    sp_consensus_slots::Slot,
    sp_core::{H160, H256},
    sp_io::hashing::blake2_256,
    sp_runtime::FixedU128,
    sp_state_machine::{StorageKey, StorageValue},
};
use std::{collections::HashMap, num::NonZeroUsize, sync::Arc};
use substrate_runtime::Balance;

const DEFAULT_LRU_CAP: NonZeroUsize = NonZeroUsize::new(256).expect("256 is non-zero");

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("Inner client error: {0}")]
    Client(#[from] sp_blockchain::Error),
    #[error("Could not find total issuance in the state")]
    MissingTotalIssuance,
    #[error("Could not find chain id in the state")]
    MissingChainId,
    #[error("Could not find aura authorities in the state")]
    MissingAuraAuthorities,
    #[error("Could not find timestamp in the state")]
    MissingTimestamp,
    #[error("Could not find the next fee multiplier in the state")]
    MissingNextFeeMultiplier,
    #[error("Could not find block number in the state")]
    MissingBlockNumber,
    #[error("Could not find relay slot info in the state")]
    MissingRelaySlotInfo,
    #[error("Could not find last relay block number in the state")]
    MissingLastRelayBlockNumber,
    #[error("Could not find aura current slot in the state")]
    MissingAuraCurrentSlot,
    #[error("Unable to decode total issuance {0}")]
    DecodeTotalIssuance(codec::Error),
    #[error("Unable to decode chain id {0}")]
    DecodeChainId(codec::Error),
    #[error("Unable to decode balance {0}")]
    DecodeBalance(codec::Error),
    #[error("Unable to decode revive account info {0}")]
    DecodeReviveAccountInfo(codec::Error),
    #[error("Unable to decode system account info {0}")]
    DecodeSystemAccountInfo(codec::Error),
    #[error("Unable to decode revive code info {0}")]
    DecodeCodeInfo(codec::Error),
    #[error("Unable to decode timestamp: {0}")]
    DecodeTimestamp(codec::Error),
    #[error("Unable to decode blockNumber: {0}")]
    DecodeBlockNumber(codec::Error),
    #[error("Unable to decode aura authorities: {0}")]
    DecodeAuraAuthorities(codec::Error),
    #[error("Unable to decode the next fee multiplier: {0}")]
    DecodeNextFeeMultiplier(codec::Error),
    #[error("Unable to decode relay slot info: {0}")]
    DecodeRelaySlotInfo(codec::Error),
    #[error("Unable to decode last relay block number: {0}")]
    DecodeLastRelayBlockNumber(codec::Error),
    #[error("Unable to decode aura current slot: {0}")]
    DecodeAuraCurrentSlot(codec::Error),
}

type Result<T> = std::result::Result<T, BackendError>;

pub struct BackendWithOverlay {
    backend: Arc<Backend>,
    overrides: Arc<Mutex<StorageOverrides>>,
}

impl BackendWithOverlay {
    pub fn new(backend: Arc<Backend>, overrides: Arc<Mutex<StorageOverrides>>) -> Self {
        Self { backend, overrides }
    }

    pub fn blockchain(&self) -> &Blockchain<Block> {
        self.backend.blockchain()
    }

    pub fn read_timestamp(&self, hash: Hash) -> Result<u64> {
        let key = well_known_keys::TIMESTAMP;

        let value =
            self.read_top_state(hash, key.to_vec())?.ok_or(BackendError::MissingTimestamp)?;
        u64::decode(&mut &value[..]).map_err(BackendError::DecodeTimestamp)
    }

    pub fn read_relay_slot_info(&self, hash: Hash) -> Result<(Slot, u32)> {
        let key = well_known_keys::RELAY_SLOT_INFO;

        let value =
            self.read_top_state(hash, key.to_vec())?.ok_or(BackendError::MissingRelaySlotInfo)?;
        <(Slot, u32)>::decode(&mut &value[..]).map_err(BackendError::DecodeRelaySlotInfo)
    }

    pub fn read_last_relay_chain_block_number(&self, hash: Hash) -> Result<u32> {
        let key = well_known_keys::LAST_RELAY_CHAIN_BLOCK_NUMBER;

        let value = self
            .read_top_state(hash, key.to_vec())?
            .ok_or(BackendError::MissingLastRelayBlockNumber)?;
        u32::decode(&mut &value[..]).map_err(BackendError::DecodeLastRelayBlockNumber)
    }

    pub fn read_aura_current_slot(&self, hash: Hash) -> Result<Slot> {
        let key = well_known_keys::CURRENT_SLOT;

        let value =
            self.read_top_state(hash, key.to_vec())?.ok_or(BackendError::MissingAuraCurrentSlot)?;
        Slot::decode(&mut &value[..]).map_err(BackendError::DecodeAuraCurrentSlot)
    }

    pub fn read_block_number(&self, hash: Hash) -> Result<u32> {
        let key = well_known_keys::BLOCK_NUMBER_KEY;
        let value =
            self.read_top_state(hash, key.to_vec())?.ok_or(BackendError::MissingBlockNumber)?;
        u32::decode(&mut &value[..]).map_err(BackendError::DecodeBlockNumber)
    }

    pub fn read_chain_id(&self, hash: Hash) -> Result<u64> {
        let key = well_known_keys::CHAIN_ID;

        let value = self.read_top_state(hash, key.to_vec())?.ok_or(BackendError::MissingChainId)?;
        u64::decode(&mut &value[..]).map_err(BackendError::DecodeChainId)
    }

    pub fn read_aura_authority(&self, hash: Hash) -> Result<AccountId> {
        let key = well_known_keys::AURA_AUTHORITIES;

        let value =
            self.read_top_state(hash, key.to_vec())?.ok_or(BackendError::MissingAuraAuthorities)?;
        let authorities = <Vec<[u8; 32]>>::decode(&mut &value[..])
            .map_err(BackendError::DecodeAuraAuthorities)?;
        // Read the first authority, since that's what we modify via RPC, or at genesis,
        // and instruct the runtime to pick via the consensus data provider, whenever it
        // needs the block author.
        let authority = *authorities.first().ok_or(BackendError::MissingAuraAuthorities)?;
        Ok(authority.into())
    }

    pub fn read_total_issuance(&self, hash: Hash) -> Result<Balance> {
        let key = well_known_keys::TOTAL_ISSUANCE;

        let value =
            self.read_top_state(hash, key.to_vec())?.ok_or(BackendError::MissingTotalIssuance)?;
        Balance::decode(&mut &value[..]).map_err(BackendError::DecodeTotalIssuance)
    }

    pub fn read_system_account_info(
        &self,
        hash: Hash,
        account_id: AccountId,
    ) -> Result<Option<SystemAccountInfo>> {
        let key = well_known_keys::system_account_info(account_id);

        self.read_top_state(hash, key)?
            .map(|value| {
                SystemAccountInfo::decode(&mut &value[..])
                    .map_err(BackendError::DecodeSystemAccountInfo)
            })
            .transpose()
    }

    pub fn read_revive_account_info(
        &self,
        hash: Hash,
        address: Address,
    ) -> Result<Option<ReviveAccountInfo>> {
        let key = well_known_keys::revive_account_info(H160::from_slice(address.as_slice()));

        self.read_top_state(hash, key)?
            .map(|value| {
                ReviveAccountInfo::decode(&mut &value[..])
                    .map_err(BackendError::DecodeReviveAccountInfo)
            })
            .transpose()
    }

    pub fn read_code_info(&self, hash: Hash, code_hash: H256) -> Result<Option<CodeInfo>> {
        let key = well_known_keys::code_info(code_hash);

        self.read_top_state(hash, key)?
            .map(|value| CodeInfo::decode(&mut &value[..]).map_err(BackendError::DecodeCodeInfo))
            .transpose()
    }

    pub fn inject_system_account_info(
        &self,
        at: Hash,
        account_id: AccountId,
        value: SystemAccountInfo,
    ) {
        let mut overrides = self.overrides.lock();
        overrides.set_system_account_info(at, account_id, value);
    }

    pub fn inject_timestamp(&self, at: Hash, timestamp: u64) {
        let mut overrides = self.overrides.lock();
        overrides.set_timestamp(at, timestamp);
    }

    pub fn inject_relay_slot_info(&self, at: Hash, slot_info: (Slot, u32)) {
        let mut overrides = self.overrides.lock();
        overrides.set_relay_slot_info(at, slot_info);
    }

    pub fn inject_last_relay_chain_block_number(&self, at: Hash, number: u32) {
        let mut overrides = self.overrides.lock();
        overrides.set_last_relay_chain_block_number(at, number);
    }

    pub fn inject_aura_current_slot(&self, at: Hash, slot: Slot) {
        let mut overrides = self.overrides.lock();
        overrides.set_aura_current_slot(at, slot);
    }

    pub fn inject_chain_id(&self, at: Hash, chain_id: u64) {
        let mut overrides = self.overrides.lock();
        overrides.set_chain_id(at, chain_id);
    }

    pub fn inject_aura_authority(&self, at: Hash, aura_authority: AccountId) {
        let mut overrides = self.overrides.lock();
        overrides.set_coinbase(at, aura_authority);
    }

    pub fn inject_next_fee_multiplier(&self, at: Hash, next_fee_multiplier: FixedU128) {
        let mut overrides = self.overrides.lock();
        overrides.set_next_fee_multiplier(at, next_fee_multiplier);
    }

    pub fn inject_total_issuance(&self, at: Hash, value: Balance) {
        let mut overrides = self.overrides.lock();
        overrides.set_total_issuance(at, value);
    }

    pub fn inject_revive_account_info(&self, at: Hash, address: Address, info: ReviveAccountInfo) {
        let mut overrides = self.overrides.lock();
        overrides.set_revive_account_info(at, address, info);
    }

    pub fn inject_pristine_code(&self, at: Hash, code_hash: H256, code: Option<Bytes>) {
        let mut overrides = self.overrides.lock();
        overrides.set_pristine_code(at, code_hash, code);
    }

    pub fn inject_code_info(&self, at: Hash, code_hash: H256, code_info: Option<CodeInfo>) {
        let mut overrides = self.overrides.lock();
        overrides.set_code_info(at, code_hash, code_info);
    }

    pub fn inject_child_storage(
        &self,
        at: Hash,
        child_key: StorageKey,
        key: StorageKey,
        value: StorageValue,
    ) {
        let mut overrides = self.overrides.lock();
        overrides.set_child_storage(at, child_key, key, value);
    }

    fn read_top_state(&self, hash: Hash, key: StorageKey) -> Result<Option<StorageValue>> {
        let maybe_overridden_val = {
            let mut guard = self.overrides.lock();

            guard.per_block.get(&hash).and_then(|overrides| overrides.top.get(&key).cloned())
        };

        if let Some(overridden_val) = maybe_overridden_val {
            return Ok(overridden_val);
        }

        let state = self.backend.state_at(hash, TrieCacheContext::Trusted)?;
        Ok(state
            .storage(key.as_slice())
            .map_err(|e| sp_blockchain::Error::from_state(Box::new(e)))?)
    }
}

pub type Storage = HashMap<StorageKey, Option<StorageValue>>;

#[derive(Default, Clone)]
pub struct BlockOverrides {
    pub top: Storage,
    pub children: HashMap<StorageKey, Storage>,
}

pub struct StorageOverrides {
    // We keep N most recently used block state overrides because we may later get RPC calls which
    // query the state of past blocks. When state is mutated by the `set_*` RPCs, it gets committed
    // to the state DB only in the next block.
    per_block: LruCache<Hash, BlockOverrides>,
}

impl StorageOverrides {
    pub fn new(lru_capacity: Option<usize>) -> Self {
        Self {
            per_block: LruCache::new(
                lru_capacity.and_then(NonZeroUsize::new).unwrap_or(DEFAULT_LRU_CAP),
            ),
        }
    }

    pub fn get(&mut self, block: &Hash) -> Option<BlockOverrides> {
        self.per_block.get(block).cloned()
    }

    fn set_chain_id(&mut self, latest_block: Hash, id: u64) {
        let mut changeset = BlockOverrides::default();
        changeset.top.insert(well_known_keys::CHAIN_ID.to_vec(), Some(id.encode()));

        self.add(latest_block, changeset);
    }

    fn set_relay_slot_info(&mut self, latest_block: Hash, slot_info: (Slot, u32)) {
        let mut changeset = BlockOverrides::default();
        changeset.top.insert(well_known_keys::RELAY_SLOT_INFO.to_vec(), Some(slot_info.encode()));

        self.add(latest_block, changeset);
    }

    fn set_last_relay_chain_block_number(&mut self, latest_block: Hash, number: u32) {
        let mut changeset = BlockOverrides::default();
        changeset
            .top
            .insert(well_known_keys::LAST_RELAY_CHAIN_BLOCK_NUMBER.to_vec(), Some(number.encode()));

        self.add(latest_block, changeset);
    }

    fn set_aura_current_slot(&mut self, latest_block: Hash, slot: Slot) {
        let mut changeset = BlockOverrides::default();
        changeset.top.insert(well_known_keys::CURRENT_SLOT.to_vec(), Some(slot.encode()));

        self.add(latest_block, changeset);
    }

    fn set_coinbase(&mut self, latest_block: Hash, aura_authority: AccountId) {
        let mut changeset = BlockOverrides::default();
        changeset.top.insert(
            well_known_keys::AURA_AUTHORITIES.to_vec(),
            Some(vec![aura_authority].encode()),
        );

        self.add(latest_block, changeset);
    }

    fn set_timestamp(&mut self, latest_block: Hash, timestamp: u64) {
        let mut changeset = BlockOverrides::default();
        changeset.top.insert(well_known_keys::TIMESTAMP.to_vec(), Some(timestamp.encode()));

        self.add(latest_block, changeset);
    }

    fn set_next_fee_multiplier(&mut self, latest_block: Hash, next_fee_multiplier: FixedU128) {
        let mut changeset = BlockOverrides::default();
        changeset.top.insert(
            well_known_keys::NEXT_FEE_MULTIPLIER.to_vec(),
            Some(next_fee_multiplier.encode()),
        );

        self.add(latest_block, changeset);
    }

    fn set_system_account_info(
        &mut self,
        latest_block: Hash,
        account_id: AccountId,
        info: SystemAccountInfo,
    ) {
        let mut changeset = BlockOverrides::default();
        changeset.top.insert(well_known_keys::system_account_info(account_id), Some(info.encode()));

        self.add(latest_block, changeset);
    }

    fn set_total_issuance(&mut self, latest_block: Hash, value: Balance) {
        let mut changeset = BlockOverrides::default();
        changeset.top.insert(well_known_keys::TOTAL_ISSUANCE.to_vec(), Some(value.encode()));

        self.add(latest_block, changeset);
    }

    fn set_revive_account_info(
        &mut self,
        latest_block: Hash,
        address: Address,
        info: ReviveAccountInfo,
    ) {
        let mut changeset = BlockOverrides::default();
        changeset.top.insert(
            well_known_keys::revive_account_info(H160::from_slice(address.as_slice())),
            Some(info.encode()),
        );

        self.add(latest_block, changeset);
    }

    fn set_pristine_code(&mut self, latest_block: Hash, code_hash: H256, code: Option<Bytes>) {
        let mut changeset = BlockOverrides::default();

        changeset
            .top
            .insert(well_known_keys::pristine_code(code_hash), code.map(|code| code.0.encode()));

        self.add(latest_block, changeset);
    }

    fn set_code_info(&mut self, latest_block: Hash, code_hash: H256, code_info: Option<CodeInfo>) {
        let mut changeset = BlockOverrides::default();

        changeset.top.insert(
            well_known_keys::code_info(code_hash),
            code_info.map(|code_info| code_info.encode()),
        );

        self.add(latest_block, changeset);
    }

    fn set_child_storage(
        &mut self,
        latest_block: Hash,
        child_key: StorageKey,
        key: StorageKey,
        value: StorageValue,
    ) {
        let mut changeset = BlockOverrides::default();

        let mut child_map = Storage::with_capacity(1);
        child_map.insert(blake2_256(key.as_slice()).to_vec(), Some(value));

        changeset.children.insert(child_key, child_map);

        self.add(latest_block, changeset);
    }

    fn add(&mut self, latest_block: Hash, changeset: BlockOverrides) {
        if let Some(per_block) = self.per_block.get_mut(&latest_block) {
            per_block.top.extend(changeset.top);

            for (child_key, child_map) in changeset.children {
                per_block.children.entry(child_key).or_default().extend(child_map);
            }
        } else {
            self.per_block.put(latest_block, changeset);
        }
    }
}
