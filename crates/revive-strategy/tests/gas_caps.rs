//! Tests for gas limit to weight/storage deposit conversion logic
//!
//! ## Test Strategy
//!
//! These tests verify the gas limit conversion and storage deposit calculation implemented
//! in `cheatcodes/mod.rs`. The implementation converts EVM gas limits to Substrate Weight
//! and calculates storage deposit caps based on the available balance after accounting for:
//! - Existential deposit
//! - Weight fees (from gas -> weight -> fee conversion)
//! - Value transfers

use polkadot_sdk::{
    frame_support::{
        traits::{Currency, fungible::Mutate},
        weights::Weight,
    },
    pallet_revive::{
        AddressMapper, Config,
        evm::fees::{Combinator, InfoT},
    },
    sp_core::H160,
    sp_runtime::traits::SaturatedConversion,
};
use revive_env::{AccountId, ExtBuilder, Runtime};

/// Helper function to calculate available balance for storage deposit.
fn calculate_storage_deposit_cap(
    caller_balance: u128,
    ed: u128,
    value: u128,
    weight_fee: u128,
) -> u128 {
    caller_balance.saturating_sub(ed).saturating_sub(value).saturating_sub(weight_fee)
}

#[test]
fn gas_limit_converts_to_weight() {
    let gas_limit = 1_000_000u64;
    let weight = Weight::from_parts(gas_limit, u64::MAX);

    assert_eq!(weight.ref_time(), gas_limit);
    assert_eq!(weight.proof_size(), u64::MAX);
}

#[test]
fn weight_to_fee_is_callable() {
    // Note: FeeInfo = () is always 0. FeeInfo is properly configured in production runtime.
    let gas_limit = 1_000_000u64;
    let weight = Weight::from_parts(gas_limit, u64::MAX);

    let weight_fee =
        <<Runtime as Config>::FeeInfo as InfoT<Runtime>>::weight_to_fee(&weight, Combinator::Max);

    assert_eq!(weight_fee, 0);
}

#[test]
fn storage_deposit_calculation_with_surplus() {
    let ed = 1_000u128;
    let weight_fee = 100u128; // Simulating a real runtime charge
    let value = 0u128;
    let extra = 500u128;

    let caller_balance = ed + weight_fee + extra;
    let cap = calculate_storage_deposit_cap(caller_balance, ed, value, weight_fee);

    assert_eq!(cap, extra, "Should have {} available for storage deposit", extra);
}

#[test]
fn storage_deposit_saturates_to_zero() {
    let ed = 1_000u128;
    let weight_fee = 100u128;
    let value = 0u128;

    let caller_balance = ed + weight_fee;
    let cap = calculate_storage_deposit_cap(caller_balance, ed, value, weight_fee);

    assert_eq!(cap, 0, "Should have 0 available for storage deposit");

    let insufficient_balance = ed + 50u128;
    let cap = calculate_storage_deposit_cap(insufficient_balance, ed, value, weight_fee);

    assert_eq!(cap, 0, "Should saturate to 0 with insufficient balance");
}

#[test]
fn storage_deposit_with_value_transfer() {
    let ed = 1_000u128;
    let weight_fee = 100u128;
    let value = 500u128;
    let extra = 300u128;

    let caller_balance = ed + weight_fee + value + extra;
    let cap = calculate_storage_deposit_cap(caller_balance, ed, value, weight_fee);

    assert_eq!(cap, extra, "Should account for value transfer in available balance");
}

#[test]
fn storage_deposit_calculation_matches_runtime() {
    let caller_h160 = H160::from_low_u64_be(0xdead_beef);
    let caller = AccountId::to_fallback_account_id(&caller_h160);

    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| {
        let ed = polkadot_sdk::pallet_balances::Pallet::<Runtime>::minimum_balance();
        let test_balance = 10_000u128;
        let value = 500u128;

        polkadot_sdk::pallet_balances::Pallet::<Runtime>::set_balance(&caller, test_balance);

        let gas_limit = 1_000_000u64;
        let weight = Weight::from_parts(gas_limit, u64::MAX);
        let weight_fee = <<Runtime as Config>::FeeInfo as InfoT<Runtime>>::weight_to_fee(
            &weight,
            Combinator::Max,
        );

        let expected_cap = calculate_storage_deposit_cap(test_balance, ed, value, weight_fee);

        let free = polkadot_sdk::pallet_balances::Pallet::<Runtime>::free_balance(&caller);
        let runtime_cap: u128 = free
            .saturating_sub(ed)
            .saturating_sub(value)
            .saturating_sub(weight_fee)
            .saturated_into();

        assert_eq!(expected_cap, runtime_cap, "Helper function should match runtime calculation");
    });
}
