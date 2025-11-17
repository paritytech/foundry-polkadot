//! This crate provides the Revive strategy for the Foundry EVM ExecutorStrategy.
//!
//! It is designed to work with the Revive runtime, allowing for the execution of smart contracts
//! in a Polkadot environment.
//!
//! It is heavily inspired from <https://github.com/matter-labs/foundry-zksync/tree/main/crates/strategy/zksync>
use std::fmt::Display;

use foundry_evm::executors::ExecutorStrategy;

use crate::executor::{
    context::ReviveExecutorStrategyContext, runner::ReviveExecutorStrategyRunner,
};

mod backend;
mod cheatcodes;
mod executor;
mod state;
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
