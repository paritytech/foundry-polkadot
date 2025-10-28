//! Genesis settings

use crate::{
    api_server::revive_conversions::ReviveAddress, config::AnvilNodeConfig,
    substrate_node::service::storage::well_known_keys,
};
use alloy_genesis::GenesisAccount;
use alloy_primitives::{Address, U256};
use codec::Encode;
use polkadot_sdk::{
    pallet_revive::genesis::ContractData,
    sc_chain_spec::{BuildGenesisBlock, resolve_state_version_from_wasm},
    sc_client_api::{BlockImportOperation, backend::Backend},
    sc_executor::RuntimeVersionOf,
    sp_blockchain,
    sp_core::{H160, storage::Storage},
    sp_runtime::{
        BuildStorage,
        traits::{Block as BlockT, Hash as HashT, HashingFor, Header as HeaderT},
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

/// Genesis settings
#[derive(Clone, Debug, Default)]
pub struct GenesisConfig {
    /// The chain id of the Substrate chain.
    pub chain_id: u64,
    /// The initial timestamp for the genesis block in milliseconds
    pub timestamp: u64,
    /// All accounts that should be initialised at genesis with their info.
    pub alloc: Option<BTreeMap<Address, GenesisAccount>>,
    /// The initial number for the genesis block
    pub number: u32,
    /// The genesis header base fee
    pub base_fee_per_gas: u64,
    /// The genesis header gas limit.
    pub gas_limit: Option<u128>,
}

impl<'a> From<&'a AnvilNodeConfig> for GenesisConfig {
    fn from(anvil_config: &'a AnvilNodeConfig) -> Self {
        Self {
            chain_id: anvil_config.get_chain_id(),
            // Anvil genesis timestamp is in seconds, while Substrate timestamp is in milliseconds.
            timestamp: anvil_config
                .get_genesis_timestamp()
                .checked_mul(1000)
                .expect("Genesis timestamp overflow"),
            alloc: anvil_config.genesis.as_ref().map(|g| g.alloc.clone()),
            number: anvil_config
                .get_genesis_number()
                .try_into()
                .expect("Genesis block number overflow"),
            base_fee_per_gas: anvil_config.get_base_fee(),
            gas_limit: anvil_config.gas_limit,
        }
    }
}

/// Used to provide genesis accounts to pallet-revive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviveGenesisAccount {
    pub address: H160,
    #[serde(default)]
    pub balance: U256,
    #[serde(default)]
    pub nonce: u64,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub contract_data: Option<ContractData>,
}

impl GenesisConfig {
    pub fn as_storage_key_value(&self) -> Vec<(Vec<u8>, Vec<u8>)> {
        let storage = vec![
            (well_known_keys::CHAIN_ID.to_vec(), self.chain_id.encode()),
            (well_known_keys::TIMESTAMP.to_vec(), self.timestamp.encode()),
            (well_known_keys::BLOCK_NUMBER_KEY.to_vec(), self.number.encode()),
        ];
        // TODO: add other fields
        storage
    }

    pub fn runtime_genesis_config_patch(&self) -> Value {
        // Relies on ReviveGenesisAccount type from pallet-revive
        let revive_genesis_accounts: Vec<ReviveGenesisAccount> = self
            .alloc
            .clone()
            .unwrap_or_default()
            .iter()
            .map(|(address, account)| {
                let genesis_address: H160 = ReviveAddress::from(*address).inner();
                let genesis_balance: U256 = account.balance;
                let genesis_nonce: u64 = account.nonce.unwrap_or_default();
                let contract_data: Option<ContractData> = if account.code.is_some() {
                    Some(ContractData {
                        code: account.code.clone().map(|code| code.to_vec()).unwrap_or_default(),
                        storage: account
                            .storage
                            .clone()
                            .map(|storage| {
                                storage
                                    .into_iter()
                                    .map(|(k, v)| (k.0.into(), v.0.into()))
                                    .collect::<BTreeMap<_, _>>()
                            })
                            .unwrap_or_default(),
                    })
                } else {
                    None
                };

                ReviveGenesisAccount {
                    address: genesis_address,
                    balance: genesis_balance,
                    nonce: genesis_nonce,
                    contract_data,
                }
            })
            .collect();

        json!({
            "revive": {
                "accounts": revive_genesis_accounts,
            },
        })
    }
}

pub struct DevelopmentGenesisBlockBuilder<Block: BlockT, B, E> {
    genesis_number: u32,
    genesis_storage: Storage,
    commit_genesis_state: bool,
    backend: Arc<B>,
    executor: E,
    _phantom: PhantomData<Block>,
}

impl<Block: BlockT, B: Backend<Block>, E: RuntimeVersionOf>
    DevelopmentGenesisBlockBuilder<Block, B, E>
{
    pub fn new(
        genesis_number: u64,
        build_genesis_storage: &dyn BuildStorage,
        commit_genesis_state: bool,
        backend: Arc<B>,
        executor: E,
    ) -> sp_blockchain::Result<Self> {
        let genesis_storage =
            build_genesis_storage.build_storage().map_err(sp_blockchain::Error::Storage)?;
        Self::new_with_storage(
            genesis_number,
            genesis_storage,
            commit_genesis_state,
            backend,
            executor,
        )
    }

    pub fn new_with_storage(
        genesis_number: u64,
        genesis_storage: Storage,
        commit_genesis_state: bool,
        backend: Arc<B>,
        executor: E,
    ) -> sp_blockchain::Result<Self> {
        Ok(Self {
            genesis_number: genesis_number.try_into().map_err(|_| {
                sp_blockchain::Error::Application(
                    format!(
                        "Genesis number {} is too large for u32 (max: {})",
                        genesis_number,
                        u32::MAX
                    )
                    .into(),
                )
            })?,
            genesis_storage,
            commit_genesis_state,
            backend,
            executor,
            _phantom: PhantomData::<Block>,
        })
    }
}

impl<Block: BlockT, B: Backend<Block>, E: RuntimeVersionOf> BuildGenesisBlock<Block>
    for DevelopmentGenesisBlockBuilder<Block, B, E>
{
    type BlockImportOperation = <B as Backend<Block>>::BlockImportOperation;

    fn build_genesis_block(self) -> sp_blockchain::Result<(Block, Self::BlockImportOperation)> {
        let Self {
            genesis_number,
            genesis_storage,
            commit_genesis_state,
            backend,
            executor,
            _phantom,
        } = self;

        let genesis_state_version =
            resolve_state_version_from_wasm::<_, HashingFor<Block>>(&genesis_storage, &executor)?;
        let mut op = backend.begin_operation()?;
        let state_root =
            op.set_genesis_state(genesis_storage, commit_genesis_state, genesis_state_version)?;
        let extrinsics_root = <<<Block as BlockT>::Header as HeaderT>::Hashing as HashT>::trie_root(
            Vec::new(),
            genesis_state_version,
        );
        let genesis_block = Block::new(
            <<Block as BlockT>::Header as HeaderT>::new(
                genesis_number.into(),
                extrinsics_root,
                state_root,
                Default::default(),
                Default::default(),
            ),
            Default::default(),
        );

        Ok((genesis_block, op))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_encoding() {
        let block_number: u32 = 5;
        let timestamp: u64 = 10;
        let chain_id: u64 = 42;
        let genesis_config =
            GenesisConfig { number: block_number, timestamp, chain_id, ..Default::default() };
        let genesis_storage = genesis_config.as_storage_key_value();
        assert!(
            genesis_storage
                .contains(&(well_known_keys::BLOCK_NUMBER_KEY.to_vec(), block_number.encode())),
            "Block number not found in genesis key-value storage"
        );
        assert!(
            genesis_storage.contains(&(well_known_keys::TIMESTAMP.to_vec(), timestamp.encode())),
            "Timestamp not found in genesis key-value storage"
        );
        assert!(
            genesis_storage.contains(&(well_known_keys::CHAIN_ID.to_vec(), chain_id.encode())),
            "Chain id not found in genesis key-value storage"
        );
    }
}
