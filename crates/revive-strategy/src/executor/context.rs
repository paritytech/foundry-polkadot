use foundry_evm::executors::ExecutorStrategyContext;

/// Defines the context for [crate::ReviveExecutorStrategyRunner].
#[derive(Debug, Default, Clone)]
pub struct ReviveExecutorStrategyContext {
    // todo!()
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
