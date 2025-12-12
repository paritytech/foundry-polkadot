use alloy_primitives::{Bytes, U256 as RU256};
use foundry_cheatcodes::Vm::{AccountAccessKind, StorageAccess};
use polkadot_sdk::{
    pallet_revive::{self, AccountInfo, Code, tracing::Tracing},
    sp_core::{H160, U256},
    sp_weights::Weight,
};
use revive_env::Runtime;

#[derive(Debug, Default)]
pub(crate) struct StorageTracer {
    /// Whether the current call is a contract creation.
    is_create: Option<Code>,

    records: Vec<AccountAccess>,
    pending: Vec<AccountAccess>,
    records_inner: Vec<AccountAccess>,
    index: usize,
    calls: Vec<H160>,
}

/// Represents the account access during vm execution.
#[derive(Debug, Clone)]
pub struct AccountAccess {
    /// Call depth.
    pub depth: u64,
    /// Call type.
    pub kind: AccountAccessKind,
    /// Account that was accessed.
    pub account: H160,
    /// Accessor account.
    pub accessor: H160,
    /// Call data.
    pub data: Bytes,
    /// Call value.
    pub value: U256,
    /// Previous balance of the accessed account.
    pub old_balance: U256,
    /// New balance of the accessed account.
    pub new_balance: U256,
    /// Storage slots that were accessed.
    pub storage_accesses: Vec<StorageAccess>,
    /// is reverted
    pub reverted: bool,
    pub index: usize,
    pub initialized: bool,
}

impl StorageTracer {
    pub fn get_records(&self) -> Vec<AccountAccess> {
        assert!(
            self.pending.is_empty(),
            "pending call stack is not empty; found calls without matching returns: {:?}",
            self.pending
        );
        assert!(
            self.records_inner.is_empty(),
            "inner stack is not empty; found calls without matching returns: {:?}",
            self.records_inner
        );
        let mut accounts = self.records.clone();
        accounts.sort_by_key(|f| f.index);
        accounts
    }

    fn current_addr(&self) -> H160 {
        self.calls.last().copied().unwrap_or_default()
    }
}

impl Tracing for StorageTracer {
    fn instantiate_code(&mut self, code: &Code, _salt: Option<&[u8; 32]>) {
        self.is_create = Some(code.clone());
    }

    fn enter_child_span(
        &mut self,
        from: H160,
        to: H160,
        is_delegate_call: bool,
        is_read_only: bool,
        value: U256,
        input: &[u8],
        _gas: Weight,
    ) {
        let code = self.is_create.take();

        if is_delegate_call {
            self.calls.push(self.current_addr());
        } else {
            self.calls.push(to);
        }

        let kind = if code.is_some() {
            AccountAccessKind::Create
        } else if is_read_only {
            AccountAccessKind::StaticCall
        } else if is_delegate_call {
            AccountAccessKind::DelegateCall
        } else {
            AccountAccessKind::Call
        };

        let last_depth = if !self.pending.is_empty() {
            self.pending.last().map(|record| record.depth).expect("must have at least one record")
        } else {
            self.records.last().map(|record| record.depth).unwrap_or_default()
        };
        let new_depth = last_depth.checked_add(1).expect("overflow in recording call depth");

        let mut record = AccountAccess {
            depth: new_depth,
            kind,
            account: to,
            accessor: from,
            data: Bytes::from(input.to_vec()),
            value,
            reverted: false,
            old_balance: pallet_revive::Pallet::<Runtime>::evm_balance(&to),
            new_balance: U256::zero(),
            storage_accesses: Default::default(),
            index: self.index,
            initialized: true,
        };
        if let Some(code) = code {
            match code {
                Code::Upload(items) => {
                    record.data = Bytes::from(items);
                }
                Code::Existing(_) => (),
            }
        }
        self.pending.push(record);
        self.index += 1;
    }

    fn terminate(
        &mut self,
        contract_address: H160,
        beneficiary_address: H160,
        _gas_left: Weight,
        value: U256,
    ) {
        let last_depth = if !self.pending.is_empty() {
            self.pending.last().map(|record| record.depth).expect("must have at least one record")
        } else {
            self.records.last().map(|record| record.depth).unwrap_or_default()
        };
        let new_depth = last_depth.checked_add(1).expect("overflow in recording call depth");
        let account = AccountInfo::<Runtime>::is_contract(&beneficiary_address);
        let record = AccountAccess {
            depth: new_depth,
            kind: AccountAccessKind::SelfDestruct,
            account: beneficiary_address,
            accessor: contract_address,
            data: Bytes::new(),
            value,
            reverted: false,
            old_balance: pallet_revive::Pallet::<Runtime>::evm_balance(&beneficiary_address),
            new_balance: U256::zero(),
            storage_accesses: Default::default(),
            index: self.index,
            initialized: account,
        };
        self.index += 1;

        self.records_inner.push(record);
    }

    fn exit_child_span_with_error(
        &mut self,
        _error: polkadot_sdk::sp_runtime::DispatchError,
        _gas_left: Weight,
    ) {
        self.calls.pop();

        let is_create = self.is_create.take();
        let mut record = self.pending.pop().expect("unexpected return while recording call");
        record.new_balance = pallet_revive::Pallet::<Runtime>::evm_balance(&self.current_addr());
        record.reverted = true;
        record.storage_accesses.iter_mut().for_each(|x| x.reverted = true);
        self.records_inner.iter_mut().for_each(|x| {
            if record.reverted {
                x.reverted = true;
                x.storage_accesses.iter_mut().for_each(|x| x.reverted = true);
            }
        });

        if let Some(code) = is_create {
            record.kind = AccountAccessKind::Create;
            match code {
                Code::Upload(items) => {
                    record.data = Bytes::from(items);
                }
                Code::Existing(_) => (),
            }
        }

        if self.pending.is_empty() {
            // no more pending records, append everything recorded so far.
            self.records.push(record);
            // append the inner records.
            if !self.records_inner.is_empty() {
                self.records.extend(std::mem::take(&mut self.records_inner));
            }
        } else {
            // we have pending records, so record to inner.
            self.records_inner.push(record);
        }
    }

    fn exit_child_span(
        &mut self,
        output: &polkadot_sdk::pallet_revive::ExecReturnValue,
        _gas_left: Weight,
    ) {
        self.calls.pop();

        let is_create = self.is_create.take();

        let mut record = self.pending.pop().expect("unexpected return while recording call");
        record.new_balance = pallet_revive::Pallet::<Runtime>::evm_balance(&self.current_addr());
        if output.did_revert() {
            record.reverted = true;
            record.storage_accesses.iter_mut().for_each(|x| x.reverted = true);
            self.records_inner.iter_mut().for_each(|x| {
                if record.reverted {
                    x.reverted = true;
                    x.storage_accesses.iter_mut().for_each(|x| x.reverted = true);
                }
            });
        }

        if let Some(code) = is_create {
            record.kind = AccountAccessKind::Create;
            match code {
                Code::Upload(items) => {
                    record.data = Bytes::from(items);
                }
                Code::Existing(_) => (),
            }
        }

        if self.pending.is_empty() {
            // no more pending records, append everything recorded so far.
            self.records.push(record);

            // append the inner records.
            if !self.records_inner.is_empty() {
                self.records.extend(std::mem::take(&mut self.records_inner));
            }
        } else {
            // we have pending records, so record to inner.
            self.records_inner.push(record);
        }
    }

    fn storage_read(&mut self, key: &polkadot_sdk::pallet_revive::Key, value: Option<&[u8]>) {
        let account = self.current_addr().0.into();
        let record = self.pending.last_mut().expect("expected at least one record");
        record.storage_accesses.push(StorageAccess {
            account,
            slot: RU256::from_be_slice(key.unhashed()).into(),
            isWrite: false,
            previousValue: RU256::from_be_slice(value.unwrap_or_default()).into(),
            newValue: RU256::from_be_slice(value.unwrap_or_default()).into(),
            reverted: false,
        });
    }
    fn storage_write(
        &mut self,
        key: &polkadot_sdk::pallet_revive::Key,
        old_value: Option<Vec<u8>>,
        new_value: Option<&[u8]>,
    ) {
        let account = self.current_addr().0.into();
        let record = self.pending.last_mut().expect("expected at least one record");
        record.storage_accesses.push(StorageAccess {
            account,
            slot: RU256::from_be_slice(key.unhashed()).into(),
            isWrite: true,
            previousValue: RU256::from_be_slice(old_value.unwrap_or_default().as_slice()).into(),
            newValue: RU256::from_be_slice(new_value.unwrap_or_default()).into(),
            reverted: false,
        });
    }
}
