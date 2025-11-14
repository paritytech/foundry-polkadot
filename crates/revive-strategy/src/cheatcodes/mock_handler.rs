use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    rc::Rc,
};

use alloy_primitives::{Address, Bytes, map::foldhash::HashMap, ruint::aliases::U256};
use foundry_cheatcodes::{Ecx, MockCallDataContext, MockCallReturnData};
use polkadot_sdk::{
    frame_system,
    pallet_revive::{
        self, AddressMapper, DelegateInfo, ExecOrigin, ExecReturnValue, Pallet, mock::MockHandler,
    },
    pallet_revive_uapi::ReturnFlags,
    polkadot_sdk_frame::prelude::OriginFor,
    sp_core::H160,
};
use revive_env::{AccountId, Runtime};

use revm::interpreter::InstructionResult;

// Implementation object that holds the mock state and implements the MockHandler trait for Revive.
// It is only purpose is to make transferring the mock state into the Revive EVM easier and then
// synchronize whatever mocks got consumed back into the Cheatcodes state after the call.
#[derive(Clone)]
pub(crate) struct MockHandlerImpl {
    inner: Rc<RefCell<MockHandlerInner<Runtime>>>,
    pub origin: ExecOrigin<Runtime>,
}

impl MockHandlerImpl {
    /// Creates a new MockHandlerImpl from the given Ecx and Cheatcodes state.
    pub(crate) fn new(
        ecx: &Ecx<'_, '_, '_>,
        caller: &Address,
        origin: &Address,
        target_address: Option<&Address>,
        callee: Option<&Address>,
        state: &mut foundry_cheatcodes::Cheatcodes,
    ) -> Self {
        let inject_env = MockHandlerInner::new(ecx, caller, target_address, callee, state);
        Self {
            inner: Rc::new(RefCell::new(inject_env)),
            origin: ExecOrigin::<Runtime>::from_runtime_origin(OriginFor::<Runtime>::signed(
                AccountId::to_fallback_account_id(&H160::from_slice(origin.as_slice())),
            ))
            .expect("Could not create tx origin"),
        }
    }

    /// Updates the given Cheatcodes state with the current mock state.
    /// This is used to synchronize the mock state after a call has been executed in Revive
    pub(crate) fn update_state_mocks(&self, state: &mut foundry_cheatcodes::Cheatcodes) {
        let mock_inner = self.inner.borrow();
        state.mocked_calls = mock_inner.mocked_calls.clone();
        state.mocked_functions = mock_inner.mocked_functions.clone();
    }

    pub(crate) fn fund_pranked_accounts(&self, account: Address) {
        // Fuzzed prank addresses have no balance, so they won't exist in revive, and
        // calls will fail, this is not a problem when running in REVM.
        // TODO: Figure it out why this is still needed.
        let balance = Pallet::<Runtime>::evm_balance(&H160::from_slice(account.as_slice()));
        if balance == 0.into() {
            Pallet::<Runtime>::set_evm_balance(
                &H160::from_slice(account.as_slice()),
                u128::MAX.into(),
            )
            .expect("Could not fund pranked account");
        }
    }
}

impl MockHandler<Runtime> for MockHandlerImpl {
    fn mock_call(
        &self,
        callee: H160,
        call_data: &[u8],
        value_transferred: polkadot_sdk::pallet_revive::U256,
    ) -> Option<pallet_revive::ExecReturnValue> {
        let mut mock_inner = self.inner.borrow_mut();
        let ctx = MockCallDataContext {
            calldata: call_data.to_vec().into(),
            value: Some(U256::from_limbs(value_transferred.0)),
        };

        // Use the same logic as in inspect.rs to find the correct mocked call and consume some of
        // them. https://github.com/paritytech/foundry-polkadot/blob/26eda0de53ac03f7ac9b6a6023d8243101cffaf1/crates/cheatcodes/src/inspector.rs#L1013
        if let Some(mock_data) =
            mock_inner.mocked_calls.get_mut(&Address::from_slice(callee.as_bytes()))
        {
            if let Some(return_data_queue) = match mock_data.get_mut(&ctx) {
                Some(found) => Some(found),
                None => mock_data
                    .iter_mut()
                    .find(|(key, _)| {
                        ctx.calldata.starts_with(&key.calldata)
                            && (key.value.is_none()
                                || ctx.value == key.value
                                || (ctx.value == Some(U256::ZERO) && key.value.is_none()))
                    })
                    .map(|(_, v)| v),
            } && let Some(return_data) = if return_data_queue.len() == 1 {
                // If the mocked calls stack has a single element in it, don't empty it
                return_data_queue.front().map(|x| x.to_owned())
            } else {
                // Else, we pop the front element
                return_data_queue.pop_front()
            } {
                return Some(ExecReturnValue {
                    flags: if matches!(return_data.ret_type, InstructionResult::Revert) {
                        ReturnFlags::REVERT
                    } else {
                        ReturnFlags::default()
                    },
                    data: return_data.data.0.to_vec(),
                });
            }
        };
        None
    }

    fn mock_caller(&self, frames_len: usize) -> Option<OriginFor<Runtime>> {
        let mock_inner = self.inner.borrow();
        if frames_len == 0 && mock_inner.delegated_caller.is_none() {
            return Some(mock_inner.caller.clone());
        }
        None
    }

    fn mock_origin(&self) -> Option<&ExecOrigin<Runtime>> {
        Some(&self.origin)
    }

    fn mock_delegated_caller(
        &self,
        dest: H160,
        input_data: &[u8],
    ) -> Option<DelegateInfo<Runtime>> {
        let mock_inner = self.inner.borrow();

        // Mocked functions are implemented by making use of the hooks for delegated calls.
        if let Some(mocked_function) =
            mock_inner.mocked_functions.get(&Address::from_slice(dest.as_bytes()))
        {
            let input_data = Bytes::from(input_data.to_vec());
            if let Some(target) = mocked_function
                .get(&input_data)
                .or_else(|| input_data.get(..4).and_then(|selector| mocked_function.get(selector)))
            {
                return Some(DelegateInfo {
                    caller:
        ExecOrigin::<Runtime>::from_runtime_origin(OriginFor::<Runtime>::signed(
                        <revive_env::Runtime as
        polkadot_sdk::pallet_revive::Config>::AddressMapper::to_account_id(&dest),
                    )).ok()?,
                callee: H160::from_slice(target.as_slice())
                }
                );
            }
        }

        mock_inner.delegated_caller.as_ref().and_then(|delegate_caller| {
            Some(DelegateInfo {
                caller: ExecOrigin::<Runtime>::from_runtime_origin(delegate_caller.clone()).ok()?,
                callee: mock_inner.callee,
            })
        })
    }
}

// Internal struct that holds the mock state. It is wrapped in an Arc<Mutex<>> in MockHandlerImpl
// to make it easier to transfer the state into Revive and back and be able to mutate it from the
// MockHandler trait methods.
#[derive(Clone)]
struct MockHandlerInner<T: frame_system::Config + pallet_revive::Config> {
    pub caller: OriginFor<T>,
    pub delegated_caller: Option<OriginFor<T>>,
    pub callee: H160,

    pub mocked_calls: HashMap<Address, BTreeMap<MockCallDataContext, VecDeque<MockCallReturnData>>>,
    pub mocked_functions: HashMap<Address, HashMap<Bytes, Address>>,
}

impl MockHandlerInner<Runtime> {
    /// Creates a new MockHandlerInner from the given Ecx and Cheatcodes state.
    /// Also returns whether a prank is currently enabled.
    fn new(
        _ecx: &Ecx<'_, '_, '_>,
        caller: &Address,
        target_address: Option<&Address>,
        callee: Option<&Address>,
        state: &mut foundry_cheatcodes::Cheatcodes,
    ) -> Self {
        let pranked_caller = OriginFor::<Runtime>::signed(AccountId::to_fallback_account_id(
            &H160::from_slice(caller.as_slice()),
        ));

        let delegated_caller = target_address.map(|addr| {
            OriginFor::<Runtime>::signed(AccountId::to_fallback_account_id(&H160::from_slice(
                addr.as_slice(),
            )))
        });

        Self {
            caller: pranked_caller,
            delegated_caller,
            mocked_calls: state.mocked_calls.clone(),
            callee: callee.map(|addr| H160::from_slice(addr.as_slice())).unwrap_or_default(),
            mocked_functions: state.mocked_functions.clone(),
        }
    }
}
