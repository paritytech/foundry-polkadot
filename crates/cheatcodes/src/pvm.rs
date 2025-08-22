use std::{any::Any, fmt::Debug, sync::Arc};

use revm::{
    interpreter::{CallInputs, Interpreter},
    primitives::{CreateScheme, SignedAuthorization},
};

use crate::{
    inspector::{CommonCreateInput, Ecx, InnerEcx},
    script::Broadcast,
    strategy::{
        CheatcodeInspectorStrategyContext, CheatcodeInspectorStrategyRunner,
        EvmCheatcodeInspectorStrategyRunner,
    },
    BroadcastableTransactions, CheatsConfig,
};

/// PVM-specific strategy context.
#[derive(Debug, Default, Clone)]
pub struct PvmCheatcodeInspectorStrategyContext {
    /// Whether we're using PVM mode
    /// Currently unused but kept for future PVM-specific logic
    #[allow(dead_code)]
    pub using_pvm: bool,
}

impl PvmCheatcodeInspectorStrategyContext {
    pub fn new() -> Self {
        Self {
            using_pvm: false, // Start in EVM mode by default
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
    fn base_contract_deployed(&self, _ctx: &mut dyn CheatcodeInspectorStrategyContext) {
        // PVM mode is enabled, but no special handling needed for now
        // Only intercept PVM-specific calls when needed in future implementations
    }

    fn record_broadcastable_create_transactions(
        &self,
        _ctx: &mut dyn CheatcodeInspectorStrategyContext,
        config: Arc<CheatsConfig>,
        input: &dyn CommonCreateInput,
        ecx_inner: InnerEcx,
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
        ecx_inner: InnerEcx,
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
        _ecx: Ecx,
    ) {
        // PVM mode is enabled, but no special initialization needed for now
        // Only intercept PVM-specific calls when needed in future implementations
    }

    fn pre_step_end(
        &self,
        _ctx: &mut dyn CheatcodeInspectorStrategyContext,
        _interpreter: &mut Interpreter,
        _ecx: Ecx,
    ) -> bool {
        // No PVM-specific opcode handling needed for now
        // Only intercept PVM-specific calls when needed in future implementations
        false // Let EVM handle all operations
    }
}

impl crate::strategy::CheatcodeInspectorStrategyExt for PvmCheatcodeInspectorStrategyRunner {
    /// Try handling the `CREATE` within PVM.
    ///
    /// If `Some` is returned then the result must be returned immediately, else the call must be
    /// handled in EVM.
    fn revive_try_create(
        &self,
        state: &mut crate::Cheatcodes,
        ecx: InnerEcx,
        input: &dyn CommonCreateInput,
        _executor: &mut dyn crate::CheatcodesExecutor,
    ) -> Option<revm::interpreter::CreateOutcome> {
        let ctx = get_context_ref_mut(state.strategy.context.as_mut());

        if !ctx.using_pvm {
            return None;
        }

        if let Some(CreateScheme::Create) = input.scheme() {
            let caller = input.caller();
            let nonce =
                ecx.load_account(input.caller()).expect("to load caller account").info.nonce;
            let address = caller.create(nonce);
            if ecx.db.get_test_contract_address().map(|addr| address == addr).unwrap_or_default() {
                info!("running create in EVM, instead of PVM (Test Contract) {:#?}", address);
                return None;
            }
        }

        None
    }
}

fn get_context_ref_mut(
    ctx: &mut dyn CheatcodeInspectorStrategyContext,
) -> &mut PvmCheatcodeInspectorStrategyContext {
    ctx.as_any_mut().downcast_mut().expect("expected PvmCheatcodeInspectorStrategyContext")
}
