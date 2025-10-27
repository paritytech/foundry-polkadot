use alloy_primitives::{Address, B256, Bytes, Log, hex, ruint::aliases::U256};
use alloy_rpc_types::BlobTransactionSidecar;
use alloy_sol_types::SolValue;
use foundry_cheatcodes::{
    Broadcast, BroadcastableTransactions, CheatcodeInspectorStrategy,
    CheatcodeInspectorStrategyContext, CheatcodeInspectorStrategyRunner, CheatsConfig, CheatsCtxt,
    CommonCreateInput, DealRecord, Ecx, Error, EvmCheatcodeInspectorStrategyRunner, Result,
    Vm::{
        dealCall, getNonce_0Call, loadCall, pvmCall, rollCall, setNonceCall, setNonceUnsafeCall,
        storeCall, warpCall,
    },
    journaled_account, precompile_error,
};
use foundry_common::sh_err;
use foundry_compilers::resolc::dual_compiled_contracts::DualCompiledContracts;
use revive_env::{AccountId, Runtime, System, Timestamp};
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    sync::Arc,
};
use tracing::warn;

use polkadot_sdk::{
    frame_support::traits::{Currency, fungible::Mutate},
    pallet_balances,
    pallet_revive::{
        self, AccountInfo, AddressMapper, BalanceOf, BalanceWithDust, Code, Config, ContractInfo,
        ExecConfig, Pallet, evm::CallTrace,
    },
    polkadot_sdk_frame::prelude::OriginFor,
    sp_core::{self, H160},
    sp_weights::Weight,
};

use crate::{
    execute_with_externalities,
    tracing::{Tracer, storage_tracer::AccountAccess},
};
use foundry_cheatcodes::Vm::{AccountAccess as FAccountAccess, ChainInfo};

use alloy_eips::eip7702::SignedAuthorization;
use revm::{
    bytecode::opcode as op,
    context::{CreateScheme, JournalTr},
    interpreter::{
        CallInputs, CallOutcome, CreateOutcome, Gas, InstructionResult, Interpreter,
        InterpreterResult, interpreter_types::Jumps,
    },
    state::Bytecode,
};
pub trait PvmCheatcodeInspectorStrategyBuilder {
    fn new_pvm(dual_compiled_contracts: DualCompiledContracts, resolc_startup: bool) -> Self;
}
impl PvmCheatcodeInspectorStrategyBuilder for CheatcodeInspectorStrategy {
    // Creates a new PVM strategy
    fn new_pvm(dual_compiled_contracts: DualCompiledContracts, resolc_startup: bool) -> Self {
        Self {
            runner: &PvmCheatcodeInspectorStrategyRunner,
            context: Box::new(PvmCheatcodeInspectorStrategyContext::new(
                dual_compiled_contracts,
                resolc_startup,
            )),
        }
    }
}

/// Controls the automatic migration to PVM mode during test execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PvmStartupMigration {
    /// Defer database migration to a later execution point.
    /// This is the initial state - waiting for the test contract to be deployed.
    Defer,
    /// Allow database migration to PVM.
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
    /// Whether we're using PVM mode
    pub using_pvm: bool,
    /// Controls automatic migration to PVM mode
    pub pvm_startup_migration: PvmStartupMigration,
    pub dual_compiled_contracts: DualCompiledContracts,
    pub remove_recorded_access_at: Option<usize>,
}

impl PvmCheatcodeInspectorStrategyContext {
    pub fn new(dual_compiled_contracts: DualCompiledContracts, resolc_startup: bool) -> Self {
        Self {
            using_pvm: false, // Start in EVM mode by default
            pvm_startup_migration: if resolc_startup {
                PvmStartupMigration::Defer // Will be set to Allow when test contract deploys
            } else {
                PvmStartupMigration::Done // Disabled - never migrate
            },
            dual_compiled_contracts,
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

fn set_nonce(address: Address, nonce: u64, ecx: Ecx<'_, '_, '_>) {
    execute_with_externalities(|externalities| {
        externalities.execute_with(|| {
            let account_id =
                AccountId::to_fallback_account_id(&H160::from_slice(address.as_slice()));
            let current_nonce = System::account_nonce(&account_id);

            assert!(
                current_nonce as u64 <= nonce,
                "Cannot set nonce lower than current nonce: {current_nonce} > {nonce}"
            );

            while (System::account_nonce(&account_id) as u64) < nonce {
                System::inc_account_nonce(&account_id);
            }
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
    let balance_native =
        BalanceWithDust::<BalanceOf<Runtime>>::from_value::<Runtime>(amount_pvm).unwrap();

    let min_balance = pallet_balances::Pallet::<Runtime>::minimum_balance();

    let old_balance = execute_with_externalities(|externalities| {
        externalities.execute_with(|| {
            let addr = &AccountId::to_fallback_account_id(&H160::from_slice(address.as_slice()));
            let old_balance = pallet_revive::Pallet::<Runtime>::evm_balance(&H160::from_slice(
                address.as_slice(),
            ));
            pallet_balances::Pallet::<Runtime>::set_balance(
                addr,
                balance_native.into_rounded_balance().saturating_add(min_balance),
            );
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

fn set_timestamp(new_timestamp: U256, ecx: Ecx<'_, '_, '_>) {
    // Set timestamp in EVM context.
    ecx.block.timestamp = new_timestamp;

    // Set timestamp in pallet-revive runtime.
    execute_with_externalities(|externalities| {
        externalities.execute_with(|| {
            Timestamp::set_timestamp(new_timestamp.try_into().expect("Timestamp exceeds u64"));
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
                    select_pvm(ctx, ccx.ecx);
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
                set_nonce(account, newNonce, ccx.ecx);

                Ok(Default::default())
            }
            t if using_pvm && is::<setNonceUnsafeCall>(t) => {
                tracing::info!(cheatcode = ?cheatcode.as_debug() , using_pvm = ?using_pvm);
                // TODO implement unsafe_set_nonce on polkadot-sdk
                let &setNonceUnsafeCall { account, newNonce } =
                    cheatcode.as_any().downcast_ref().unwrap();
                set_nonce(account, newNonce, ccx.ecx);
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
            tracing::info!("startup PVM migration initiated");
            select_pvm(ctx, ecx);
            ctx.pvm_startup_migration.done();
            tracing::info!("startup PVM migration completed");
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

fn select_pvm(ctx: &mut PvmCheatcodeInspectorStrategyContext, data: Ecx<'_, '_, '_>) {
    if ctx.using_pvm {
        tracing::info!("already in PVM");
        return;
    }

    tracing::info!("switching to PVM");
    ctx.using_pvm = true;

    let block_number = data.block.number;
    let timestamp = data.block.timestamp;

    execute_with_externalities(|externalities| {
        externalities.execute_with(|| {
            System::set_block_number(block_number.saturating_to());
            Timestamp::set_timestamp(timestamp.saturating_to::<u64>() * 1000);

            let test_contract = data.journaled_state.database.get_test_contract_address();
            let persistent_accounts = data.journaled_state.database.persistent_accounts().clone();

            for address in persistent_accounts.into_iter().chain([data.tx.caller]) {
                let acc = data.journaled_state.load_account(address).expect("failed to load account");
                let amount = acc.data.info.balance;
                let nonce = acc.data.info.nonce;
                let account_id =
                    AccountId::to_fallback_account_id(&H160::from_slice(address.as_slice()));

                // Convert EVM balance to PVM balance with precision handling
                // TODO: needs to be replaced with `set_evm_balance`` once new pallet-revive is used
                let amount_pvm =
                    sp_core::U256::from_little_endian(&amount.as_le_bytes()).min(u128::MAX.into());
                let balance_native =
                    BalanceWithDust::<BalanceOf<Runtime>>::from_value::<Runtime>(amount_pvm).unwrap();
                let balance = Pallet::<Runtime>::convert_native_to_evm(balance_native);
                let amount_evm = U256::from_limbs(balance.0);

                if amount != amount_evm {
                    let _ = sh_err!(
                        "Amount mismatch {amount} != {amount_evm}, Polkadot balances are u128. Test results may be incorrect."
                    );
                }

                let min_balance = pallet_balances::Pallet::<Runtime>::minimum_balance();
                <Runtime as Config>::Currency::set_balance(
                    &account_id,
                    balance_native.into_rounded_balance().saturating_add(min_balance),
                );
                // END OF THE BLOCK TO BE REMOVED

                let current_nonce = System::account_nonce(&account_id);
                assert!(
                    current_nonce as u64 <= nonce,
                    "Cannot set nonce lower than current nonce: {current_nonce} > {nonce}"
                );

                while (System::account_nonce(&account_id) as u64) < nonce {
                    System::inc_account_nonce(&account_id);
                }

                // TODO handle immutables
                // Migrate bytecode for deployed contracts (skip test contract)
                if test_contract != Some(address)
                    && let Some(bytecode) = acc.data.info.code.as_ref() {

                    let account_h160 = H160::from_slice(address.as_slice());

                    // Skip if contract already exists in PVM
                    if AccountInfo::<Runtime>::load_contract(&account_h160).is_none() {
                        if let Some(pvm_bytecode) = ctx.dual_compiled_contracts
                            .find_by_evm_deployed_bytecode_with_immutables(bytecode.original_byte_slice())
                            .and_then(|(_, contract)| {
                                contract.resolc_bytecode.as_bytes()
                            })
                        {
                            let origin = OriginFor::<Runtime>::signed(Pallet::<Runtime>::account_id());
                            let code_hash = Pallet::<Runtime>::bare_upload_code(
                                origin,
                                pvm_bytecode.to_vec(),
                                BalanceOf::<Runtime>::MAX,
                            )
                            .ok()
                            .map(|upload_result| upload_result.code_hash)
                            .expect("Failed to upload PVM bytecode");

                            let contract_info = ContractInfo::<Runtime>::new(&account_h160, nonce as u32, code_hash)
                                .expect("Failed to create contract info");

                            AccountInfo::<Runtime>::insert_contract(&account_h160, contract_info);

                        } else {
                            tracing::info!(
                                address = ?address,
                                "no PVM equivalent found for EVM bytecode, skipping migration"
                            );
                        }
                    }
                }
            }
        })
    });
}

fn select_evm(ctx: &mut PvmCheatcodeInspectorStrategyContext, data: Ecx<'_, '_, '_>) {
    if !ctx.using_pvm {
        tracing::info!("already in EVM");
        return;
    }

    tracing::info!("switching to EVM");
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
                    if let Some((code_hash, bytecode)) = ctx
                        .dual_compiled_contracts
                        .find_by_resolc_bytecode_hash(hash)
                        .and_then(|(_, contract)| {
                            contract.evm_deployed_bytecode.as_bytes().map(|evm_bytecode| {
                                (
                                    contract.evm_bytecode_hash,
                                    Bytecode::new_raw(evm_bytecode.clone()),
                                )
                            })
                        })
                    {
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
        let ctx = get_context_ref_mut(state.strategy.context.as_mut());

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
        tracing::info!("running create in PVM");

        let find_contract = ctx
            .dual_compiled_contracts
            .find_bytecode(&init_code.0)
            .unwrap_or_else(|| panic!("failed finding contract for {init_code:?}"));

        let constructor_args = find_contract.constructor_args();
        let contract = find_contract.contract().clone();
        let mut tracer = Tracer::new(true);
        let res = execute_with_externalities(|externalities| {
            externalities.execute_with(|| {
                tracer.trace(|| {
                    let origin = OriginFor::<Runtime>::signed(AccountId::to_fallback_account_id(
                        &H160::from_slice(input.caller().as_slice()),
                    ));
                    let evm_value = sp_core::U256::from_little_endian(&input.value().as_le_bytes());

                    let code = Code::Upload(contract.resolc_bytecode.as_bytes().unwrap().to_vec());
                    let data = constructor_args.to_vec();
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
                        ExecConfig::new_substrate_tx(),
                    )
                })
            })
        });

        let mut gas = Gas::new(input.gas_limit());
        if res.result.as_ref().is_ok_and(|r| !r.result.did_revert()) {
            self.append_recorded_accesses(state, ecx, tracer.get_recorded_accesses());
        }
        post_exec(state, ecx, executor, &mut tracer, false);
        match &res.result {
            Ok(result) => {
                let _ = gas.record_cost(res.gas_required.ref_time());

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
                            output: contract.resolc_bytecode.as_bytes().unwrap().to_owned(),
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

        if !ctx.using_pvm {
            return None;
        }

        if ecx
            .journaled_state
            .database
            .get_test_contract_address()
            .map(|addr| call.bytecode_address == addr)
            .unwrap_or_default()
        {
            tracing::info!(
                "running call in EVM, instead of PVM (Test Contract) {:#?}",
                call.bytecode_address
            );
            return None;
        }

        tracing::info!("running call in PVM {:#?}", call);
        let mut tracer = Tracer::new(true);
        let res = execute_with_externalities(|externalities| {
            externalities.execute_with(|| {
                tracer.trace(|| {
                    let origin = OriginFor::<Runtime>::signed(AccountId::to_fallback_account_id(
                        &H160::from_slice(call.caller.as_slice()),
                    ));

                    let evm_value =
                        sp_core::U256::from_little_endian(&call.call_value().as_le_bytes());

                    let target = H160::from_slice(call.target_address.as_slice());

                    Pallet::<Runtime>::bare_call(
                        origin,
                        target,
                        evm_value,
                        Weight::MAX,
                        // TODO: fixing.
                        BalanceOf::<Runtime>::MAX,
                        call.input.bytes(ecx).to_vec(),
                        ExecConfig::new_substrate_tx(),
                    )
                })
            })
        });

        let mut gas = Gas::new(call.gas_limit);
        if res.result.as_ref().is_ok_and(|r| !r.did_revert()) {
            self.append_recorded_accesses(state, ecx, tracer.get_recorded_accesses());
        }
        post_exec(state, ecx, executor, &mut tracer, call.is_static);
        match res.result {
            Ok(result) => {
                let _ = gas.record_cost(res.gas_required.ref_time());

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
