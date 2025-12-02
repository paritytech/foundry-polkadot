mod mock_handler;

use alloy_primitives::{Address, B256, Bytes, Log, hex, ruint::aliases::U256};
use alloy_rpc_types::BlobTransactionSidecar;
use alloy_sol_types::SolValue;
use foundry_cheatcodes::{
    Broadcast, BroadcastableTransactions, CheatcodeInspectorStrategy,
    CheatcodeInspectorStrategyContext, CheatcodeInspectorStrategyRunner, CheatsConfig, CheatsCtxt,
    CommonCreateInput, Ecx, EvmCheatcodeInspectorStrategyRunner, Result,
    Vm::{
        chainIdCall, coinbaseCall, dealCall, etchCall, getNonce_0Call, loadCall, polkadotSkipCall,
        pvmCall, resetNonceCall, revertToStateAndDeleteCall, revertToStateCall, rollCall,
        setNonceCall, setNonceUnsafeCall, snapshotStateCall, storeCall, warpCall,
    },
    journaled_account, precompile_error,
};

use foundry_compilers::resolc::dual_compiled_contracts::DualCompiledContracts;
use foundry_evm::constants::CHEATCODE_ADDRESS;
use revive_env::{AccountId, Runtime, System, Timestamp};
use std::{
    any::{Any, TypeId},
    sync::Arc,
};
use tracing::warn;

use alloy_eips::eip7702::SignedAuthorization;
use polkadot_sdk::{
    pallet_revive::{
        AccountInfo, AddressMapper, BalanceOf, BytecodeType, Code, ContractInfo, DebugSettings,
        ExecConfig, Pallet, evm::CallTrace,
    },
    polkadot_sdk_frame::prelude::OriginFor,
    sp_core::{self, H160, H256},
    sp_io,
    sp_weights::Weight,
};

use crate::{
    cheatcodes::mock_handler::MockHandlerImpl,
    state::TestEnv,
    tracing::{Tracer, storage_tracer::AccountAccess},
};
use foundry_cheatcodes::Vm::{AccountAccess as FAccountAccess, ChainInfo};
use polkadot_sdk::pallet_revive::tracing::Tracing;

use revm::{
    bytecode::opcode as op,
    context::{CreateScheme, JournalTr},
    interpreter::{
        CallInputs, CallOutcome, CallScheme, CreateOutcome, Gas, InstructionResult, Interpreter,
        InterpreterResult, interpreter_types::Jumps,
    },
    state::Bytecode,
};
pub trait PvmCheatcodeInspectorStrategyBuilder {
    fn new_pvm(
        dual_compiled_contracts: DualCompiledContracts,
        runtime_mode: crate::ReviveRuntimeMode,
        externalities: TestEnv,
    ) -> Self;
}
impl PvmCheatcodeInspectorStrategyBuilder for CheatcodeInspectorStrategy {
    // Creates a new PVM strategy
    fn new_pvm(
        dual_compiled_contracts: DualCompiledContracts,
        runtime_mode: crate::ReviveRuntimeMode,
        externalities: TestEnv,
    ) -> Self {
        Self {
            runner: &PvmCheatcodeInspectorStrategyRunner,
            context: Box::new(PvmCheatcodeInspectorStrategyContext::new(
                dual_compiled_contracts,
                runtime_mode,
                externalities,
            )),
        }
    }
}

/// Controls the automatic migration to pallet-revive during test execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PvmStartupMigration {
    /// Defer database migration to a later execution point.
    /// This is the initial state - waiting for the test contract to be deployed.
    Defer,
    /// Allow database migration to pallet-revive (EVM or PVM mode).
    /// Set by `base_contract_deployed()` when the test contract is deployed.
    #[default]
    Allow,
    /// Database migration has already been performed.
    /// Prevents redundant migrations.
    Done,
}

impl PvmStartupMigration {
    /// Check if startup migration is allowed
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow)
    }

    /// Allow migrating the database to PVM storage
    pub fn allow(&mut self) {
        *self = Self::Allow;
    }

    /// Mark the migration as completed
    pub fn done(&mut self) {
        *self = Self::Done;
    }
}
/// PVM-specific strategy context.
#[derive(Debug, Default, Clone)]
pub struct PvmCheatcodeInspectorStrategyContext {
    /// Whether we're currently using pallet-revive (migrated from REVM)
    pub using_pvm: bool,
    /// When in PVM context, execute the next CALL or CREATE in the EVM instead.
    pub skip_pvm: bool,
    /// Any contracts that were deployed in `skip_pvm` step.
    /// This makes it easier to dispatch calls to any of these addresses in PVM context, directly
    /// to EVM. Alternatively, we'd need to add `vm.polkadotSkip()` to these calls manually.
    pub skip_pvm_addresses: std::collections::HashSet<Address>,
    /// Records the next create address for `skip_pvm_addresses`.
    pub record_next_create_address: bool,
    /// Controls automatic migration to pallet-revive
    pub pvm_startup_migration: PvmStartupMigration,
    pub dual_compiled_contracts: DualCompiledContracts,
    /// Runtime backend mode when using pallet-revive (PVM or EVM)
    pub runtime_mode: crate::ReviveRuntimeMode,
    pub remove_recorded_access_at: Option<usize>,
    pub externalities: TestEnv,
}

impl PvmCheatcodeInspectorStrategyContext {
    pub fn new(
        dual_compiled_contracts: DualCompiledContracts,
        runtime_mode: crate::ReviveRuntimeMode,
        externalities: TestEnv,
    ) -> Self {
        Self {
            // Start in REVM mode by default
            using_pvm: false,
            skip_pvm: false,
            skip_pvm_addresses: Default::default(),
            record_next_create_address: Default::default(),
            // Will be set to Allow when test contract deploys
            pvm_startup_migration: PvmStartupMigration::Defer,
            dual_compiled_contracts,
            runtime_mode,
            remove_recorded_access_at: None,
            externalities,
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

impl PvmCheatcodeInspectorStrategyRunner {
    fn append_recorded_accesses(
        &self,
        state: &mut foundry_cheatcodes::Cheatcodes,
        ecx: Ecx<'_, '_, '_>,
        account_accesses: Vec<AccountAccess>,
    ) {
        if state.recording_accesses {
            for record in &account_accesses {
                for r in &record.storage_accesses {
                    if !r.isWrite {
                        state.accesses.record_read(
                            Address::from(record.account.0),
                            alloy_primitives::U256::from_be_slice(r.slot.clone().as_slice()),
                        );
                    } else {
                        state.accesses.record_write(
                            Address::from(record.account.0),
                            alloy_primitives::U256::from_be_slice(r.slot.clone().as_slice()),
                        );
                    }
                }
            }
        }

        if let Some(recorded_account_diffs_stack) = state.recorded_account_diffs_stack.as_mut() {
            // A duplicate entry is inserted on call/create start by the revm, and updated on
            // call/create end.
            //
            // If we are inside a nested call (stack depth > 1), the placeholder
            // lives in the *parent* frame.  Its index will be exactly the current
            // length of that parent vector (`len()`), so we record that length.
            //
            // If we are at the root (depth == 1), the placeholder is already the
            // last element of the root vector.  We therefore record `len() - 1`.
            //
            // `zksync_fix_recorded_accesses()` uses this index later to drop the
            // single duplicate.
            //
            // TODO(zk): This is currently a hack, as account access recording is
            // done in 4 parts - create/create_end and call/call_end. And these must all be
            // moved to strategy.

            let stack_insert_index = if recorded_account_diffs_stack.len() > 1 {
                recorded_account_diffs_stack
                    .get(recorded_account_diffs_stack.len() - 2)
                    .map_or(0, Vec::len)
            } else {
                // `len() - 1`
                recorded_account_diffs_stack.first().map_or(0, |v| v.len().saturating_sub(1))
            };

            if let Some(last) = recorded_account_diffs_stack.last_mut() {
                let ctx = get_context_ref_mut(state.strategy.context.as_mut());
                ctx.remove_recorded_access_at = Some(stack_insert_index);
                for record in account_accesses {
                    let access = FAccountAccess {
                        chainInfo: ChainInfo {
                            forkId: ecx
                                .journaled_state
                                .database
                                .active_fork_id()
                                .unwrap_or_default(),
                            chainId: U256::from(ecx.cfg.chain_id),
                        },
                        accessor: Address::from(record.accessor.0),
                        account: Address::from(record.account.0),
                        kind: record.kind,
                        initialized: true,
                        oldBalance: U256::from_limbs(record.old_balance.0),
                        newBalance: U256::from_limbs(record.new_balance.0),
                        value: U256::from_limbs(record.value.0),
                        data: record.data,
                        reverted: false,
                        deployedCode: if record.deployed_bytecode_hash.unwrap_or_default().is_zero()
                        {
                            Default::default()
                        } else {
                            Bytes::from(record.deployed_bytecode_hash.unwrap_or_default().0)
                        },
                        storageAccesses: record.storage_accesses,
                        depth: record.depth,
                    };
                    last.push(access);
                }
            }
        }
    }
}

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
        let ctx: &mut PvmCheatcodeInspectorStrategyContext =
            get_context_ref_mut(ccx.state.strategy.context.as_mut());
        let using_pvm = ctx.using_pvm;
        match cheatcode.as_any().type_id() {
            t if is::<pvmCall>(t) => {
                tracing::info!(cheatcode = ?cheatcode.as_debug() , using_pvm = ?using_pvm);
                let pvmCall { enabled } = cheatcode.as_any().downcast_ref().unwrap();
                let ctx: &mut PvmCheatcodeInspectorStrategyContext =
                    get_context_ref_mut(ccx.state.strategy.context.as_mut());
                if *enabled {
                    select_revive(ctx, ccx.ecx);
                } else {
                    select_evm(ctx, ccx.ecx);
                }
                Ok(Default::default())
            }
            t if is::<polkadotSkipCall>(t) => {
                let polkadotSkipCall { .. } = cheatcode.as_any().downcast_ref().unwrap();
                let ctx = get_context_ref_mut(ccx.state.strategy.context.as_mut());
                ctx.skip_pvm = true;
                Ok(Default::default())
            }
            t if using_pvm && is::<dealCall>(t) => {
                tracing::info!(cheatcode = ?cheatcode.as_debug() , using_pvm = ?using_pvm);
                let dealCall { account, newBalance } = cheatcode.as_any().downcast_ref().unwrap();

                ctx.externalities.set_balance(*account, *newBalance);
                cheatcode.dyn_apply(ccx, executor)
            }
            t if using_pvm && is::<setNonceCall>(t) => {
                tracing::info!(cheatcode = ?cheatcode.as_debug() , using_pvm = ?using_pvm);

                let &setNonceCall { account, newNonce } =
                    cheatcode.as_any().downcast_ref().unwrap();
                ctx.externalities.set_nonce(account, newNonce);

                cheatcode.dyn_apply(ccx, executor)
            }
            t if using_pvm && is::<setNonceUnsafeCall>(t) => {
                tracing::info!(cheatcode = ?cheatcode.as_debug() , using_pvm = ?using_pvm);

                let &setNonceUnsafeCall { account, newNonce } =
                    cheatcode.as_any().downcast_ref().unwrap();
                ctx.externalities.set_nonce(account, newNonce);

                cheatcode.dyn_apply(ccx, executor)
            }
            t if using_pvm && is::<resetNonceCall>(t) => {
                tracing::info!(cheatcode = ?cheatcode.as_debug() , using_pvm = ?using_pvm);
                let &resetNonceCall { account } = cheatcode.as_any().downcast_ref().unwrap();
                ctx.externalities.set_nonce(account, 0);
                cheatcode.dyn_apply(ccx, executor)
            }
            t if using_pvm && is::<getNonce_0Call>(t) => {
                tracing::info!(cheatcode = ?cheatcode.as_debug() , using_pvm = ?using_pvm);
                let &getNonce_0Call { account } = cheatcode.as_any().downcast_ref().unwrap();
                let ctx = get_context_ref_mut(ccx.state.strategy.context.as_mut());
                let nonce = ctx.externalities.get_nonce(account);
                Ok(u64::from(nonce).abi_encode())
            }
            t if using_pvm && is::<rollCall>(t) => {
                let &rollCall { newHeight } = cheatcode.as_any().downcast_ref().unwrap();

                ctx.externalities.set_block_number(newHeight);

                cheatcode.dyn_apply(ccx, executor)
            }
            t if using_pvm && is::<snapshotStateCall>(t) => {
                ctx.externalities.start_snapshotting();
                cheatcode.dyn_apply(ccx, executor)
            }
            t if using_pvm && is::<revertToStateAndDeleteCall>(t) => {
                let &revertToStateAndDeleteCall { snapshotId } =
                    cheatcode.as_any().downcast_ref().unwrap();

                ctx.externalities.revert(snapshotId.try_into().unwrap());
                cheatcode.dyn_apply(ccx, executor)
            }
            t if using_pvm && is::<revertToStateCall>(t) => {
                let &revertToStateCall { snapshotId } = cheatcode.as_any().downcast_ref().unwrap();

                ctx.externalities.revert(snapshotId.try_into().unwrap());
                cheatcode.dyn_apply(ccx, executor)
            }
            t if using_pvm && is::<warpCall>(t) => {
                let &warpCall { newTimestamp } = cheatcode.as_any().downcast_ref().unwrap();

                tracing::info!(cheatcode = ?cheatcode.as_debug() , using_pvm = ?using_pvm);
                ctx.externalities.set_timestamp(newTimestamp);

                cheatcode.dyn_apply(ccx, executor)
            }

            t if using_pvm && is::<chainIdCall>(t) => {
                let &chainIdCall { newChainId } = cheatcode.as_any().downcast_ref().unwrap();

                tracing::info!(cheatcode = ?cheatcode.as_debug() , using_pvm = ?using_pvm);
                ctx.externalities.set_chain_id(newChainId.to());

                cheatcode.dyn_apply(ccx, executor)
            }
            t if using_pvm && is::<coinbaseCall>(t) => {
                let &coinbaseCall { newCoinbase } = cheatcode.as_any().downcast_ref().unwrap();

                tracing::info!(cheatcode = ?cheatcode.as_debug() , using_pvm = ?using_pvm);
                ctx.externalities.set_block_author(newCoinbase);

                cheatcode.dyn_apply(ccx, executor)
            }
            t if using_pvm && is::<etchCall>(t) => {
                let etchCall { target, newRuntimeBytecode } =
                    cheatcode.as_any().downcast_ref().unwrap();
                let ctx = get_context_ref_mut(ccx.state.strategy.context.as_mut());

                ctx.externalities.etch_call(target, newRuntimeBytecode, ccx.ecx)?;
                Ok(Default::default())
            }

            t if is::<etchCall>(t) => {
                let etchCall { target, newRuntimeBytecode: _ } =
                    cheatcode.as_any().downcast_ref().unwrap();
                // Etch could be called from the test contract constructor, so we allow it
                // even if we're not yet using revive yet and mark the target as persistent, so
                // the bytecode gets persisted.
                ccx.ecx.journaled_state.database.add_persistent_account(*target);

                cheatcode.dyn_apply(ccx, executor)
            }
            t if using_pvm && is::<loadCall>(t) => {
                tracing::info!(cheatcode = ?cheatcode.as_debug() , using_pvm = ?using_pvm);
                let &loadCall { target, slot } = cheatcode.as_any().downcast_ref().unwrap();

                // Check if target is the test contract - if so, read from REVM state instead
                if ccx
                    .ecx
                    .journaled_state
                    .database
                    .get_test_contract_address()
                    .map(|addr| target == addr)
                    .unwrap_or_default()
                {
                    cheatcode.dyn_apply(ccx, executor)
                } else {
                    let ctx = get_context_ref_mut(ccx.state.strategy.context.as_mut());
                    let storage_value = ctx.externalities.get_storage(target, slot)?;
                    let result = storage_value.map(|b| B256::from_slice(&b)).unwrap_or(B256::ZERO);
                    Ok(result.abi_encode())
                }
            }
            t if using_pvm && is::<storeCall>(t) => {
                tracing::info!(cheatcode = ?cheatcode.as_debug() , using_pvm = ?using_pvm);
                let &storeCall { target, slot, value } = cheatcode.as_any().downcast_ref().unwrap();
                if ccx.is_precompile(&target) {
                    return Err(precompile_error(&target));
                }
                let ctx = get_context_ref_mut(ccx.state.strategy.context.as_mut());
                if target != CHEATCODE_ADDRESS {
                    ctx.externalities.set_storage(target, slot, value)?;
                }
                cheatcode.dyn_apply(ccx, executor)
            }
            // Not custom, just invoke the default behavior
            _ => cheatcode.dyn_apply(ccx, executor),
        }
    }

    fn base_contract_deployed(&self, ctx: &mut dyn CheatcodeInspectorStrategyContext) {
        let ctx = get_context_ref_mut(ctx);

        tracing::debug!("allowing startup PVM migration");
        ctx.pvm_startup_migration.allow();
    }

    fn record_broadcastable_create_transactions(
        &self,
        _ctx: &mut dyn CheatcodeInspectorStrategyContext,
        config: Arc<CheatsConfig>,
        input: &dyn CommonCreateInput,
        ecx_inner: Ecx<'_, '_, '_>,
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
        ecx_inner: Ecx<'_, '_, '_>,
        broadcast: &Broadcast,
        broadcastable_transactions: &mut BroadcastableTransactions,
        active_delegations: Vec<SignedAuthorization>,
        active_blob_sidecar: Option<BlobTransactionSidecar>,
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
            active_delegations,
            active_blob_sidecar,
        );
    }

    fn post_initialize_interp(
        &self,
        ctx: &mut dyn CheatcodeInspectorStrategyContext,
        _interpreter: &mut Interpreter,
        ecx: Ecx<'_, '_, '_>,
    ) {
        let ctx = get_context_ref_mut(ctx);

        if ctx.pvm_startup_migration.is_allowed() && !ctx.using_pvm {
            tracing::info!("startup pallet-revive migration initiated");
            select_revive(ctx, ecx);
            ctx.pvm_startup_migration.done();
            tracing::info!("startup pallet-revive migration completed");
        }
    }

    fn pre_step_end(
        &self,
        ctx: &mut dyn CheatcodeInspectorStrategyContext,
        interpreter: &mut Interpreter,
        _ecx: Ecx<'_, '_, '_>,
    ) -> bool {
        let ctx = get_context_ref_mut(ctx);

        if !ctx.using_pvm {
            return false;
        }

        let address = match interpreter.bytecode.opcode() {
            op::SELFBALANCE => interpreter.input.target_address,
            op::BALANCE => {
                if interpreter.stack.is_empty() {
                    return true;
                }

                Address::from_word(B256::from(unsafe { interpreter.stack.pop_unsafe() }))
            }
            _ => return true,
        };

        let balance = ctx.externalities.get_balance(address);
        tracing::info!(operation = "get_balance" , using_pvm = ?ctx.using_pvm, target = ?address, balance = ?balance);

        // Skip the current BALANCE instruction since we've already handled it
        if interpreter.stack.push(balance) {
            interpreter.bytecode.relative_jump(1);
        } else {
            // stack overflow; nothing else to do here
        }

        false // Let EVM handle all operations
    }
}

fn select_revive(ctx: &mut PvmCheatcodeInspectorStrategyContext, data: Ecx<'_, '_, '_>) {
    if ctx.using_pvm {
        tracing::info!("already using pallet-revive");
        return;
    }

    tracing::info!("switching to pallet-revive ({} mode)", ctx.runtime_mode);
    ctx.using_pvm = true;

    let block_number = data.block.number;
    let timestamp = data.block.timestamp;

    ctx.externalities.execute_with(||{
            // Enable debug mode to bypass EIP-170 size checks during testing
            if data.cfg.limit_contract_code_size == Some(usize::MAX) {
                let debug_settings = DebugSettings::new(true, true, true);
                debug_settings.write_to_storage::<Runtime>();
            }
            System::set_block_number(block_number.saturating_to());
            Timestamp::set_timestamp(timestamp.saturating_to::<u64>() * 1000);
            <revive_env::Runtime as polkadot_sdk::pallet_revive::Config>::ChainId::set(
                &data.cfg.chain_id,
            );
            let persistent_accounts = data.journaled_state.database.persistent_accounts().clone();
            for address in persistent_accounts.into_iter().chain([data.tx.caller]) {
                let acc = data.journaled_state.load_account(address).expect("failed to load account");
                let amount = acc.data.info.balance;
                let nonce = acc.data.info.nonce;
                let account = H160::from_slice(address.as_slice());
                let account_id =
                    AccountId::to_fallback_account_id(&account);
                let amount_pvm = sp_core::U256::from_little_endian(&amount.as_le_bytes()).min(u128::MAX.into());
                Pallet::<Runtime>::set_evm_balance(&account, amount_pvm)
                    .expect("failed to set evm balance");

                polkadot_sdk::frame_system::Account::<Runtime>::mutate(&account_id, |a| {
                    a.nonce = nonce.min(u32::MAX.into()).try_into().expect("shouldn't happen");
                });

                if let Some(bytecode) = acc.data.info.code.as_ref() {
                    let account_h160 = H160::from_slice(address.as_slice());

                    // Skip if contract already exists in pallet-revive
                    if AccountInfo::<Runtime>::load_contract(&account_h160).is_none() {
                        // Find the matching dual-compiled contract by EVM bytecode
                        if let Some((_, contract)) = ctx.dual_compiled_contracts
                            .find_by_evm_deployed_bytecode_with_immutables(bytecode.original_byte_slice())
                        {
                            let (code_bytes, immutable_data, code_type) = match ctx.runtime_mode {
                                crate::ReviveRuntimeMode::Pvm => {
                                    let immutable_data = contract.evm_immutable_references
                                        .as_ref()
                                        .map(|immutable_refs| {
                                            let evm_bytecode = bytecode.original_byte_slice();

                                            // Collect all immutable bytes from their scattered offsets
                                            immutable_refs
                                                .values().filter_map(|offsets| offsets.first())
                                                .flat_map(|offset| {
                                                    let start = offset.start as usize;
                                                    let end = start + offset.length as usize;
                                                    evm_bytecode.get(start..end).unwrap_or_else(|| panic!("Immutable offset out of bounds: address={:?}, offset={}..{}, bytecode_len={}",
                                                        address, start, end, evm_bytecode.len())).iter().rev()
                                                })
                                                .copied()
                                                .collect::<Vec<u8>>()
                                        });
                                    (contract.resolc_deployed_bytecode.as_bytes().map(|b| b.to_vec()),immutable_data, BytecodeType::Pvm)
                                },
                                crate::ReviveRuntimeMode::Evm => {
                                    (Some(bytecode.bytecode().to_vec()), None, BytecodeType::Evm)
                                },
                            };

                            if let Some(code_bytes) = code_bytes {
                                let upload_result = Pallet::<Runtime>::try_upload_code(
                                    Pallet::<Runtime>::account_id(),
                                    code_bytes.clone(),
                                    code_type,
                                    u64::MAX.into(),
                                    &ExecConfig::new_substrate_tx(),
                                );
                                match upload_result {
                                    Ok(_) => {
                                        let code_hash = H256(sp_io::hashing::keccak_256(&code_bytes));
                                        let contract_info = ContractInfo::<Runtime>::new(&account_h160, nonce as u32, code_hash)
                                            .expect("Failed to create contract info");
                                        AccountInfo::<Runtime>::insert_contract(&account_h160, contract_info);
                                        if let Some(data) = immutable_data.and_then(|immutables| immutables.try_into().ok())
                                        {
                                            Pallet::<Runtime>::set_immutables(account_h160, data).expect("Failed to migrate immutables");
                                        }
                                    }
                                    Err(err) => {
                                        tracing::warn!(
                                            address = ?address,
                                            runtime_mode = ?ctx.runtime_mode,
                                            bytecode_len = code_bytes.len(),
                                            error = ?err,
                                            "Failed to upload bytecode to pallet-revive, skipping migration"
                                        );
                                    }
                                }
                            } else {
                                tracing::info!(
                                    address = ?address,
                                    "no PVM equivalent found for EVM bytecode, skipping migration"
                                );
                            }
                        } else {
                            tracing::info!("Setting evm bytecode stored in account {:?} balance: {:?}", address, amount);
                            // Even if no dual-compiled contract is found, we still upload the existing bytecode because it might be some EVM bytecode that got etched earlier.
                            let code_bytes = bytecode.original_byte_slice().to_vec();
                            let upload_result = Pallet::<Runtime>::try_upload_code(
                                Pallet::<Runtime>::account_id(),
                                code_bytes.clone(),
                                BytecodeType::Evm,
                                u64::MAX.into(),
                                &ExecConfig::new_substrate_tx_without_bump(),
                            );
                            match upload_result {
                                Ok(_) => {
                                    let code_hash = H256(sp_io::hashing::keccak_256(&code_bytes));
                                    let contract_info = ContractInfo::<Runtime>::new(&account_h160, nonce as u32, code_hash)
                                        .expect("Failed to create contract info");
                                    AccountInfo::<Runtime>::insert_contract(&account_h160, contract_info);
                                }
                                Err(err) => {
                                    tracing::warn!(
                                        address = ?address,
                                        runtime_mode = ?ctx.runtime_mode,
                                        bytecode_len = code_bytes.len(),
                                        error = ?err,
                                        "Failed to upload bytecode to pallet-revive, skipping migration"
                                    );
                                }
                            }
                        }
                    }
                    if  AccountInfo::<Runtime>::load_contract(&account_h160).is_some() {
                           // Migrate complete account state (storage) for newly created/existing contract
                           for (slot, storage_slot) in &acc.data.storage {
                            let slot_bytes = slot.to_be_bytes::<32>();
                            let value_bytes = storage_slot.present_value.to_be_bytes::<32>();

                            if !storage_slot.present_value.is_zero() {
                                let _ = Pallet::<Runtime>::set_storage(
                                    account_h160,
                                    slot_bytes,
                                    Some(value_bytes.to_vec()),
                                );
                            }
                        }
                    }
                }
            }
        })
}

fn select_evm(ctx: &mut PvmCheatcodeInspectorStrategyContext, data: Ecx<'_, '_, '_>) {
    if !ctx.using_pvm {
        tracing::info!("already using REVM");
        return;
    }

    tracing::info!("switching from pallet-revive back to REVM");
    ctx.using_pvm = false;

    ctx.externalities.execute_with(|| {
        let block_number = System::block_number();
        let timestamp = Timestamp::get();

        data.block.number = U256::from(block_number);
        data.block.timestamp = U256::from(timestamp / 1000);

        let test_contract = data.journaled_state.database.get_test_contract_address();
        let persistent_accounts = data.journaled_state.database.persistent_accounts().clone();
        for address in persistent_accounts.into_iter().chain([data.tx.caller]) {
            let account_evm = H160::from_slice(address.as_slice());
            let pallet_evm_nonce = Pallet::<Runtime>::evm_nonce(&account_evm);
            let pallet_evm_balance = Pallet::<Runtime>::evm_balance(&account_evm);
            let amount_evm = U256::from_limbs(pallet_evm_balance.0);
            let account = journaled_account(data, address).expect("failed to load account");
            account.info.balance = amount_evm;
            account.info.nonce = pallet_evm_nonce as u64;

            // Migrate bytecode for deployed contracts (skip test contract)
            if test_contract != Some(address)
                && let Some(info) = AccountInfo::<Runtime>::load_contract(&account_evm)
            {
                let hash = hex::encode(info.code_hash);

                if let Some((code_hash, bytecode)) = match ctx.runtime_mode {
                    crate::ReviveRuntimeMode::Pvm => ctx
                        .dual_compiled_contracts
                        .find_by_resolc_bytecode_hash(hash)
                        .and_then(|(_, contract)| {
                            contract.evm_deployed_bytecode.as_bytes().map(|evm_bytecode| {
                                (
                                    contract.evm_bytecode_hash,
                                    Bytecode::new_raw(evm_bytecode.clone()),
                                )
                            })
                        }),
                    crate::ReviveRuntimeMode::Evm => ctx
                        .dual_compiled_contracts
                        .find_by_evm_bytecode_hash(hash)
                        .and_then(|(_, contract)| {
                            contract.evm_deployed_bytecode.as_bytes().map(|evm_bytecode| {
                                (
                                    contract.evm_bytecode_hash,
                                    Bytecode::new_raw(evm_bytecode.clone()),
                                )
                            })
                        }),
                } {
                    account.info.code_hash = code_hash;
                    account.info.code = Some(bytecode);
                } else {
                    tracing::info!(
                        address = ?address,
                        "no EVM equivalent found for PVM bytecode, skipping migration"
                    );
                }
            }
        }
    });
}

impl foundry_cheatcodes::CheatcodeInspectorStrategyExt for PvmCheatcodeInspectorStrategyRunner {
    fn is_pvm_enabled(&self, state: &mut foundry_cheatcodes::Cheatcodes) -> bool {
        let ctx = get_context_ref_mut(state.strategy.context.as_mut());

        ctx.using_pvm
    }

    /// Try handling the `CREATE` within PVM.
    ///
    /// If `Some` is returned then the result must be returned immediately, else the call must be
    /// handled in EVM.
    fn revive_try_create(
        &self,
        state: &mut foundry_cheatcodes::Cheatcodes,
        ecx: Ecx<'_, '_, '_>,
        input: &dyn CommonCreateInput,
        executor: &mut dyn foundry_cheatcodes::CheatcodesExecutor,
    ) -> Option<CreateOutcome> {
        let mock_handler =
            MockHandlerImpl::new(&ecx, &input.caller(), &ecx.tx.caller, None, None, state);

        let ctx: &mut PvmCheatcodeInspectorStrategyContext =
            get_context_ref_mut(state.strategy.context.as_mut());

        if !ctx.using_pvm {
            return None;
        }

        if ctx.skip_pvm {
            ctx.skip_pvm = false; // handled the skip, reset flag
            ctx.record_next_create_address = true;
            tracing::info!("running create in EVM, instead of pallet-revive (skipped)");
            return None;
        }

        if let Some(CreateScheme::Create) = input.scheme() {
            let caller = input.caller();
            let nonce = ecx
                .journaled_state
                .load_account(input.caller())
                .expect("to load caller account")
                .info
                .nonce;
            let address = caller.create(nonce);
            if ecx
                .journaled_state
                .database
                .get_test_contract_address()
                .map(|addr| address == addr)
                .unwrap_or_default()
            {
                tracing::info!(
                    "running create in EVM, instead of PVM (Test Contract) {:#?}",
                    address
                );
                return None;
            }
        }

        let init_code = input.init_code();

        // Determine which bytecode to use based on runtime mode
        let (code_bytes, constructor_args) = match ctx.runtime_mode {
            crate::ReviveRuntimeMode::Pvm => {
                // PVM mode: use resolc (PVM) bytecode
                tracing::info!("running create in PVM mode with PVM bytecode");
                let find_contract = ctx
                    .dual_compiled_contracts
                    .find_bytecode(&init_code.0)
                    .unwrap_or_else(|| panic!("failed finding contract for {init_code:?}"));
                let constructor_args = find_contract.constructor_args();
                let contract = find_contract.contract();
                (contract.resolc_bytecode.as_bytes().unwrap().to_vec(), constructor_args.to_vec())
            }
            crate::ReviveRuntimeMode::Evm => {
                // EVM mode: use EVM bytecode directly
                tracing::info!("running create in EVM mode with EVM bytecode");
                (init_code.0.to_vec(), vec![])
            }
        };

        let gas_price_pvm =
            sp_core::U256::from_little_endian(&U256::from(ecx.tx.gas_price).as_le_bytes());
        let mut tracer = Tracer::new(true);
        let caller_h160 = H160::from_slice(input.caller().as_slice());

        let res = ctx.externalities.execute_with(|| {
            tracer.watch_address(&caller_h160);

            tracer.trace(|| {
                let origin_account_id = AccountId::to_fallback_account_id(&caller_h160);
                let origin = OriginFor::<Runtime>::signed(origin_account_id.clone());
                let evm_value = sp_core::U256::from_little_endian(&input.value().as_le_bytes());
                mock_handler.fund_pranked_accounts(input.caller());
                System::inc_account_nonce(&origin_account_id);
                let code = Code::Upload(code_bytes.clone());
                let data = constructor_args;
                let salt = match input.scheme() {
                    Some(CreateScheme::Create2 { salt }) => Some(
                        salt.as_limbs()
                            .iter()
                            .flat_map(|&x| x.to_le_bytes())
                            .collect::<Vec<u8>>()
                            .try_into()
                            .unwrap(),
                    ),
                    _ => None,
                };

                let exec_config = ExecConfig {
                    // IMPORTANT: Do NOT bump nonce here!
                    // When calling bare_instantiate directly (not through dispatch), the nonce
                    // has NOT been incremented pre-dispatch. Setting bump_nonce=true would cause
                    // pallet-revive to increment the nonce AFTER computing the CREATE address,
                    // but the address computation subtracts 1 from the nonce assuming it was
                    // already incremented. This causes all deployments to use nonce-1 for
                    // address computation, resulting in duplicate addresses.
                    bump_nonce: false,
                    collect_deposit_from_hold: None,
                    effective_gas_price: Some(gas_price_pvm),
                    mock_handler: Some(Box::new(mock_handler.clone())),
                    is_dry_run: None,
                };

                Pallet::<Runtime>::bare_instantiate(
                    origin,
                    evm_value,
                    Weight::MAX,
                    // TODO: fixing.
                    BalanceOf::<Runtime>::MAX,
                    code,
                    data,
                    salt,
                    exec_config,
                )
            })
        });
        let mut gas = Gas::new(input.gas_limit());
        if res.result.as_ref().is_ok_and(|r| !r.result.did_revert()) {
            self.append_recorded_accesses(state, ecx, tracer.get_recorded_accesses());
        }
        post_exec(state, ecx, executor, &mut tracer, false);
        mock_handler.update_state_mocks(state);

        match &res.result {
            Ok(result) => {
                // Only record gas cost if gas metering is not paused.
                // When paused, the gas counter should remain frozen.
                if !state.gas_metering.paused {
                    let _ = gas.record_cost(res.gas_required.ref_time());
                }

                let outcome = if result.result.did_revert() {
                    CreateOutcome {
                        result: InterpreterResult {
                            result: InstructionResult::Revert,
                            output: result.result.data.clone().into(),
                            gas,
                        },
                        address: None,
                    }
                } else {
                    CreateOutcome {
                        result: InterpreterResult {
                            result: InstructionResult::Return,
                            output: code_bytes.into(),
                            gas,
                        },
                        address: Some(Address::from_slice(result.addr.as_bytes())),
                    }
                };

                Some(outcome)
            }
            Err(e) => {
                tracing::error!("Contract creation failed: {e:#?}");
                Some(CreateOutcome {
                    result: InterpreterResult {
                        result: InstructionResult::Revert,
                        output: Bytes::from_iter(
                            format!("Contract creation failed: {e:#?}").as_bytes(),
                        ),
                        gas,
                    },
                    address: None,
                })
            }
        }
    }

    /// Try handling the `CALL` within PVM.
    ///
    /// If `Some` is returned then the result must be returned immediately, else the call must be
    /// handled in EVM.
    fn revive_try_call(
        &self,
        state: &mut foundry_cheatcodes::Cheatcodes,
        ecx: Ecx<'_, '_, '_>,
        call: &CallInputs,
        executor: &mut dyn foundry_cheatcodes::CheatcodesExecutor,
    ) -> Option<CallOutcome> {
        let ctx = get_context_ref_mut(state.strategy.context.as_mut());
        let target_address = match call.scheme {
            CallScheme::DelegateCall => Some(call.target_address),
            _ => None,
        };

        if !ctx.using_pvm {
            return None;
        }

        if ctx.skip_pvm || ctx.skip_pvm_addresses.contains(&call.target_address) {
            ctx.skip_pvm = false; // handled the skip, reset flag
            tracing::info!("running call in EVM, instead of pallet-revive (skipped)");
            return None;
        }

        if ecx
            .journaled_state
            .database
            .get_test_contract_address()
            .map(|addr| call.bytecode_address == addr || call.target_address == addr)
            .unwrap_or_default()
        {
            tracing::info!(
                "running call in EVM, instead of pallet-revive (Test Contract) {:#?}",
                call.bytecode_address
            );
            return None;
        }

        tracing::info!("running call on pallet-revive with {} {:#?}", ctx.runtime_mode, call);

        let gas_price_pvm =
            sp_core::U256::from_little_endian(&U256::from(ecx.tx.gas_price).as_le_bytes());
        let mock_handler = MockHandlerImpl::new(
            &ecx,
            &call.caller,
            &ecx.tx.caller,
            target_address.as_ref(),
            Some(&call.bytecode_address),
            state,
        );

        let ctx = get_context_ref_mut(state.strategy.context.as_mut());

        // Get nonce before execute_with closure
        let should_bump_nonce = !call.is_static;
        let caller_h160 = H160::from_slice(call.caller.as_slice());

        let mut tracer = Tracer::new(true);
        let res = ctx.externalities.execute_with(|| {
            // Watch the caller's address so its nonce changes get tracked in prestate trace
            tracer.watch_address(&caller_h160);

            tracer.trace(|| {
                let origin =
                    OriginFor::<Runtime>::signed(AccountId::to_fallback_account_id(&caller_h160));
                mock_handler.fund_pranked_accounts(call.caller);

                let evm_value = sp_core::U256::from_little_endian(&call.call_value().as_le_bytes());
                let target = H160::from_slice(call.target_address.as_slice());
                let exec_config = ExecConfig {
                    bump_nonce: false, // only works for constructors
                    collect_deposit_from_hold: None,
                    effective_gas_price: Some(gas_price_pvm),
                    mock_handler: Some(Box::new(mock_handler.clone())),
                    is_dry_run: None,
                };
                if should_bump_nonce {
                    System::inc_account_nonce(AccountId::to_fallback_account_id(&caller_h160));
                }
                Pallet::<Runtime>::bare_call(
                    origin,
                    target,
                    evm_value,
                    Weight::MAX,
                    // TODO: fixing.
                    BalanceOf::<Runtime>::MAX,
                    call.input.bytes(ecx).to_vec(),
                    exec_config,
                )
            })
        });
        mock_handler.update_state_mocks(state);
        let mut gas = Gas::new(call.gas_limit);
        if res.result.as_ref().is_ok_and(|r| !r.did_revert()) {
            self.append_recorded_accesses(state, ecx, tracer.get_recorded_accesses());
        }
        post_exec(state, ecx, executor, &mut tracer, call.is_static);

        match res.result {
            Ok(result) => {
                // Only record gas cost if gas metering is not paused.
                // When paused, the gas counter should remain frozen.
                if !state.gas_metering.paused {
                    let _ = gas.record_cost(res.gas_required.ref_time());
                }

                let outcome = if result.did_revert() {
                    tracing::info!("Contract call reverted");
                    CallOutcome {
                        result: InterpreterResult {
                            result: InstructionResult::Revert,
                            output: result.data.into(),
                            gas,
                        },
                        memory_offset: call.return_memory_offset.clone(),
                    }
                } else if result.data.is_empty() {
                    CallOutcome {
                        result: InterpreterResult {
                            result: InstructionResult::Stop,
                            output: result.data.into(),
                            gas,
                        },
                        memory_offset: call.return_memory_offset.clone(),
                    }
                } else {
                    CallOutcome {
                        result: InterpreterResult {
                            result: InstructionResult::Return,
                            output: result.data.into(),
                            gas,
                        },
                        memory_offset: call.return_memory_offset.clone(),
                    }
                };

                Some(outcome)
            }
            Err(e) => {
                tracing::error!("Contract call failed: {e:#?}");
                Some(CallOutcome {
                    result: InterpreterResult {
                        result: InstructionResult::Revert,
                        output: Bytes::from_iter(
                            format!("Contract call failed: {e:#?}").as_bytes(),
                        ),
                        gas,
                    },
                    memory_offset: call.return_memory_offset.clone(),
                })
            }
        }
    }

    fn revive_remove_duplicate_account_access(&self, state: &mut foundry_cheatcodes::Cheatcodes) {
        let ctx = get_context_ref_mut(state.strategy.context.as_mut());

        if let Some(index) = ctx.remove_recorded_access_at.take()
            && let Some(recorded_account_diffs_stack) = state.recorded_account_diffs_stack.as_mut()
            && let Some(last) = recorded_account_diffs_stack.last_mut()
        {
            // This entry has been inserted during CREATE/CALL operations in revm's
            // cheatcode inspector and must be removed.
            if index < last.len() {
                let _ = last.remove(index);
            } else {
                warn!(index, len = last.len(), "skipping duplicate access removal: out of bounds");
            }
        }
    }
    fn revive_call_end(
        &self,
        state: &mut foundry_cheatcodes::Cheatcodes,
        ecx: Ecx<'_, '_, '_>,
        call: &CallInputs,
    ) {
        let ctx = get_context_ref_mut(state.strategy.context.as_mut());

        // Skip storage sync if: in PVM mode AND no test contract
        if ctx.using_pvm
            && ecx
                .journaled_state
                .database
                .get_test_contract_address()
                .map(|addr| call.bytecode_address != addr && call.target_address != addr)
                .unwrap_or(true)
        {
            return;
        }

        apply_revm_storage_diff(ctx, ecx, call.target_address);
    }

    fn revive_record_create_address(
        &self,
        state: &mut foundry_cheatcodes::Cheatcodes,
        outcome: &CreateOutcome,
    ) {
        let ctx = get_context_ref_mut(state.strategy.context.as_mut());

        if ctx.record_next_create_address {
            ctx.record_next_create_address = false;
            if let Some(address) = outcome.address {
                ctx.skip_pvm_addresses.insert(address);
                tracing::info!(
                    "recorded address {:?} for skip execution in the pallet-revive",
                    address
                );
            }
        }
    }
}

fn post_exec(
    state: &mut foundry_cheatcodes::Cheatcodes,
    ecx: Ecx<'_, '_, '_>,
    executor: &mut dyn foundry_cheatcodes::CheatcodesExecutor,
    tracer: &mut Tracer,
    is_static_call: bool,
) {
    let ctx = &mut get_context_ref_mut(state.strategy.context.as_mut());

    let externalities = &mut ctx.externalities;
    let dual_compiled_contracts = &ctx.dual_compiled_contracts;

    let call_traces = externalities.execute_with(|| {
        tracer.apply_prestate_trace(ecx, dual_compiled_contracts);
        tracer.collect_call_traces()
    });
    if let Some(traces) = call_traces
        && !is_static_call
    {
        let mut logs: Vec<(u32, Log)> = vec![];
        logs.sort_by(|a, b| a.0.cmp(&b.0));
        if !state.expected_emits.is_empty() || state.recorded_logs.is_some() {
            logs = collect_logs(&traces);
        }
        if !state.expected_emits.is_empty() {
            logs.clone().into_iter().for_each(|(_, log)| {
                foundry_cheatcodes::handle_expect_emit(state, &log, &mut Default::default());
            })
        }
        if let Some(records) = &mut state.recorded_logs {
            records.extend(logs.iter().map(|(_, log)| foundry_cheatcodes::Vm::Log {
                data: log.data.data.clone(),
                emitter: log.address,
                topics: log.topics().to_owned(),
            }));
        };
        executor.trace_revive(state, ecx, Box::new(traces));
    }

    if let Some(expected_revert) = &mut state.expected_revert {
        expected_revert.max_depth =
            std::cmp::max(ecx.journaled_state.depth() + 1, expected_revert.max_depth);
    }
}

struct LogWithIndex {
    log: CallTrace,
    index: Vec<(Log, u32)>,
}

impl From<CallTrace> for LogWithIndex {
    fn from(value: CallTrace) -> Self {
        Self { log: value, index: vec![] }
    }
}

fn assign_indexes(trace: &mut LogWithIndex, mut index: u32) -> (u32, Vec<(Log, u32)>) {
    let mut sub_call_index = 0;
    for (i, _) in trace.log.logs.clone().iter().enumerate() {
        while sub_call_index < trace.log.logs[i].position {
            let (new_index, logs) =
                assign_indexes(&mut trace.log.calls[sub_call_index as usize].clone().into(), index);
            index = new_index;
            trace.index.extend(logs.into_iter());
            sub_call_index += 1;
        }
        let log = trace.log.logs[i].clone();
        trace.index.push((
            Log::new_unchecked(
                Address::from(log.address.0),
                log.topics.iter().map(|x| U256::from_be_slice(x.as_bytes()).into()).collect(),
                Bytes::from(log.data.0),
            ),
            index,
        ));
        index += 1;
    }
    while (sub_call_index as usize) < trace.log.calls.len() {
        let (new_index, logs) =
            assign_indexes(&mut trace.log.calls[sub_call_index as usize].clone().into(), index);
        index = new_index;
        trace.index.extend(logs.into_iter());
        sub_call_index += 1;
    }
    (index, trace.index.clone())
}

fn collect_logs(trace: &CallTrace) -> Vec<(u32, Log)> {
    let (_, mut l) = assign_indexes(&mut trace.clone().into(), 0);
    l.sort_by(|a, b| a.1.cmp(&b.1));
    l.into_iter().map(|x| (x.1, x.0)).collect()
}

fn get_context_ref_mut(
    ctx: &mut dyn CheatcodeInspectorStrategyContext,
) -> &mut PvmCheatcodeInspectorStrategyContext {
    ctx.as_any_mut().downcast_mut().expect("expected PvmCheatcodeInspectorStrategyContext")
}

fn apply_revm_storage_diff(
    ctx: &mut PvmCheatcodeInspectorStrategyContext,
    ecx: Ecx<'_, '_, '_>,
    address: Address,
) {
    let Some(account_state) = ecx.journaled_state.state.get(&address) else {
        return;
    };

    let h160_address = H160::from_slice(address.as_slice());

    // Check if contract exists in pallet-revive before applying storage diffs
    let contract_exists = ctx
        .externalities
        .execute_with(|| AccountInfo::<Runtime>::load_contract(&h160_address).is_some());

    if !contract_exists {
        return;
    }

    ctx.externalities.execute_with(|| {
        for (slot, storage_slot) in &account_state.storage {
            if storage_slot.is_changed() {
                let slot_bytes = slot.to_be_bytes::<32>();
                let new_value = storage_slot.present_value;

                if !new_value.is_zero() {
                    let _ = Pallet::<Runtime>::set_storage(
                        h160_address,
                        slot_bytes,
                        Some(new_value.to_be_bytes::<32>().to_vec()),
                    );
                } else {
                    let _ = Pallet::<Runtime>::set_storage(h160_address, slot_bytes, None);
                }
            }
        }
    });
}
