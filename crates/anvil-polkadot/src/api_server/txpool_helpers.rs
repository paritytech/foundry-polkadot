//! Helper functions for txpool RPC methods
//!
//! This module contains utilities for extracting transaction information from
//! Substrate extrinsics, including support for impersonated transactions with
//! fake signatures.

use alloy_primitives::{Address, B256, U256, keccak256};
use alloy_rpc_types::txpool::TxpoolInspectSummary;
use codec::{DecodeLimit, Encode};
use polkadot_sdk::{
    pallet_revive::evm::TransactionSigned,
    sp_core::{self, H256},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use substrate_runtime::{RuntimeCall, UncheckedExtrinsic};

use crate::substrate_node::host::recover_maybe_impersonated_address;

const MAX_EXTRINSIC_DEPTH: u32 = 256;

/// Transaction info for txpool RPCs with Option fields to match Anvil's null values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxpoolTransactionInfo {
    pub hash: H256,
    pub block_hash: Option<H256>,
    pub block_number: Option<sp_core::U256>,
    pub transaction_index: Option<sp_core::U256>,
    pub from: sp_core::H160,
    pub transaction_signed: TransactionSigned,
}

/// Decode extrinsic into ETH transaction payload and signed transaction
pub(super) fn decode_eth_transaction(
    tx_data: &Arc<polkadot_sdk::sp_runtime::OpaqueExtrinsic>,
) -> Option<(Vec<u8>, TransactionSigned)> {
    let encoded = tx_data.encode();
    let ext =
        UncheckedExtrinsic::decode_all_with_depth_limit(MAX_EXTRINSIC_DEPTH, &mut &encoded[..])
            .ok()?;

    let polkadot_sdk::sp_runtime::generic::UncheckedExtrinsic {
        function: RuntimeCall::Revive(polkadot_sdk::pallet_revive::Call::eth_transact { payload }),
        ..
    } = ext.0
    else {
        return None;
    };

    let signed_tx = TransactionSigned::decode(&payload).ok()?;

    Some((payload, signed_tx))
}

/// Check if transaction matches ETH hash
pub(super) fn transaction_matches_eth_hash(
    tx_data: &Arc<polkadot_sdk::sp_runtime::OpaqueExtrinsic>,
    target_eth_hash: B256,
) -> bool {
    let Some((payload, _signed_tx)) = decode_eth_transaction(tx_data) else {
        return false;
    };

    let tx_eth_hash = keccak256(&payload);
    B256::from_slice(tx_eth_hash.as_ref()) == target_eth_hash
}

/// Fields extracted from an Ethereum transaction
pub(super) struct TransactionFields {
    pub nonce: sp_core::U256,
    pub to: Option<sp_core::H160>,
    pub value: sp_core::U256,
    pub gas: sp_core::U256,
    pub gas_price: sp_core::U256,
}

/// Extract fields from ETH transaction
fn extract_tx_fields(signed_tx: &TransactionSigned) -> TransactionFields {
    match signed_tx {
        TransactionSigned::TransactionLegacySigned(tx) => {
            let t = &tx.transaction_legacy_unsigned;
            TransactionFields {
                nonce: t.nonce,
                to: t.to,
                value: t.value,
                gas: t.gas,
                gas_price: t.gas_price,
            }
        }
        TransactionSigned::Transaction2930Signed(tx) => {
            let t = &tx.transaction_2930_unsigned;
            TransactionFields {
                nonce: t.nonce,
                to: t.to,
                value: t.value,
                gas: t.gas,
                gas_price: t.gas_price,
            }
        }
        TransactionSigned::Transaction1559Signed(tx) => {
            let t = &tx.transaction_1559_unsigned;
            TransactionFields {
                nonce: t.nonce,
                to: t.to,
                value: t.value,
                gas: t.gas,
                gas_price: t.max_fee_per_gas,
            }
        }
        TransactionSigned::Transaction4844Signed(tx) => {
            let t = &tx.transaction_4844_unsigned;
            TransactionFields {
                nonce: t.nonce,
                to: Some(t.to),
                value: t.value,
                gas: t.gas,
                gas_price: t.max_fee_per_gas,
            }
        }
        TransactionSigned::Transaction7702Signed(tx) => {
            let t = &tx.transaction_7702_unsigned;
            TransactionFields {
                nonce: t.nonce,
                to: Some(t.to),
                value: t.value,
                gas: t.gas,
                gas_price: t.max_fee_per_gas,
            }
        }
    }
}

/// Extract transaction summary from extrinsic
pub(super) fn extract_tx_summary(
    tx_data: &Arc<polkadot_sdk::sp_runtime::OpaqueExtrinsic>,
) -> Option<(Address, u64, TxpoolInspectSummary)> {
    let (_payload, signed_tx) = decode_eth_transaction(tx_data)?;

    let from = recover_maybe_impersonated_address(&signed_tx).ok()?;
    let sender = Address::from_slice(from.as_bytes());

    let fields = extract_tx_fields(&signed_tx);

    let to_addr = fields.to.map(|addr| Address::from_slice(addr.as_bytes()));
    let value_u256 = U256::from_limbs(fields.value.0);
    let gas_u64 = fields.gas.as_u64();
    let gas_price_u128 = fields.gas_price.as_u128();
    let nonce_u64 = fields.nonce.as_u64();

    Some((
        sender,
        nonce_u64,
        TxpoolInspectSummary {
            to: to_addr,
            value: value_u256,
            gas: gas_u64,
            gas_price: gas_price_u128,
        },
    ))
}

/// Extract full transaction info from extrinsic
pub(super) fn extract_tx_info(
    tx_data: &Arc<polkadot_sdk::sp_runtime::OpaqueExtrinsic>,
) -> Option<(Address, u64, TxpoolTransactionInfo)> {
    let (payload, signed_tx) = decode_eth_transaction(tx_data)?;

    let eth_hash = keccak256(&payload);
    let eth_hash_h256 = H256::from_slice(eth_hash.as_ref());

    let from = recover_maybe_impersonated_address(&signed_tx).ok()?;
    let sender = Address::from_slice(from.as_bytes());

    let fields = extract_tx_fields(&signed_tx);
    let nonce_u64 = fields.nonce.as_u64();

    let tx_info = TxpoolTransactionInfo {
        hash: eth_hash_h256,
        block_hash: None,
        block_number: None,
        transaction_index: None,
        from,
        transaction_signed: signed_tx,
    };

    Some((sender, nonce_u64, tx_info))
}

/// Extract sender address from extrinsic as Alloy Address type.
/// Helper for `anvil_remove_pool_transactions` to compare sender addresses.
pub(super) fn extract_sender(
    tx_data: &Arc<polkadot_sdk::sp_runtime::OpaqueExtrinsic>,
) -> Option<Address> {
    let (_payload, signed_tx) = decode_eth_transaction(tx_data)?;

    let from = recover_maybe_impersonated_address(&signed_tx).ok()?;
    let sender = Address::from_slice(from.as_bytes());

    Some(sender)
}
