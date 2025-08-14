use alloy_primitives::{Address, U256};
use foundry_cheatcodes::CheatcodeInspectorStrategy;
use foundry_evm::{
    backend::BackendStrategy,
    executors::{EvmExecutorStrategyRunner, ExecutorStrategyContext, ExecutorStrategyRunner},
};
use polkadot_sdk::{
    frame_support::traits::fungible::Mutate,
    pallet_balances,
    pallet_revive::AddressMapper,
    polkadot_runtime_common::U256ToBalance,
    sp_core::{self, H160},
    sp_runtime::traits::Convert,
};
use revive_env::{AccountId, Runtime, System};
use revm::primitives::{EnvWithHandlerCfg, ResultAndState};

use crate::{
    backend::{get_backend_ref, ReviveBackendStrategyBuilder},
    executor::context::ReviveExecutorStrategyContext,
};

/// Defines the [ExecutorStrategyRunner] strategy for Revive.
#[derive(Debug, Default, Clone)]
pub struct ReviveExecutorStrategyRunner;

impl ExecutorStrategyRunner for ReviveExecutorStrategyRunner {
    fn new_backend_strategy(
        &self,
        _ctx: &dyn foundry_evm::executors::ExecutorStrategyContext,
    ) -> foundry_evm::backend::BackendStrategy {
        BackendStrategy::new_revive()
    }

    fn new_cheatcodes_strategy(
        &self,
        ctx: &dyn foundry_evm::executors::ExecutorStrategyContext,
    ) -> foundry_cheatcodes::CheatcodesStrategy {
        let _ctx = get_context_ref(ctx);
        // todo!("Context should be used to configure the cheatcodes strategy");
        CheatcodeInspectorStrategy::new_pvm()
    }

    fn set_balance(
        &self,
        executor: &mut foundry_evm::executors::Executor,
        address: Address,
        amount: U256,
    ) -> foundry_evm::backend::BackendResult<()> {
        // todo!(): This should be done in client places, this is just a workaround for now, because
        // ethereum supports U256::MAX while polkadot does not.
        let amount = if amount == U256::MAX { U256::from(u128::MAX) } else { amount };
        EvmExecutorStrategyRunner.set_balance(executor, address, amount)?;

        let backend = get_backend_ref(executor.backend().strategy.context.as_ref());
        let mut ext = backend.revive_test_externalities.lock().unwrap();

        // todo!(), not sure this is the right call, with the right conversion
        let amount = sp_core::U256::from_little_endian(&amount.as_le_bytes());
        let amount = U256ToBalance::convert(amount);
        ext.execute_with(|| {
            pallet_balances::Pallet::<Runtime>::set_balance(
                &AccountId::to_fallback_account_id(&H160::from_slice(address.as_slice())),
                amount,
            );
        });
        Ok(())
    }

    fn get_balance(
        &self,
        executor: &foundry_evm::executors::Executor,
        address: Address,
    ) -> foundry_evm::backend::BackendResult<U256> {
        let evm_balance = EvmExecutorStrategyRunner.get_balance(executor, address)?;

        let backend = get_backend_ref(executor.backend().strategy.context.as_ref());
        let mut ext = backend.revive_test_externalities.lock().unwrap();
        let balance = ext.execute_with(|| {
            pallet_balances::Pallet::<Runtime>::free_balance(AccountId::to_fallback_account_id(
                &H160::from_slice(address.as_slice()),
            ))
        });
        assert_eq!(evm_balance, U256::from(balance));
        Ok(evm_balance)
    }

    fn set_nonce(
        &self,
        executor: &mut foundry_evm::executors::Executor,
        address: Address,
        nonce: u64,
    ) -> foundry_evm::backend::BackendResult<()> {
        EvmExecutorStrategyRunner.set_nonce(executor, address, nonce)?;
        let backend = get_backend_ref(executor.backend().strategy.context.as_ref());
        let mut ext = backend.revive_test_externalities.lock().unwrap();
        ext.execute_with(|| {
            let current_nonce = System::account_nonce(AccountId::to_fallback_account_id(
                &H160::from_slice(address.as_slice()),
            ));

            if current_nonce as u64 > nonce {
                todo!("Cannot set nonce lower than current nonce");
            }

            while (System::account_nonce(AccountId::to_fallback_account_id(&H160::from_slice(
                address.as_slice(),
            ))) as u64)
                < nonce
            {
                System::inc_account_nonce(AccountId::to_fallback_account_id(&H160::from_slice(
                    address.as_slice(),
                )));
            }
        });
        Ok(())
    }

    fn get_nonce(
        &self,
        executor: &foundry_evm::executors::Executor,
        address: Address,
    ) -> foundry_evm::backend::BackendResult<u64> {
        let evm_nonce = EvmExecutorStrategyRunner.get_nonce(executor, address)?;
        let backend = get_backend_ref(executor.backend().strategy.context.as_ref());
        let mut ext = backend.revive_test_externalities.lock().unwrap();
        let revive_nonce = ext.execute_with(|| {
            System::account_nonce(AccountId::to_fallback_account_id(&H160::from_slice(
                address.as_slice(),
            )))
        });

        assert_eq!(evm_nonce, revive_nonce as u64,);
        Ok(evm_nonce)
    }

    fn call(
        &self,
        ctx: &dyn foundry_evm::executors::ExecutorStrategyContext,
        backend: &mut foundry_evm::backend::CowBackend<'_>,
        env: &mut EnvWithHandlerCfg,
        executor_env: &EnvWithHandlerCfg,
        inspector: &mut foundry_evm::inspectors::InspectorStack,
    ) -> eyre::Result<ResultAndState> {
        // todo!(): Needs to decide if it should use revive depending on the context.
        EvmExecutorStrategyRunner.call(ctx, backend, env, executor_env, inspector)
    }

    fn transact(
        &self,
        ctx: &mut dyn foundry_evm::executors::ExecutorStrategyContext,
        backend: &mut foundry_evm::backend::Backend,
        env: &mut EnvWithHandlerCfg,
        executor_env: &EnvWithHandlerCfg,
        inspector: &mut foundry_evm::inspectors::InspectorStack,
    ) -> eyre::Result<ResultAndState> {
        // todo!(): Needs to decide if it should use revive depending on the context.
        EvmExecutorStrategyRunner.transact(ctx, backend, env, executor_env, inspector)
    }
}

fn get_context_ref(ctx: &dyn ExecutorStrategyContext) -> &ReviveExecutorStrategyContext {
    ctx.as_any_ref().downcast_ref().expect("expected ReviveExecutorStrategyContext")
}
