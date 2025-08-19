use std::{
    any::{Any, TypeId},
    fmt::Debug,
    sync::{Arc, Mutex},
};

use alloy_primitives::U256;
use foundry_common::sh_err;
use revive_env::{AccountId, Runtime};
use revm::{
    interpreter::{CallInputs, Interpreter},
    primitives::SignedAuthorization,
};

use foundry_cheatcodes::{
    Broadcast, BroadcastableTransactions, CheatcodeInspectorStrategy,
    CheatcodeInspectorStrategyContext, CheatcodeInspectorStrategyRunner, CheatsConfig, CheatsCtxt,
    CommonCreateInput, Ecx, EvmCheatcodeInspectorStrategyRunner, InnerEcx, Result, Vm::pvmCall,
};

use polkadot_sdk::{
    frame_support::traits::fungible::Mutate,
    pallet_balances,
    pallet_revive::AddressMapper,
    polkadot_runtime_common::U256ToBalance,
    sp_core::{self, H160},
    sp_io,
    sp_runtime::traits::Convert,
};

pub trait PvmCheatcodeInspectorStrategyBuilder {
    fn new_pvm(test_externalities: Arc<Mutex<sp_io::TestExternalities>>) -> Self;
}
impl PvmCheatcodeInspectorStrategyBuilder for CheatcodeInspectorStrategy {
    // Creates a new PVM strategy
    fn new_pvm(test_externalities: Arc<Mutex<sp_io::TestExternalities>>) -> Self {
        Self {
            runner: &PvmCheatcodeInspectorStrategyRunner,
            context: Box::new(PvmCheatcodeInspectorStrategyContext::new(test_externalities)),
        }
    }
}

/// PVM-specific strategy context.
#[derive(Debug, Default, Clone)]
pub struct PvmCheatcodeInspectorStrategyContext {
    /// Whether we're using PVM mode
    /// Currently unused but kept for future PVM-specific logic
    pub using_pvm: bool,
    pub revive_test_externalities: Arc<Mutex<sp_io::TestExternalities>>,
}

impl PvmCheatcodeInspectorStrategyContext {
    pub fn new(test_externalities: Arc<Mutex<sp_io::TestExternalities>>) -> Self {
        Self {
            using_pvm: false, // Start in EVM mode by default
            revive_test_externalities: test_externalities,
        }
    }
}

impl CheatcodeInspectorStrategyContext for PvmCheatcodeInspectorStrategyContext {
    fn new_cloned(&self) -> Box<dyn CheatcodeInspectorStrategyContext> {
        Box::new(self.clone())
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }
}

/// Implements [CheatcodeInspectorStrategyRunner] for PVM.
#[derive(Debug, Default, Clone)]
pub struct PvmCheatcodeInspectorStrategyRunner;

impl CheatcodeInspectorStrategyRunner for PvmCheatcodeInspectorStrategyRunner {
    fn apply_full(
        &self,
        cheatcode: &dyn foundry_cheatcodes::DynCheatcode,
        ccx: &mut CheatsCtxt<'_, '_, '_, '_>,
        executor: &mut dyn foundry_cheatcodes::CheatcodesExecutor,
    ) -> Result {
        fn is<T: std::any::Any>(t: TypeId) -> bool {
            TypeId::of::<T>() == t
        }

        match cheatcode.as_any().type_id() {
            t if is::<pvmCall>(t) => {
                let pvmCall { enabled } = cheatcode.as_any().downcast_ref().unwrap();
                if *enabled {
                    let ctx = get_context(ccx.state.strategy.context.as_mut());
                    select_pvm(ctx, ccx.ecx);
                } else {
                    todo!("Switch back to EVM");
                }

                Ok(Default::default())
            }
            // Not custom, just invoke the default behavior
            _ => cheatcode.dyn_apply(ccx, executor),
        }
    }

    fn base_contract_deployed(&self, _ctx: &mut dyn CheatcodeInspectorStrategyContext) {
        // PVM mode is enabled, but no special handling needed for now
        // Only intercept PVM-specific calls when needed in future implementations
    }

    fn record_broadcastable_create_transactions(
        &self,
        _ctx: &mut dyn CheatcodeInspectorStrategyContext,
        config: Arc<CheatsConfig>,
        input: &dyn CommonCreateInput,
        ecx_inner: InnerEcx<'_, '_, '_>,
        broadcast: &Broadcast,
        broadcastable_transactions: &mut BroadcastableTransactions,
    ) {
        // Use EVM implementation for now
        // Only intercept PVM-specific calls when needed in future implementations
        EvmCheatcodeInspectorStrategyRunner.record_broadcastable_create_transactions(
            _ctx,
            config,
            input,
            ecx_inner,
            broadcast,
            broadcastable_transactions,
        );
    }

    fn record_broadcastable_call_transactions(
        &self,
        _ctx: &mut dyn CheatcodeInspectorStrategyContext,
        config: Arc<CheatsConfig>,
        call: &CallInputs,
        ecx_inner: InnerEcx<'_, '_, '_>,
        broadcast: &Broadcast,
        broadcastable_transactions: &mut BroadcastableTransactions,
        active_delegation: &mut Option<SignedAuthorization>,
    ) {
        // Use EVM implementation for now
        // Only intercept PVM-specific calls when needed in future implementations
        EvmCheatcodeInspectorStrategyRunner.record_broadcastable_call_transactions(
            _ctx,
            config,
            call,
            ecx_inner,
            broadcast,
            broadcastable_transactions,
            active_delegation,
        );
    }

    fn post_initialize_interp(
        &self,
        _ctx: &mut dyn CheatcodeInspectorStrategyContext,
        _interpreter: &mut Interpreter,
        _ecx: Ecx<'_, '_, '_>,
    ) {
        // PVM mode is enabled, but no special initialization needed for now
        // Only intercept PVM-specific calls when needed in future implementations
    }

    fn pre_step_end(
        &self,
        _ctx: &mut dyn CheatcodeInspectorStrategyContext,
        _interpreter: &mut Interpreter,
        _ecx: Ecx<'_, '_, '_>,
    ) -> bool {
        // No PVM-specific opcode handling needed for now
        // Only intercept PVM-specific calls when needed in future implementations
        false // Let EVM handle all operations
    }
}

fn select_pvm(ctx: &mut PvmCheatcodeInspectorStrategyContext, data: InnerEcx<'_, '_, '_>) {
    if ctx.using_pvm {
        tracing::info!("already in PVM");
        return;
    }

    tracing::info!("switching to PVM");
    ctx.using_pvm = true;
    let persistent_accounts = data.db.persistent_accounts().clone();

    // Dirty Balance translation
    for address in persistent_accounts {
        let acc = data.load_account(address).expect("msg");
        let amount = acc.data.info.balance;

        let amount_pvm =
            sp_core::U256::from_little_endian(&amount.as_le_bytes()).min(u128::MAX.into());
        let amount_pvm = U256ToBalance::convert(amount_pvm);
        let amount_evm = U256::from(amount_pvm);
        if amount != amount_evm {
            let _ = sh_err!("Amount mismatch {amount} != {amount_evm}, Polkadot balances are u128. Test results may be incorrect.");
        }

        ctx.revive_test_externalities.lock().unwrap().execute_with(|| {
            pallet_balances::Pallet::<Runtime>::set_balance(
                &AccountId::to_fallback_account_id(&H160::from_slice(address.as_slice())),
                amount_pvm,
            );
        })
    }
}

fn get_context(
    ctx: &mut dyn CheatcodeInspectorStrategyContext,
) -> &mut PvmCheatcodeInspectorStrategyContext {
    ctx.as_any_mut().downcast_mut().expect("expected PvmCheatcodeInspectorStrategyContext")
}
