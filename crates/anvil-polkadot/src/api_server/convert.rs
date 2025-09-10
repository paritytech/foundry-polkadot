use alloy_eips::{BlockId, BlockNumberOrTag};
use alloy_primitives::{Address, U160, U256 as AU256};
use alloy_rpc_types::{request::TransactionRequest, AccessList};
use alloy_signer::k256::elliptic_curve::weierstrass::add;
use polkadot_sdk::{
    pallet_revive::evm::{
        AccessListEntry, BlockNumberOrTagOrHash, BlockTag, Byte, GenericTransaction,
    },
    sp_core::{H256, U256 as SU256},
};
use subxt::utils::H160;

pub(crate) fn from_address_to_h160(addr: Address) -> H160 {
    H160::from_slice(addr.as_ref())
}

pub fn from_h160_to_address(addr: H160) -> Address {
    Address(U160::from_be_bytes(*addr.as_fixed_bytes()).into())
}

pub(crate) fn from_sp_u256_to_alloy_u256(from: SU256) -> AU256 {
    AU256::from_be_bytes(from.to_big_endian())
}

pub(crate) fn from_alloy_u256_to_sp_u256(from: AU256) -> SU256 {
    SU256::from_big_endian(&from.to_be_bytes_vec())
}

pub(crate) fn to_block_number_or_tag_or_hash(block_id: Option<BlockId>) -> BlockNumberOrTagOrHash {
    // TODO: handle the fork case
    block_id.map_or(BlockNumberOrTagOrHash::BlockTag(BlockTag::Latest), |b_id| match b_id {
        BlockId::Hash(rpc_hash) => {
            let h256: H256 = H256::from_slice(rpc_hash.block_hash.as_slice());
            BlockNumberOrTagOrHash::BlockHash(h256)
        }
        BlockId::Number(number_or_tag) => match number_or_tag {
            BlockNumberOrTag::Number(num) => {
                BlockNumberOrTagOrHash::BlockNumber(polkadot_sdk::pallet_revive::U256::from(num))
            }
            BlockNumberOrTag::Latest => BlockNumberOrTagOrHash::BlockTag(BlockTag::Latest),
            BlockNumberOrTag::Earliest => BlockNumberOrTagOrHash::BlockTag(BlockTag::Earliest),
            BlockNumberOrTag::Pending => BlockNumberOrTagOrHash::BlockTag(BlockTag::Pending),
            BlockNumberOrTag::Safe => BlockNumberOrTagOrHash::BlockTag(BlockTag::Safe),
            BlockNumberOrTag::Finalized => BlockNumberOrTagOrHash::BlockTag(BlockTag::Finalized),
        },
    })
}

fn convert_alloy_access_list_to_sp_access_list(
    access_list: Option<AccessList>,
) -> Option<Vec<AccessListEntry>> {
    access_list.map_or(None, |acc| {
        Some(
            acc.0
                .into_iter()
                .map(|access_list_entry| AccessListEntry {
                    address: from_address_to_h160(access_list_entry.address),
                    storage_keys: access_list_entry
                        .storage_keys
                        .into_iter()
                        .map(|storage_key| H256::from_slice(storage_key.as_ref()))
                        .collect(),
                })
                .collect(),
        )
    })
}

pub(crate) fn try_convert_transaction_request(
    transaction: TransactionRequest,
) -> GenericTransaction {
    GenericTransaction {
        access_list: convert_alloy_access_list_to_sp_access_list(transaction.access_list),
        blob_versioned_hashes: transaction
            .blob_versioned_hashes
            .unwrap_or_default()
            .into_iter()
            .map(|b256| H256::from(b256.as_ref()))
            .collect(),
        blobs: Default::default(),
        chain_id: transaction.chain_id.map(SU256::from),
        from: transaction.from.map(|addr| from_address_to_h160(addr)),
        gas: transaction.gas.map(|gas| SU256::from(gas)),
        gas_price: transaction.gas_price.map(|gas_price| SU256::from(gas_price)),
        input: Default::default(),
        max_fee_per_blob_gas: transaction.max_fee_per_blob_gas.map(|mfpbg| SU256::from(mfpbg)),
        max_fee_per_gas: transaction.max_fee_per_gas.map(|mfpg| SU256::from(mfpg)),
        max_priority_fee_per_gas: transaction
            .max_priority_fee_per_gas
            .map(|mpfpg| SU256::from(mpfpg)),
        nonce: transaction.nonce.map(|nonce| SU256::from(nonce)),
        to: transaction.from.map(|addr| from_address_to_h160(addr)),
        r#type: transaction.transaction_type.map(|byte| Byte::from(byte)),
        value: transaction.value.map(|value| from_alloy_u256_to_sp_u256(value)),
    }
}
