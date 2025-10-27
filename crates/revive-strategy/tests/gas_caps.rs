use polkadot_sdk::{
    frame_support::{
        dispatch::DispatchClass,
        traits::{Currency, fungible::Mutate},
    },
    pallet_revive::AddressMapper,
    sp_core::H160,
    sp_runtime::traits::SaturatedConversion,
};
use revive_env::{AccountId, ExtBuilder, Runtime};

#[test]
fn normal_class_weight_cap_nonzero() {
    let block_weights = <Runtime as polkadot_sdk::frame_system::Config>::BlockWeights::get();
    let normal = block_weights.get(DispatchClass::Normal);
    let max_weight_cap = normal
        .max_extrinsic
        .unwrap_or(normal.max_total.unwrap_or(block_weights.max_block))
        .saturating_sub(normal.base_extrinsic);
    assert!(max_weight_cap.ref_time() > 0 || max_weight_cap.proof_size() > 0);
}

#[test]
fn storage_deposit_cap_saturates_to_zero() {
    let caller_h160 = H160::from_low_u64_be(0xdead_beef);
    let caller = AccountId::to_fallback_account_id(&caller_h160);

    // Create externalities
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| {
        let ed = polkadot_sdk::pallet_balances::Pallet::<Runtime>::minimum_balance();
        let value_sent_native = ed;
        let free = ed.saturating_add(value_sent_native);
        polkadot_sdk::pallet_balances::Pallet::<Runtime>::set_balance(&caller, free);

        let free_now = polkadot_sdk::pallet_balances::Pallet::<Runtime>::free_balance(&caller);
        let available = free_now.saturating_sub(ed).saturating_sub(value_sent_native);

        let cap: u128 = available.saturated_into();
        assert_eq!(cap, 0u128);
    });
}
