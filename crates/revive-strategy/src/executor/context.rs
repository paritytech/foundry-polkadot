use foundry_compilers::{
    ProjectCompileOutput, resolc::dual_compiled_contracts::DualCompiledContracts,
};
use foundry_evm::executors::ExecutorStrategyContext;

use crate::{ReviveRuntimeMode, state::TestEnv};

/// Defines the context for [crate::ReviveExecutorStrategyRunner].
#[derive(Debug, Default, Clone)]
pub struct ReviveExecutorStrategyContext {
    /// Runtime backend mode (PVM or EVM on Polkadot)
    pub(crate) runtime_mode: ReviveRuntimeMode,
    /// Dual compiled contracts.
    pub(crate) dual_compiled_contracts: DualCompiledContracts,
    /// Compilation output.
    pub(crate) compilation_output: Option<ProjectCompileOutput>,
    pub(crate) externalties: TestEnv,
}

impl ReviveExecutorStrategyContext {
    pub fn new(runtime_mode: ReviveRuntimeMode) -> Self {
        Self { runtime_mode, ..Default::default() }
    }
}

impl ExecutorStrategyContext for ReviveExecutorStrategyContext {
    fn new_cloned(&self) -> Box<dyn ExecutorStrategyContext> {
        Box::new(self.clone())
    }

    fn as_any_ref(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
