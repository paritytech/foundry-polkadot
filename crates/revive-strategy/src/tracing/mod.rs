use alloy_primitives::{Address, Bytes, U256 as RU256};
use foundry_cheatcodes::Ecx;
use polkadot_sdk::{
    frame_support::traits::{Currency, fungible::Mutate},
    pallet_balances,
    pallet_revive::{
        Pallet, U256, Weight, AddressMapper,
        evm::{
            CallTrace, CallTracer, PrestateTrace, PrestateTraceInfo, PrestateTracer,
            PrestateTracerConfig, Tracer as ReviveTracer, TracerType,
        },
        tracing::{Tracing, trace as trace_revive},
        BalanceOf, BalanceWithDust,
    },
    sp_core::H160,
};
use revive_env::Runtime;
use revm::{context::JournalTr, database::states::StorageSlot, state::Bytecode};
use storage_tracer::{AccountAccess, StorageTracer};
pub mod storage_tracer;
use crate::execute_with_externalities;
use std::collections::HashMap;


pub struct Tracer {
    pub call_tracer: CallTracer<U256, fn(Weight) -> U256>,
    pub prestate_tracer: PrestateTracer<Runtime>,
    pub storage_accesses: Option<StorageTracer>,
}

impl Tracer {
    pub fn new(is_recording: bool) -> Self {
        let call_tracer =
            match Pallet::<revive_env::Runtime>::evm_tracer(TracerType::CallTracer(None)) {
                ReviveTracer::CallTracer(tracer) => tracer,
                _ => unreachable!("Expected CallTracer variant"),
            };

        let prestate_tracer: PrestateTracer<revive_env::Runtime> =
            PrestateTracer::new(PrestateTracerConfig {
                diff_mode: true,
                disable_storage: false,
                disable_code: false,
            });

        let storage_tracer = if is_recording { Some(Default::default()) } else { None };

        Self { call_tracer, prestate_tracer, storage_accesses: storage_tracer }
    }

    pub fn trace<R, F: FnOnce() -> R>(&mut self, f: F) -> R {
        trace_revive(self, f)
    }

    /// Collects call traces
    pub fn collect_call_traces(&mut self) -> Option<CallTrace> {
        execute_with_externalities(|externalities| {
            externalities.execute_with(|| self.call_tracer.clone().collect_trace())
        })
    }

    /// Collects prestate traces
    fn collect_prestate_traces(&mut self) -> PrestateTrace {
        execute_with_externalities(|externalities| {
            externalities.execute_with(|| self.prestate_tracer.clone().collect_trace())
        })
    }

    /// Collects recorded accesses
    pub fn get_recorded_accesses(&mut self) -> Vec<AccountAccess> {
        self.storage_accesses.take().unwrap_or_default().get_records()
    }

    /// Applies `PrestateTrace` diffs to the revm state
    pub fn apply_prestate_trace(&mut self, ecx: Ecx<'_, '_, '_>) {
        let prestate_trace = self.collect_prestate_traces();
        match prestate_trace {
            polkadot_sdk::pallet_revive::evm::PrestateTrace::DiffMode { pre: _, post } => {
                for (key, PrestateTraceInfo { balance, nonce, code, storage }) in post {
                    let address = Address::from_slice(key.as_bytes());

                    let account = ecx
                        .journaled_state
                        .load_account(address)
                        .expect("account could not be loaded")
                        .data;

                    account.mark_touch();

                    if let Some(balance) = balance {
                        account.info.balance = RU256::from_limbs(balance.0);
                    };

                    if let Some(nonce) = nonce {
                        account.info.nonce = nonce.into();
                    };

                    if let Some(code) = code {
                        let account =
                            ecx.journaled_state.state.get_mut(&address).expect("account is loaded");
                        let bytecode = Bytecode::new_raw(Bytes::from(code.0));
                        account.info.code_hash = bytecode.hash_slow();
                        account.info.code = Some(bytecode);
                    }
                    ecx.journaled_state.load_account(address).expect("account could not be loaded");

                    ecx.journaled_state.touch(address);
                    for (slot, entry) in storage {
                        let key = RU256::from_be_slice(&slot.0);
                        let previous = ecx.journaled_state.sload(address, key).expect("to load");

                        if let Some(e_entry) = entry {
                            let entry = RU256::from_be_slice(&e_entry.0);
                            let new_slot = StorageSlot::new_changed(previous.data, entry);
                            ecx.journaled_state
                                .sstore(address, key, new_slot.present_value)
                                .expect("to succeed");
                        }
                    }
                }
            }
            _ => panic!("Can't happen"),
        };
    }

}

/// Applies REVM storage diffs to pallet-revive (REVM → PVM sync)
/// This is a standalone function, similar to post_exec()
///
/// Uses REVM's built-in journaling as a "diff mode tracer":
/// - StorageSlot.is_changed() detects changes
/// - original_value and present_value provide the diff
///
/// Note: Balance/nonce are NOT synced here as they're handled by migration in select_revive()
pub fn apply_revm_storage_diff(ecx: Ecx<'_, '_, '_>, address: Address) {
    let mut storage_changes = HashMap::new();

    // Extract storage changes using REVM's built-in change tracking
    // This is equivalent to PrestateTracer diff mode for REVM
    if let Some(account_state) = ecx.journaled_state.state.get(&address) {
        for (slot, storage_slot) in &account_state.storage {
            // REVM's StorageSlot tracks original_value and present_value (diff mode!)
            if storage_slot.is_changed() {
                tracing::trace!(
                    "REVM storage diff: slot {:?} changed: {:?} -> {:?}",
                    slot,
                    storage_slot.original_value,
                    storage_slot.present_value
                );
                storage_changes.insert(
                    *slot,
                    (storage_slot.original_value, storage_slot.present_value),
                );
            }
        }
    }

    if storage_changes.is_empty() {
        return;
    }

    // Apply storage diffs to pallet-revive
    execute_with_externalities(|externalities| {
        externalities.execute_with(|| {
            let h160_address = H160::from_slice(address.as_slice());

            tracing::debug!(
                "Applying REVM storage diff to pallet-revive: {} changes for address {:?}",
                storage_changes.len(),
                address
            );

            // Sync storage changes only (balance/nonce handled by migration)
            for (slot, (old_value, new_value)) in storage_changes {
                let slot_bytes = slot.to_be_bytes::<32>();
                let new_value_bytes = new_value.to_be_bytes::<32>();

                if !new_value.is_zero() {
                    // Write new value
                    let _ = Pallet::<Runtime>::set_storage(
                        h160_address,
                        slot_bytes,
                        Some(new_value_bytes.to_vec()),
                    );

                    tracing::trace!(
                        "Storage slot {:?} updated: {:?} -> {:?}",
                        slot,
                        old_value,
                        new_value
                    );
                } else {
                    // Delete slot (value is now zero)
                    let _ = Pallet::<Runtime>::set_storage(
                        h160_address,
                        slot_bytes,
                        None,
                    );

                    tracing::trace!(
                        "Storage slot {:?} deleted (was {:?})",
                        slot,
                        old_value
                    );
                }
            }
        })
    });
}

impl Tracing for Tracer {
    fn watch_address(&mut self, addr: &polkadot_sdk::sp_core::H160) {
        self.prestate_tracer.watch_address(addr);
        self.call_tracer.watch_address(addr);
        if let Some(storage_tracer) = &mut self.storage_accesses {
            storage_tracer.watch_address(addr);
        }
    }

    fn enter_child_span(
        &mut self,
        from: polkadot_sdk::sp_core::H160,
        to: polkadot_sdk::sp_core::H160,
        is_delegate_call: bool,
        is_read_only: bool,
        value: U256,
        input: &[u8],
        gas: Weight,
    ) {
        self.prestate_tracer.enter_child_span(
            from,
            to,
            is_delegate_call,
            is_read_only,
            value,
            input,
            gas,
        );
        self.call_tracer.enter_child_span(
            from,
            to,
            is_delegate_call,
            is_read_only,
            value,
            input,
            gas,
        );
        if let Some(storage_tracer) = &mut self.storage_accesses {
            storage_tracer.enter_child_span(
                from,
                to,
                is_delegate_call,
                is_read_only,
                value,
                input,
                gas,
            )
        }
    }

    fn instantiate_code(
        &mut self,
        code: &polkadot_sdk::pallet_revive::Code,
        salt: Option<&[u8; 32]>,
    ) {
        self.prestate_tracer.instantiate_code(code, salt);
        self.call_tracer.instantiate_code(code, salt);
        if let Some(storage_tracer) = &mut self.storage_accesses {
            storage_tracer.instantiate_code(code, salt);
        }
    }

    fn balance_read(&mut self, addr: &polkadot_sdk::sp_core::H160, value: U256) {
        self.prestate_tracer.balance_read(addr, value);
        self.call_tracer.balance_read(addr, value);
        if let Some(storage_tracer) = &mut self.storage_accesses {
            storage_tracer.balance_read(addr, value);
        }
    }

    fn storage_read(&mut self, key: &polkadot_sdk::pallet_revive::Key, value: Option<&[u8]>) {
        self.prestate_tracer.storage_read(key, value);
        self.call_tracer.storage_read(key, value);
        if let Some(storage_tracer) = &mut self.storage_accesses {
            storage_tracer.storage_read(key, value);
        }
    }

    fn storage_write(
        &mut self,
        key: &polkadot_sdk::pallet_revive::Key,
        old_value: Option<Vec<u8>>,
        new_value: Option<&[u8]>,
    ) {
        self.prestate_tracer.storage_write(key, old_value.clone(), new_value);
        self.call_tracer.storage_write(key, old_value.clone(), new_value);
        if let Some(storage_tracer) = &mut self.storage_accesses {
            storage_tracer.storage_write(key, old_value, new_value);
        }
    }

    fn log_event(
        &mut self,
        event: polkadot_sdk::sp_core::H160,
        topics: &[polkadot_sdk::sp_core::H256],
        data: &[u8],
    ) {
        self.prestate_tracer.log_event(event, topics, data);
        self.call_tracer.log_event(event, topics, data);
        if let Some(storage_tracer) = &mut self.storage_accesses {
            storage_tracer.log_event(event, topics, data);
        }
    }

    fn exit_child_span(
        &mut self,
        output: &polkadot_sdk::pallet_revive::ExecReturnValue,
        gas_left: Weight,
    ) {
        self.prestate_tracer.exit_child_span(output, gas_left);
        self.call_tracer.exit_child_span(output, gas_left);
        if let Some(storage_tracer) = &mut self.storage_accesses {
            storage_tracer.exit_child_span(output, gas_left);
        }
    }

    fn exit_child_span_with_error(
        &mut self,
        error: polkadot_sdk::sp_runtime::DispatchError,
        gas_left: Weight,
    ) {
        self.prestate_tracer.exit_child_span_with_error(error, gas_left);
        self.call_tracer.exit_child_span_with_error(error, gas_left);
        if let Some(storage_tracer) = &mut self.storage_accesses {
            storage_tracer.exit_child_span_with_error(error, gas_left);
        }
    }
}
