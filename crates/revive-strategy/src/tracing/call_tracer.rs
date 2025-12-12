use alloy_primitives::{Address, U256 as RU256};
use foundry_cheatcodes::ExpectedCallTracker;
use polkadot_sdk::{
    pallet_revive,
    pallet_revive::tracing::Tracing,
    sp_core::{H160, U256},
    sp_weights::Weight,
};

#[derive(Debug)]
pub(crate) struct ExpectedCallTracer {
    pub data: ExpectedCallTracker,
    is_create: bool,
}

impl ExpectedCallTracer {
    pub fn new(data: ExpectedCallTracker) -> Self {
        Self { data, is_create: false }
    }
}

impl Tracing for ExpectedCallTracer {
    fn enter_child_span(
        &mut self,
        _from: H160,
        to: H160,
        _is_delegate_call: bool,
        _is_read_only: bool,
        value: U256,
        input: &[u8],
        _gas: Weight,
    ) {
        if !self.is_create
            && let Some(expected_calls_for_target) = self.data.get_mut(&Address::from(to.0))
        {
            // Match every partial/full calldata
            for (calldata, (expected, actual_count)) in expected_calls_for_target {
                // Increment actual times seen if...
                // The calldata is at most, as big as this call's input, and
                if calldata.len() <= input.len() &&
                    // Both calldata match, taking the length of the assumed smaller one (which will have at least the selector), and
                    *calldata == input[..calldata.len()] &&
                    // The value matches, if provided
                    expected
                        .value.is_none_or(|v| v == RU256::from_limbs(value.0))
                // gas tracking is broken now
                // // The gas matches, if provided
                // expected.gas.is_none_or(|g| g == gas) &&
                // // The minimum gas matches, if provided
                {
                    *actual_count += 1;
                }
            }
        }
    }
    fn exit_child_span(&mut self, _output: &pallet_revive::ExecReturnValue, _gas_left: Weight) {
        self.is_create = false;
    }
    fn instantiate_code(
        &mut self,
        _code: &polkadot_sdk::pallet_revive::Code,
        _salt: Option<&[u8; 32]>,
    ) {
        self.is_create = true;
    }
}
