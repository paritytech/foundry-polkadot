use alloy_primitives::{Address, Bytes, U256 as RU256};
use call_tracer::ExpectedCallTracer;
use foundry_cheatcodes::{Ecx, ExpectedCallTracker};
use polkadot_sdk::pallet_revive::{
    Pallet, U256, Weight,
    evm::{
        CallTrace, CallTracer, PrestateTrace, PrestateTraceInfo, PrestateTracer,
        PrestateTracerConfig, Tracer as ReviveTracer, TracerType,
    },
    tracing::{Tracing, trace as trace_revive},
};
use revert_tracer::RevertTracer;
use revive_env::Runtime;
use revm::{context::JournalTr, database::states::StorageSlot, state::Bytecode};
use storage_tracer::{AccountAccess, StorageTracer};
mod call_tracer;
mod revert_tracer;
pub mod storage_tracer;
pub struct Tracer {
    pub call_tracer: CallTracer<U256, fn(Weight) -> U256>,
    pub prestate_tracer: PrestateTracer<Runtime>,
    pub storage_accesses: StorageTracer,
    pub revert_tracer: RevertTracer,
    pub expect_call_tracer: ExpectedCallTracer,
}

impl Tracer {
    pub fn new(data: ExpectedCallTracker) -> Self {
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

        Self {
            call_tracer,
            prestate_tracer,
            storage_accesses: Default::default(),
            revert_tracer: RevertTracer::new(),
            expect_call_tracer: ExpectedCallTracer::new(data),
        }
    }

    pub fn trace<R, F: FnOnce() -> R>(&mut self, f: F) -> R {
        trace_revive(self, f)
    }

    /// Collects call traces
    pub fn collect_call_traces(&mut self) -> Option<CallTrace> {
        self.call_tracer.clone().collect_trace()
    }

    /// Collects prestate traces
    fn collect_prestate_traces(&mut self) -> PrestateTrace {
        self.prestate_tracer.clone().collect_trace()
    }

    /// Collects recorded accesses
    pub fn get_recorded_accesses(&mut self) -> Vec<AccountAccess> {
        self.storage_accesses.get_records()
    }

    /// Applies `PrestateTrace` diffs to the revm state
    pub fn apply_prestate_trace(&mut self, ecx: Ecx<'_, '_, '_>) {
        let prestate_trace = self.collect_prestate_traces();
        match prestate_trace {
            polkadot_sdk::pallet_revive::evm::PrestateTrace::DiffMode { pre: _digests, post } => {
                for (key, PrestateTraceInfo { balance, nonce, code, storage }) in post {
                    let address = Address::from_slice(key.as_bytes());

                    ecx.journaled_state.load_account(address).expect("account could not be loaded");

                    ecx.journaled_state.touch(address);
                    let account = ecx.journaled_state.state.get_mut(&address).unwrap();
                    if let Some(balance) = balance {
                        account.info.balance = RU256::from_limbs(balance.0);
                    };

                    if let Some(nonce) = nonce {
                        account.info.nonce = nonce.into();
                    };

                    if let Some(ref code) = code {
                        let code = code.clone();
                        let account =
                            ecx.journaled_state.state.get_mut(&address).expect("account is loaded");
                        let bytecode = Bytecode::new_raw(Bytes::from(code.0));
                        account.info.code_hash = bytecode.hash_slow();
                        account.info.code = Some(bytecode);
                    }
                    let storage = { storage };

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

impl Tracing for Tracer {
    fn watch_address(&mut self, addr: &polkadot_sdk::sp_core::H160) {
        self.prestate_tracer.watch_address(addr);
        self.call_tracer.watch_address(addr);
        self.storage_accesses.watch_address(addr);
        self.revert_tracer.watch_address(addr);
    }

    fn terminate(
        &mut self,
        contract_address: polkadot_sdk::sp_core::H160,
        beneficiary_address: polkadot_sdk::sp_core::H160,
        gas_left: Weight,
        value: U256,
    ) {
        self.prestate_tracer.terminate(contract_address, beneficiary_address, gas_left, value);
        self.call_tracer.terminate(contract_address, beneficiary_address, gas_left, value);
        self.storage_accesses.terminate(contract_address, beneficiary_address, gas_left, value);
        self.revert_tracer.terminate(contract_address, beneficiary_address, gas_left, value);
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
        self.storage_accesses.enter_child_span(
            from,
            to,
            is_delegate_call,
            is_read_only,
            value,
            input,
            gas,
        );
        self.revert_tracer.enter_child_span(
            from,
            to,
            is_delegate_call,
            is_read_only,
            value,
            input,
            gas,
        );
        self.expect_call_tracer.enter_child_span(
            from,
            to,
            is_delegate_call,
            is_read_only,
            value,
            input,
            gas,
        )
    }

    fn instantiate_code(
        &mut self,
        code: &polkadot_sdk::pallet_revive::Code,
        salt: Option<&[u8; 32]>,
    ) {
        self.prestate_tracer.instantiate_code(code, salt);
        self.call_tracer.instantiate_code(code, salt);
        self.storage_accesses.instantiate_code(code, salt);
        self.revert_tracer.instantiate_code(code, salt);
        self.expect_call_tracer.instantiate_code(code, salt);
    }

    fn balance_read(&mut self, addr: &polkadot_sdk::sp_core::H160, value: U256) {
        self.prestate_tracer.balance_read(addr, value);
        self.call_tracer.balance_read(addr, value);
        self.storage_accesses.balance_read(addr, value);
        self.revert_tracer.balance_read(addr, value);
    }

    fn storage_read(&mut self, key: &polkadot_sdk::pallet_revive::Key, value: Option<&[u8]>) {
        self.prestate_tracer.storage_read(key, value);
        self.call_tracer.storage_read(key, value);
        self.storage_accesses.storage_read(key, value);
        self.revert_tracer.storage_read(key, value);
    }

    fn storage_write(
        &mut self,
        key: &polkadot_sdk::pallet_revive::Key,
        old_value: Option<Vec<u8>>,
        new_value: Option<&[u8]>,
    ) {
        self.prestate_tracer.storage_write(key, old_value.clone(), new_value);
        self.call_tracer.storage_write(key, old_value.clone(), new_value);
        self.storage_accesses.storage_write(key, old_value.clone(), new_value);
        self.revert_tracer.storage_write(key, old_value, new_value);
    }

    fn log_event(
        &mut self,
        event: polkadot_sdk::sp_core::H160,
        topics: &[polkadot_sdk::sp_core::H256],
        data: &[u8],
    ) {
        self.prestate_tracer.log_event(event, topics, data);
        self.call_tracer.log_event(event, topics, data);
        self.storage_accesses.log_event(event, topics, data);
        self.revert_tracer.log_event(event, topics, data);
    }

    fn exit_child_span(
        &mut self,
        output: &polkadot_sdk::pallet_revive::ExecReturnValue,
        gas_left: Weight,
    ) {
        self.prestate_tracer.exit_child_span(output, gas_left);
        self.call_tracer.exit_child_span(output, gas_left);
        self.storage_accesses.exit_child_span(output, gas_left);
        self.revert_tracer.exit_child_span(output, gas_left);
        self.expect_call_tracer.exit_child_span(output, gas_left);
    }

    fn exit_child_span_with_error(
        &mut self,
        error: polkadot_sdk::sp_runtime::DispatchError,
        gas_left: Weight,
    ) {
        self.prestate_tracer.exit_child_span_with_error(error, gas_left);
        self.call_tracer.exit_child_span_with_error(error, gas_left);
        self.storage_accesses.exit_child_span_with_error(error, gas_left);
        self.revert_tracer.exit_child_span_with_error(error, gas_left);
        self.expect_call_tracer.exit_child_span_with_error(error, gas_left);
    }
}
