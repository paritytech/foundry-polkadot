use alloy_primitives::{Bytes, U256 as RU256};
use foundry_cheatcodes::Vm::{AccountAccessKind, StorageAccess};
use polkadot_sdk::{
    pallet_revive::{self, Code, tracing::Tracing},
    sp_core::{H160, H256, U256},
    sp_weights::Weight,
};
use revive_env::Runtime;

#[derive(Debug, Default)]
pub(crate) struct StorageTracer {
    /// The current address of the contract's which storage is being accessed.
    current_addr: H160,
    /// Whether the current call is a contract creation.
    is_create: Option<Code>,

    records: Vec<AccountAccess>,
    pending: Vec<AccountAccess>,
    records_inner: Vec<AccountAccess>,
    /// Track the calls that must be skipped.
    /// We track this on a different stack to easily skip the `call_end`
    /// instances, if they were marked to be skipped in the `call_start`.
    call_skip_tracker: Vec<bool>,
    /// Mark the next call at a given depth and having the given address accesses.
    /// This is useful, for example to skip nested constructor calls after CREATE,
    /// to allow us to omit/flatten them like in EVM.
    skip_next_call: Option<(u64, CallAddresses)>,
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
    /// Deployed bytecode hash if CREATE.
    pub deployed_bytecode_hash: Option<H256>,
    /// Call value.
    pub value: U256,
    /// Previous balance of the accessed account.
    pub old_balance: U256,
    /// New balance of the accessed account.
    pub new_balance: U256,
    /// Storage slots that were accessed.
    pub storage_accesses: Vec<StorageAccess>,
}

#[derive(Debug, Default, Clone)]
struct CallAddresses {
    pub to: H160,
    pub from: H160,
}

impl StorageTracer {
    pub fn get_records(&self) -> Vec<AccountAccess> {
        assert!(
            self.call_skip_tracker.is_empty(),
            "call skip tracker is not empty; found calls without matching returns: {:?}",
            self.call_skip_tracker
        );
        assert!(
            self.skip_next_call.is_none(),
            "skip next call is not empty: {:?}",
            self.skip_next_call
        );
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
        self.records.clone()
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
        use pallet_revive::{AccountId32Mapper, AddressMapper};
        let system_addr = AccountId32Mapper::<Runtime>::to_address(
            &pallet_revive::Pallet::<Runtime>::account_id(),
        );
        if system_addr == from || system_addr == to || is_read_only {
            self.call_skip_tracker.push(true);
            return;
        }
        let kind = if self.is_create.is_some() {
            AccountAccessKind::Create
        } else {
            AccountAccessKind::Call
        };

        let last_depth = if !self.pending.is_empty() {
            self.pending.last().map(|record| record.depth).expect("must have at least one record")
        } else {
            self.records.last().map(|record| record.depth).unwrap_or_default()
        };
        let new_depth = last_depth.checked_add(1).expect("overflow in recording call depth");

        // For create we expect another CALL if the constructor is invoked. We need to skip/flatten
        // this call so it is consistent with CREATE in the EVM.
        match kind {
            AccountAccessKind::Create => {
                // skip the next nested call to the created address from the caller.
                self.skip_next_call =
                    Some((new_depth.saturating_add(1), CallAddresses { to, from }));
            }
            AccountAccessKind::Call => {
                if let Some((depth, call_addr)) = self.skip_next_call.take()
                    && depth == new_depth
                    && call_addr.from == from
                    && call_addr.to == to
                {
                    self.call_skip_tracker.push(true);
                    return;
                }
            }
            _ => panic!("cant be matched"),
        }
        self.call_skip_tracker.push(false);
        self.pending.push(AccountAccess {
            depth: new_depth,
            kind,
            account: to,
            accessor: from,
            data: Bytes::from(input.to_vec()),
            deployed_bytecode_hash: None,
            value,
            old_balance: pallet_revive::Pallet::<Runtime>::evm_balance(&to),
            new_balance: U256::zero(),
            storage_accesses: Default::default(),
        });

        if !is_delegate_call {
            self.current_addr = to;
        }
    }

    fn exit_child_span_with_error(
        &mut self,
        _error: polkadot_sdk::sp_runtime::DispatchError,
        _gas_left: Weight,
    ) {
        self.is_create = None
    }

    fn exit_child_span(
        &mut self,
        _output: &polkadot_sdk::pallet_revive::ExecReturnValue,
        _gas_left: Weight,
    ) {
        let skip_call =
            self.call_skip_tracker.pop().expect("unexpected return while skipping call recording");
        if skip_call {
            return;
        }
        let mut record = self.pending.pop().expect("unexpected return while recording call");
        record.new_balance = pallet_revive::Pallet::<Runtime>::evm_balance(&self.current_addr);
        let is_create = self.is_create.take();
        if is_create.is_some() {
            match is_create {
                Some(Code::Existing(_)) => (),
                Some(Code::Upload(_)) => (),
                None => (),
            }
        }

        if let Some((depth, _)) = &self.skip_next_call
            && record.depth < *depth
        {
            // reset call skip if not encountered (depth has been crossed)
            self.skip_next_call = None;
        }

        if self.pending.is_empty() {
            // no more pending records, append everything recorded so far.
            self.records.push(record);

            // also append the inner records.
            if !self.records_inner.is_empty() {
                self.records.extend(std::mem::take(&mut self.records_inner));
            }
        } else {
            // we have pending records, so record to inner.
            self.records_inner.push(record);
        }
    }

    fn storage_read(&mut self, key: &polkadot_sdk::pallet_revive::Key, value: Option<&[u8]>) {
        let record = self.pending.last_mut().expect("expected at least one record");
        record.storage_accesses.push(StorageAccess {
            account: self.current_addr.0.into(),
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
        let record = self.pending.last_mut().expect("expected at least one record");
        record.storage_accesses.push(StorageAccess {
            account: self.current_addr.0.into(),
            slot: RU256::from_be_slice(key.unhashed()).into(),
            isWrite: true,
            previousValue: RU256::from_be_slice(old_value.unwrap_or_default().as_slice()).into(),
            newValue: RU256::from_be_slice(new_value.unwrap_or_default()).into(),
            reverted: false,
        });
    }
}
