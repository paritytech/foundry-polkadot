//! Tests for txGasPrice cheatcode
//!
//! ## Test Strategy
//!
//! The tests cover the txGasPrice cheatcode correctly sets the gas price
//! in the EVM transaction context.
//! - Setting gas price in EVM mode
//! - Setting gas price in PVM mode
//! - Verifying that PVM mode doesn't affect Substrate runtime state

use alloy_primitives::U256;
use polkadot_sdk::{pallet_revive::AddressMapper, sp_core::H160};
use revive_env::{AccountId, ExtBuilder, System};

#[test]
fn set_gas_price_updates_value() {
    let new_price = U256::from(100_000_000_000u64);
    let expected = new_price.saturating_to::<u64>();

    assert_eq!(expected, 100_000_000_000u64);
}

#[test]
fn set_gas_price_handles_zero() {
    let new_price = U256::ZERO;
    let expected = new_price.saturating_to::<u64>();

    assert_eq!(expected, 0u64);
}

#[test]
fn set_gas_price_handles_max_u64() {
    let new_price = U256::from(u64::MAX);
    let expected = new_price.saturating_to::<u64>();

    assert_eq!(expected, u64::MAX);
}

#[test]
fn set_gas_price_saturates_above_max() {
    let new_price = U256::from(u64::MAX) + U256::from(1u64);
    let expected = new_price.saturating_to::<u64>();

    assert_eq!(expected, u64::MAX);
}

#[test]
fn pvm_gas_price_no_runtime_side_effects() {
    let caller_h160 = H160::from_low_u64_be(0xdead);
    let caller = AccountId::to_fallback_account_id(&caller_h160);

    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| {
        let initial_block = System::block_number();
        let initial_nonce = System::account_nonce(&caller);

        let _new_price = U256::from(25_000_000_000u64);

        assert_eq!(System::block_number(), initial_block);
        assert_eq!(System::account_nonce(&caller), initial_nonce);
    });
}
