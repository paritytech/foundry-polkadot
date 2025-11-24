mod mock_handler;

use alloy_primitives::{Address, B256, Bytes, Log, hex, ruint::aliases::U256};
use alloy_rpc_types::BlobTransactionSidecar;
use alloy_sol_types::SolValue;
use foundry_cheatcodes::{
    Broadcast, BroadcastableTransactions, CheatcodeInspectorStrategy,
    CheatcodeInspectorStrategyContext, CheatcodeInspectorStrategyRunner, CheatsConfig, CheatsCtxt,
    CommonCreateInput, DealRecord, Ecx, Error, EvmCheatcodeInspectorStrategyRunner, Result,
    Vm::{
        chainIdCall, dealCall, etchCall, getNonce_0Call, loadCall, pvmCall, resetNonceCall,
        rollCall, setNonceCall, setNonceUnsafeCall, storeCall, warpCall,
    },
    journaled_account, precompile_error,
};

use foundry_compilers::resolc::dual_compiled_contracts::DualCompiledContracts;
use foundry_evm::constants::CHEATCODE_ADDRESS;
use revive_env::{AccountId, Runtime, System, Timestamp};
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    sync::Arc,
};
use tracing::warn;

use alloy_eips::eip7702::SignedAuthorization;
use polkadot_sdk::{
    pallet_revive::{
        self, AccountInfo, AddressMapper, BalanceOf, BytecodeType, Code, ContractInfo,
        DebugSettings, ExecConfig, Executable, Pallet, evm::CallTrace,
    },
    polkadot_sdk_frame::prelude::OriginFor,
    sp_core::{self, H160, H256},
    sp_io,
    sp_weights::Weight,
};

use crate::{
    cheatcodes::mock_handler::MockHandlerImpl,
    execute_with_externalities,
    tracing::{Tracer, storage_tracer::AccountAccess},
};
use foundry_cheatcodes::Vm::{AccountAccess as FAccountAccess, ChainInfo};

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
    ) -> Self;
}
impl PvmCheatcodeInspectorStrategyBuilder for CheatcodeInspectorStrategy {
    // Creates a new PVM strategy
    fn new_pvm(
        dual_compiled_contracts: DualCompiledContracts,
        runtime_mode: crate::ReviveRuntimeMode,
    ) -> Self {
        Self {
            runner: &PvmCheatcodeInspectorStrategyRunner,
            context: Box::new(PvmCheatcodeInspectorStrategyContext::new(
                dual_compiled_contracts,
                runtime_mode,
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
    /// Controls automatic migration to pallet-revive
    pub pvm_startup_migration: PvmStartupMigration,
    pub dual_compiled_contracts: DualCompiledContracts,
    /// Runtime backend mode when using pallet-revive (PVM or EVM)
    pub runtime_mode: crate::ReviveRuntimeMode,
    pub remove_recorded_access_at: Option<usize>,
}

impl PvmCheatcodeInspectorStrategyContext {
    pub fn new(
        dual_compiled_contracts: DualCompiledContracts,
        runtime_mode: crate::ReviveRuntimeMode,
    ) -> Self {
        Self {
            // Start in REVM mode by default
            using_pvm: false,
            // Will be set to Allow when test contract deploys
            pvm_startup_migration: PvmStartupMigration::Defer,
            dual_compiled_contracts,
            runtime_mode,
            remove_recorded_access_at: None,
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

fn set_nonce(address: Address, nonce: u64, ecx: Ecx<'_, '_, '_>, check_nonce: bool) {
    execute_with_externalities(|externalities| {
        externalities.execute_with(|| {
            let account_id =
                AccountId::to_fallback_account_id(&H160::from_slice(address.as_slice()));
            let current_nonce = System::account_nonce(&account_id);
            if check_nonce {
                assert!(
                    current_nonce as u64 <= nonce,
                    "Cannot set nonce lower than current nonce: {current_nonce} > {nonce}"
                );
            }

            polkadot_sdk::frame_system::Account::<Runtime>::mutate(&account_id, |a| {
                a.nonce = nonce.min(u32::MAX.into()).try_into().expect("shouldn't happen");
            });
        })
    });
    let account = ecx.journaled_state.load_account(address).expect("account loaded").data;
    account.mark_touch();
    account.info.nonce = nonce;
}

fn set_balance(address: Address, amount: U256, ecx: Ecx<'_, '_, '_>) -> U256 {
    let account = ecx.journaled_state.load_account(address).expect("account loaded").data;
    account.mark_touch();
    account.info.balance = amount;
    let amount_pvm = sp_core::U256::from_little_endian(&amount.as_le_bytes()).min(u128::MAX.into());

    let old_balance = execute_with_externalities(|externalities| {
        externalities.execute_with(|| {
            let h160_addr = H160::from_slice(address.as_slice());
            let old_balance = pallet_revive::Pallet::<Runtime>::evm_balance(&h160_addr);
            pallet_revive::Pallet::<Runtime>::set_evm_balance(&h160_addr, amount_pvm)
                .expect("failed to set evm balance");
            old_balance
        })
    });
    U256::from_limbs(old_balance.0)
}

fn set_block_number(new_height: U256, ecx: Ecx<'_, '_, '_>) {
    // Set block number in EVM context.
    ecx.block.number = new_height;

    // Set block number in pallet-revive runtime.
    execute_with_externalities(|externalities| {
        externalities.execute_with(|| {
            System::set_block_number(new_height.try_into().expect("Block number exceeds u64"));
        })
    });
}

// Implements the `etch` cheatcode for PVM.
fn etch_call(target: &Address, new_runtime_code: &Bytes, ecx: Ecx<'_, '_, '_>) -> Result {
    let origin_address = H160::from_slice(ecx.tx.caller.as_slice());
    let origin_account = AccountId::to_fallback_account_id(&origin_address);

    let target_address = H160::from_slice(target.as_slice());
    let target_account = AccountId::to_fallback_account_id(&target_address);

    execute_with_externalities(|externalities| {
        externalities.execute_with(|| {
            let code = new_runtime_code.to_vec();
            let code_type =
                if code.starts_with(b"PVM\0") { BytecodeType::Pvm } else { BytecodeType::Evm };
            let contract_blob = Pallet::<Runtime>::try_upload_code(
                origin_account.clone(),
                code,
                code_type,
                BalanceOf::<Runtime>::MAX,
                &ExecConfig::new_substrate_tx(),
            )
            .map_err(|_| <&str as Into<Error>>::into("Could not upload PVM code"))?
            .0;

            let mut contract_info = if let Some(contract_info) =
                AccountInfo::<Runtime>::load_contract(&target_address)
            {
                contract_info
            } else {
                let contract_info = ContractInfo::<Runtime>::new(
                    &target_address,
                    System::account_nonce(target_account),
                    *contract_blob.code_hash(),
                )
                .map_err(|err| {
                    tracing::error!("Could not create contract info: {:?}", err);
                    <&str as Into<Error>>::into("Could not create contract info")
                })?;
                System::inc_account_nonce(AccountId::to_fallback_account_id(&target_address));
                contract_info
            };
            contract_info.code_hash = *contract_blob.code_hash();
            AccountInfo::<Runtime>::insert_contract(
                &H160::from_slice(target.as_slice()),
                contract_info,
            );
            Ok::<(), Error>(())
        })
    })?;
    Ok(Default::default())
}

fn set_timestamp(new_timestamp: U256, ecx: Ecx<'_, '_, '_>) {
    // Set timestamp in EVM context (seconds).
    ecx.block.timestamp = new_timestamp;

    // Set timestamp in pallet-revive runtime (milliseconds).
    execute_with_externalities(|externalities| {
        externalities.execute_with(|| {
            let timestamp_ms = new_timestamp.saturating_to::<u64>().saturating_mul(1000);
            Timestamp::set_timestamp(timestamp_ms);
        })
    });
}

fn set_chain_id(new_chain_id: u64, ecx: Ecx<'_, '_, '_>) {
    // Set new chain id.
    ecx.cfg.chain_id = new_chain_id;

    // Set chain id in pallet-revive runtime.
    execute_with_externalities(|externalities| {
        externalities.execute_with(|| {
            <revive_env::Runtime as polkadot_sdk::pallet_revive::Config>::ChainId::set(
                &ecx.cfg.chain_id,
            );
        })
    });
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
        let using_pvm = get_context_ref_mut(ccx.state.strategy.context.as_mut()).using_pvm;

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
            t if using_pvm && is::<dealCall>(t) => {
                tracing::info!(cheatcode = ?cheatcode.as_debug() , using_pvm = ?using_pvm);

                let &dealCall { account, newBalance } = cheatcode.as_any().downcast_ref().unwrap();

                let old_balance = set_balance(account, newBalance, ccx.ecx);
                let record = DealRecord { address: account, old_balance, new_balance: newBalance };
                ccx.state.eth_deals.push(record);
                Ok(Default::default())
            }
            t if using_pvm && is::<setNonceCall>(t) => {
                tracing::info!(cheatcode = ?cheatcode.as_debug() , using_pvm = ?using_pvm);

                let &setNonceCall { account, newNonce } =
                    cheatcode.as_any().downcast_ref().unwrap();
                set_nonce(account, newNonce, ccx.ecx, false);

                Ok(Default::default())
            }
            t if using_pvm && is::<setNonceUnsafeCall>(t) => {
                tracing::info!(cheatcode = ?cheatcode.as_debug() , using_pvm = ?using_pvm);

                let &setNonceUnsafeCall { account, newNonce } =
                    cheatcode.as_any().downcast_ref().unwrap();
                set_nonce(account, newNonce, ccx.ecx, false);

                Ok(Default::default())
            }
            t if using_pvm && is::<resetNonceCall>(t) => {
                tracing::info!(cheatcode = ?cheatcode.as_debug() , using_pvm = ?using_pvm);
                let &resetNonceCall { account } = cheatcode.as_any().downcast_ref().unwrap();
                set_nonce(account, 0, ccx.ecx, false);
                Ok(Default::default())
            }
            t if using_pvm && is::<getNonce_0Call>(t) => {
                tracing::info!(cheatcode = ?cheatcode.as_debug() , using_pvm = ?using_pvm);
                let &getNonce_0Call { account } = cheatcode.as_any().downcast_ref().unwrap();
                let nonce = execute_with_externalities(|externalities| {
                    externalities.execute_with(|| {
                        System::account_nonce(AccountId::to_fallback_account_id(&H160::from_slice(
                            account.as_slice(),
                        )))
                    })
                });
                Ok(u64::from(nonce).abi_encode())
            }
            t if using_pvm && is::<rollCall>(t) => {
                let &rollCall { newHeight } = cheatcode.as_any().downcast_ref().unwrap();

                set_block_number(newHeight, ccx.ecx);

                Ok(Default::default())
            }
            t if using_pvm && is::<warpCall>(t) => {
                let &warpCall { newTimestamp } = cheatcode.as_any().downcast_ref().unwrap();

                tracing::info!(cheatcode = ?cheatcode.as_debug() , using_pvm = ?using_pvm);
                set_timestamp(newTimestamp, ccx.ecx);

                Ok(Default::default())
            }

            t if using_pvm && is::<chainIdCall>(t) => {
                let &chainIdCall { newChainId } = cheatcode.as_any().downcast_ref().unwrap();

                tracing::info!(cheatcode = ?cheatcode.as_debug() , using_pvm = ?using_pvm);
                set_chain_id(newChainId.to(), ccx.ecx);

                Ok(Default::default())
            }
            t if using_pvm && is::<etchCall>(t) => {
                let etchCall { target, newRuntimeBytecode } =
                    cheatcode.as_any().downcast_ref().unwrap();
                etch_call(target, newRuntimeBytecode, ccx.ecx)?;
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
                let target_address_h160 = H160::from_slice(target.as_slice());
                let storage_value = execute_with_externalities(|externalities| {
                    externalities.execute_with(|| {
                        Pallet::<Runtime>::get_storage(target_address_h160, slot.into())
                    })
                });
                let result = storage_value
                    .ok()
                    .flatten()
                    .map(|b| B256::from_slice(&b))
                    .unwrap_or(B256::ZERO);
                Ok(result.abi_encode())
            }
            t if using_pvm && is::<storeCall>(t) => {
                tracing::info!(cheatcode = ?cheatcode.as_debug() , using_pvm = ?using_pvm);
                let &storeCall { target, slot, value } = cheatcode.as_any().downcast_ref().unwrap();
                if ccx.is_precompile(&target) {
                    return Err(precompile_error(&target));
                }
                if target == CHEATCODE_ADDRESS {
                    cheatcode.dyn_apply(ccx, executor)
                } else {
                    let target_address_h160 = H160::from_slice(target.as_slice());
                    let _ = execute_with_externalities(|externalities| {
                        externalities.execute_with(|| {
                            Pallet::<Runtime>::set_storage(
                                target_address_h160,
                                slot.into(),
                                Some(value.to_vec()),
                            )
                        })
                    })
                    .map_err(|_| <&str as Into<Error>>::into("Could not set storage"))?;
                    Ok(Default::default())
                }
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

        let balance = execute_with_externalities(|externalities| {
            externalities.execute_with(|| {
                Pallet::<Runtime>::evm_balance(&H160::from_slice(address.as_slice()))
            })
        });
        let balance = U256::from_limbs(balance.0);
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

    execute_with_externalities(|externalities| {
        externalities.execute_with(|| {
            // Enable debug mode to bypass EIP-170 size checks during testing
            if data.cfg.limit_contract_code_size == Some(usize::MAX) {
                let debug_settings = DebugSettings::new(true);
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
                                                .values().map(|offsets| offsets.first()).flatten()
                                                .flat_map(|offset| {
                                                    let start = offset.start as usize;
                                                    let end = start + offset.length as usize;
                                                    evm_bytecode.get(start..end).unwrap_or_else(|| panic!("Immutable offset out of bounds: address={:?}, offset={}..{}, bytecode_len={}",
                                                        address, start, end, evm_bytecode.len())).into_iter().rev()
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
                                &ExecConfig::new_substrate_tx(),
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
                        // Migrate complete account state (storage) for newly created contract
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
    });
}

fn select_evm(ctx: &mut PvmCheatcodeInspectorStrategyContext, data: Ecx<'_, '_, '_>) {
    if !ctx.using_pvm {
        tracing::info!("already using REVM");
        return;
    }

    tracing::info!("switching from pallet-revive back to REVM");
    ctx.using_pvm = false;

    execute_with_externalities(|externalities| {
        externalities.execute_with(|| {
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
        })
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
        let res = execute_with_externalities(|externalities| {
            externalities.execute_with(|| {
                tracer.trace(|| {
                    let origin = OriginFor::<Runtime>::signed(AccountId::to_fallback_account_id(
                        &H160::from_slice(input.caller().as_slice()),
                    ));
                    let evm_value = sp_core::U256::from_little_endian(&input.value().as_le_bytes());

                    mock_handler.fund_pranked_accounts(input.caller());

                    // Pre-Dispatch Increments the nonce of the origin, so let's make sure we do
                    // that here too to replicate the same address generation.
                    System::inc_account_nonce(AccountId::to_fallback_account_id(
                        &H160::from_slice(input.caller().as_slice()),
                    ));

                    let exec_config = ExecConfig {
                        bump_nonce: true,
                        collect_deposit_from_hold: None,
                        effective_gas_price: Some(gas_price_pvm),
                        mock_handler: Some(Box::new(mock_handler.clone())),
                        is_dry_run: None,
                    };
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

        let mut tracer = Tracer::new(true);
        let res = execute_with_externalities(|externalities| {
            externalities.execute_with(|| {
                tracer.trace(|| {
                    let origin = OriginFor::<Runtime>::signed(AccountId::to_fallback_account_id(
                        &H160::from_slice(call.caller.as_slice()),
                    ));

                    mock_handler.fund_pranked_accounts(call.caller);

                    let evm_value =
                        sp_core::U256::from_little_endian(&call.call_value().as_le_bytes());
                    let target = H160::from_slice(call.target_address.as_slice());
                    let exec_config = ExecConfig {
                        bump_nonce: true,
                        collect_deposit_from_hold: None,
                        effective_gas_price: Some(gas_price_pvm),
                        mock_handler: Some(Box::new(mock_handler.clone())),
                        is_dry_run: None,
                    };
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

        apply_revm_storage_diff(ecx, call.target_address);
    }
}

fn post_exec(
    state: &mut foundry_cheatcodes::Cheatcodes,
    ecx: Ecx<'_, '_, '_>,
    executor: &mut dyn foundry_cheatcodes::CheatcodesExecutor,
    tracer: &mut Tracer,
    is_static_call: bool,
) {
    tracer.apply_prestate_trace(ecx);
    if let Some(traces) = tracer.collect_call_traces()
        && !is_static_call
    {
        let mut logs = vec![];
        if !state.expected_emits.is_empty() || state.recorded_logs.is_some() {
            collect_logs(&mut logs, &traces);
        }
        if !state.expected_emits.is_empty() {
            logs.clone().into_iter().for_each(|log| {
                foundry_cheatcodes::handle_expect_emit(state, &log, &mut Default::default());
            })
        }
        if let Some(records) = &mut state.recorded_logs {
            records.extend(logs.iter().map(|log| foundry_cheatcodes::Vm::Log {
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

fn collect_logs(accumulator: &mut Vec<Log>, trace: &CallTrace) {
    accumulator.extend(trace.logs.iter().map(|log| {
        let log = log.clone();
        Log::new_unchecked(
            Address::from(log.address.0),
            log.topics.iter().map(|x| U256::from_be_slice(x.as_bytes()).into()).collect(),
            Bytes::from(log.data.0),
        )
    }));
    for call in &trace.calls {
        collect_logs(accumulator, call);
    }
}

fn get_context_ref_mut(
    ctx: &mut dyn CheatcodeInspectorStrategyContext,
) -> &mut PvmCheatcodeInspectorStrategyContext {
    ctx.as_any_mut().downcast_mut().expect("expected PvmCheatcodeInspectorStrategyContext")
}

/// Applies REVM storage diffs to pallet-revive (REVM → pallet-revive sync)
/// Note: Balance/nonce are NOT synced here as they're handled by migration in select_revive()
fn apply_revm_storage_diff(ecx: Ecx<'_, '_, '_>, address: Address) {
    let Some(account_state) = ecx.journaled_state.state.get(&address) else {
        return;
    };

    let h160_address = H160::from_slice(address.as_slice());

    // Check if contract exists in pallet-revive before applying storage diffs
    let contract_exists = execute_with_externalities(|externalities| {
        externalities
            .execute_with(|| AccountInfo::<Runtime>::load_contract(&h160_address).is_some())
    });

    if !contract_exists {
        return;
    }

    execute_with_externalities(|externalities| {
        externalities.execute_with(|| {
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
        })
    });
}
