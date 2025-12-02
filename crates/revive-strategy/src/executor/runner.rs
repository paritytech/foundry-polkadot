use alloy_primitives::{Address, U256};
use foundry_cheatcodes::CheatcodeInspectorStrategy;
use foundry_compilers::{
    ProjectCompileOutput, compilers::resolc::dual_compiled_contracts::DualCompiledContracts,
};
use foundry_evm::{
    Env,
    backend::BackendStrategy,
    executors::{
        EvmExecutorStrategyRunner, ExecutorStrategyContext, ExecutorStrategyRunner,
        strategy::ExecutorStrategyExt,
    },
};
use polkadot_sdk::sp_externalities::Externalities;
use revm::context::result::ResultAndState;

use crate::{
    backend::ReviveBackendStrategyBuilder, cheatcodes::PvmCheatcodeInspectorStrategyBuilder,
    executor::context::ReviveExecutorStrategyContext,
};

/// Defines the [ExecutorStrategyRunner] strategy for Revive.
#[derive(Debug, Default, Clone)]
pub struct ReviveExecutorStrategyRunner;

impl ReviveExecutorStrategyRunner {
    pub fn new() -> Self {
        Self
    }
}

impl ExecutorStrategyRunner for ReviveExecutorStrategyRunner {
    fn new_backend_strategy(&self, _ctx: &dyn ExecutorStrategyContext) -> BackendStrategy {
        BackendStrategy::new_revive()
    }

    fn new_cheatcodes_strategy(
        &self,
        ctx: &dyn ExecutorStrategyContext,
    ) -> foundry_cheatcodes::CheatcodesStrategy {
        let ctx = get_context_ref(ctx);
        CheatcodeInspectorStrategy::new_pvm(
            ctx.dual_compiled_contracts.clone(),
            ctx.runtime_mode,
            ctx.externalties.shallow_clone(),
        )
    }

    /// Sets the balance of an account.
    ///
    /// Amount should be in the range of [0, u128::MAX] despite the type
    /// because Ethereum balances are u256 while Polkadot balances are u128.
    fn set_balance(
        &self,
        executor: &mut foundry_evm::executors::Executor,
        address: Address,
        amount: U256,
    ) -> foundry_evm::backend::BackendResult<()> {
        EvmExecutorStrategyRunner.set_balance(executor, address, amount)?;

        let ctx = get_context_ref_mut(executor.strategy.context.as_mut());

        ctx.externalties.set_balance(address, amount);
        Ok(())
    }

    fn get_balance(
        &self,
        executor: &mut foundry_evm::executors::Executor,
        address: Address,
    ) -> foundry_evm::backend::BackendResult<U256> {
        let evm_balance = EvmExecutorStrategyRunner.get_balance(executor, address)?;
        let ctx = get_context_ref_mut(executor.strategy.context.as_mut());

        let revive_balance = ctx.externalties.get_balance(address);
        assert_eq!(evm_balance, revive_balance);
        Ok(evm_balance)
    }

    fn set_nonce(
        &self,
        executor: &mut foundry_evm::executors::Executor,
        address: Address,
        nonce: u64,
    ) -> foundry_evm::backend::BackendResult<()> {
        EvmExecutorStrategyRunner.set_nonce(executor, address, nonce)?;
        let ctx = get_context_ref_mut(executor.strategy.context.as_mut());
        ctx.externalties.set_nonce(address, nonce);
        Ok(())
    }

    fn get_nonce(
        &self,
        executor: &mut foundry_evm::executors::Executor,
        address: Address,
    ) -> foundry_evm::backend::BackendResult<u64> {
        let evm_nonce = EvmExecutorStrategyRunner.get_nonce(executor, address)?;
        let ctx = get_context_ref_mut(executor.strategy.context.as_mut());

        let revive_nonce = ctx.externalties.get_nonce(address);

        assert_eq!(evm_nonce, revive_nonce as u64);
        Ok(evm_nonce)
    }

    fn call(
        &self,
        ctx: &dyn ExecutorStrategyContext,
        backend: &mut foundry_evm::backend::CowBackend<'_>,
        env: &mut Env,
        executor_env: &Env,
        inspector: &mut foundry_evm::inspectors::InspectorStack,
    ) -> eyre::Result<ResultAndState> {
        EvmExecutorStrategyRunner.call(ctx, backend, env, executor_env, inspector)
    }

    fn transact(
        &self,
        ctx: &mut dyn ExecutorStrategyContext,
        backend: &mut foundry_evm::backend::Backend,
        env: &mut Env,
        executor_env: &Env,
        inspector: &mut foundry_evm::inspectors::InspectorStack,
    ) -> eyre::Result<ResultAndState> {
        EvmExecutorStrategyRunner.transact(ctx, backend, env, executor_env, inspector)
    }
}

fn get_context_ref(ctx: &dyn ExecutorStrategyContext) -> &ReviveExecutorStrategyContext {
    ctx.as_any_ref().downcast_ref().expect("expected ReviveExecutorStrategyContext")
}

fn get_context_ref_mut(
    ctx: &mut dyn ExecutorStrategyContext,
) -> &mut ReviveExecutorStrategyContext {
    ctx.as_any_mut().downcast_mut().expect("expected ReviveExecutorStrategyContext")
}

impl ExecutorStrategyExt for ReviveExecutorStrategyRunner {
    fn revive_set_dual_compiled_contracts(
        &self,
        ctx: &mut dyn ExecutorStrategyContext,
        dual_compiled_contracts: DualCompiledContracts,
    ) {
        let ctx = get_context_ref_mut(ctx);
        ctx.dual_compiled_contracts = dual_compiled_contracts;
    }

    fn revive_set_compilation_output(
        &self,
        ctx: &mut dyn ExecutorStrategyContext,
        output: ProjectCompileOutput,
    ) {
        let ctx = get_context_ref_mut(ctx);
        ctx.compilation_output.replace(output);
    }
    fn start_transaction(&self, ctx: &dyn ExecutorStrategyContext) {
        let ctx = get_context_ref(ctx);
        let mut externalities = ctx.externalties.0.lock().unwrap();
        externalities.externalities.ext().storage_start_transaction();
    }

    fn rollback_transaction(&self, ctx: &dyn ExecutorStrategyContext) {
        let ctx = get_context_ref(ctx);
        let mut state = ctx.externalties.0.lock().unwrap();
        let _ = state.externalities.ext().storage_rollback_transaction();
    }
}
