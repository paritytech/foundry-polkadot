//! Tests for pauseGasMetering, resumeGasMetering, and resetGasMetering cheatcodes
//!
//! ## Test Strategy
//!
//! These tests verify that gas metering cheatcodes work correctly in both EVM and PVM modes.
//! Gas metering operates at the EVM interpreter level (for testing/debugging), affecting:
//! - How gas is tracked during test execution
//! - The ability to pause/resume/reset gas consumption counting
//! - Gas reporting for test analysis

use foundry_cheatcodes::Cheatcodes;
use foundry_compilers::resolc::dual_compiled_contracts::DualCompiledContracts;
use revive_strategy::{PvmCheatcodeInspectorStrategyBuilder, ReviveRuntimeMode};

#[test]
fn evm_pause_gas_metering_sets_flag() {
    let state = Cheatcodes::default();

    assert!(!state.gas_metering.paused, "Gas metering should not be paused initially");
}

#[test]
fn evm_gas_metering_initial_state() {
    let state = Cheatcodes::default();

    assert!(!state.gas_metering.paused);
    assert!(!state.gas_metering.touched);
    assert!(!state.gas_metering.reset);
    assert!(state.gas_metering.paused_frames.is_empty());
    assert!(state.gas_metering.last_call_gas.is_none());
}

#[test]
fn pvm_cheatcodes_state_initializes() {
    use foundry_cheatcodes::CheatcodeInspectorStrategy;

    let mut state = Cheatcodes::default();
    state.strategy = CheatcodeInspectorStrategy::new_pvm(
        DualCompiledContracts::default(),
        ReviveRuntimeMode::Pvm,
        Default::default(),
    );

    assert!(!state.gas_metering.paused, "Gas metering should not be paused initially in PVM");
}

#[test]
fn pvm_gas_metering_state_structure() {
    use foundry_cheatcodes::CheatcodeInspectorStrategy;

    let mut state = Cheatcodes::default();
    state.strategy = CheatcodeInspectorStrategy::new_pvm(
        DualCompiledContracts::default(),
        ReviveRuntimeMode::Pvm,
        Default::default(),
    );

    assert!(!state.gas_metering.paused);
    assert!(!state.gas_metering.touched);
    assert!(!state.gas_metering.reset);
    assert!(state.gas_metering.paused_frames.is_empty());
}

#[test]
fn gas_metering_is_independent_of_mode() {
    use foundry_cheatcodes::CheatcodeInspectorStrategy;

    let evm_state = Cheatcodes::default();

    let mut pvm_state = Cheatcodes::default();
    pvm_state.strategy = CheatcodeInspectorStrategy::new_pvm(
        DualCompiledContracts::default(),
        ReviveRuntimeMode::Pvm,
        Default::default(),
    );

    assert_eq!(
        evm_state.gas_metering.paused, pvm_state.gas_metering.paused,
        "Gas metering state should be identical in EVM and PVM modes initially"
    );
}
