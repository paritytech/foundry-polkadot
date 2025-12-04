use polkadot_sdk::{
    pallet_revive::{Code, tracing::Tracing},
    sp_core::{H160, U256},
    sp_weights::Weight,
};

#[derive(Debug)]
pub(crate) struct RevertTracer {
    pub max_depth: usize,
    pub has_reverted: Option<polkadot_sdk::sp_core::H160>,
    pub calls: Vec<polkadot_sdk::sp_core::H160>,
    pub is_create: bool,
    pub call_types: Vec<Type>,
}

#[derive(Debug)]
pub enum Type {
    Create,
    Rest,
}

impl RevertTracer {
    fn current_addr(&self) -> H160 {
        self.calls.last().copied().unwrap_or_default()
    }
    pub fn new() -> Self {
        Self {
            max_depth: 1,
            has_reverted: None,
            calls: vec![],
            is_create: false,
            call_types: vec![],
        }
    }
}

impl Tracing for RevertTracer {
    fn instantiate_code(&mut self, _code: &Code, _salt: Option<&[u8; 32]>) {
        self.is_create = true;
    }

    fn enter_child_span(
        &mut self,
        _from: H160,
        to: H160,
        _is_delegate_call: bool,
        _is_read_only: bool,
        _value: U256,
        _input: &[u8],
        _gas: Weight,
    ) {
        self.call_types.push(if self.is_create { Type::Create } else { Type::Rest });

        self.calls.push(if self.call_types.last().is_some_and(|x| matches!(x, Type::Create)) {
            self.current_addr()
        } else {
            to
        });

        if self.has_reverted.is_none() {
            self.max_depth += 1;
        }
    }

    fn exit_child_span(
        &mut self,
        output: &polkadot_sdk::pallet_revive::ExecReturnValue,
        _gas_left: Weight,
    ) {
        let addr = self.calls.pop().unwrap_or_default();

        if output.did_revert() && self.has_reverted.is_none() {
            self.has_reverted = Some(addr);
        }
        let typ = self.call_types.pop();
        if typ.is_some_and(|x| matches!(x, Type::Create)) {
            self.is_create = false;
        }
        if self.has_reverted.is_none() {
            self.max_depth -= 1;
        }
    }
}
