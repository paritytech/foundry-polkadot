//! This crate provides the Revive strategy for the Foundry EVM ExecutorStrategy.
//!
//! It is designed to work with the Revive runtime, allowing for the execution of smart contracts
//! in a Polkadot environment.
//!
//! It is heavily inspired from <https://github.com/matter-labs/foundry-zksync/tree/main/crates/strategy/zksync>
use std::fmt::Display;

use foundry_evm::executors::ExecutorStrategy;
use polkadot_sdk::{
    sp_core::{self, H160},
    sp_io,
    sp_state_machine::InMemoryBackend,
};
use revive_env::ExtBuilder;

use crate::executor::{
    context::ReviveExecutorStrategyContext, runner::ReviveExecutorStrategyRunner,
};

mod backend;
mod cheatcodes;
mod executor;
mod tracing;

pub use cheatcodes::{PvmCheatcodeInspectorStrategyBuilder, PvmStartupMigration};

/// Runtime backend mode for pallet-revive
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReviveRuntimeMode {
    /// Run PolkaVM backend on pallet-revive (PVM mode)
    Pvm,
    #[default]
    /// Run EVM backend on pallet-revive (EVM mode on Polkadot)
    Evm,
}

impl Display for ReviveRuntimeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pvm => write!(f, "PVM"),
            Self::Evm => write!(f, "EVM"),
        }
    }
}

/// Create Revive strategy for [ExecutorStrategy].
pub trait ReviveExecutorStrategyBuilder {
    /// Create new revive strategy.
    fn new_revive(runtime_mode: ReviveRuntimeMode) -> Self;
}

impl ReviveExecutorStrategyBuilder for ExecutorStrategy {
    fn new_revive(runtime_mode: ReviveRuntimeMode) -> Self {
        Self {
            runner: Box::leak(Box::new(ReviveExecutorStrategyRunner::new())),
            context: Box::new(ReviveExecutorStrategyContext::new(runtime_mode)),
        }
    }
}

// TODO: rewrite this to something proper rather than a thread local variable
std::thread_local! {
    pub static TEST_EXTERNALITIES: std::cell::RefCell<sp_io::TestExternalities> = std::cell::RefCell::new(ExtBuilder::default()
    .balance_genesis_config(vec![(H160::from_low_u64_be(1), 1000)])
    .build());

    pub static CHECKPOINT : std::cell::RefCell<InMemoryBackend<sp_core::Blake2Hasher> > = panic!("not set");
}

fn execute_with_externalities<R, F: FnOnce(&mut sp_io::TestExternalities) -> R>(f: F) -> R {
    TEST_EXTERNALITIES.with_borrow_mut(f)
}

pub fn with_externalities<R, F: FnOnce() -> R>(mut backend: Backend, f: F) -> R {
    let mut test_externalities = ExtBuilder::default().build();
    std::mem::swap(&mut test_externalities.backend, &mut backend.0);
    TEST_EXTERNALITIES.set(test_externalities);
    f()
}

fn save_checkpoint() {
    TEST_EXTERNALITIES.with_borrow_mut(|f| CHECKPOINT.set(f.as_backend()))
}

fn return_to_checkpoint() {
    let mut test_externalities = ExtBuilder::default().build();
    let mut backend = CHECKPOINT.take();
    std::mem::swap(&mut test_externalities.backend, &mut backend);

    TEST_EXTERNALITIES.set(test_externalities)
}

#[derive(Clone)]
pub struct Backend(InMemoryBackend<sp_core::Blake2Hasher>);

impl Backend {
    /// Get the backend of test_externalities
    pub fn get() -> Self {
        TEST_EXTERNALITIES.with_borrow_mut(|f| Self(f.as_backend()))
    }
}
