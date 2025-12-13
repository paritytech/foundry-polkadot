use alloy_primitives::{B256, Bytes, U256 as RU256, hex};
use foundry_cheatcodes::ExpectedCreate;
use foundry_compilers::resolc::dual_compiled_contracts::DualCompiledContracts;
use itertools::Itertools;
use polkadot_sdk::{
    pallet_revive::{AccountInfo, Code, Pallet, tracing::Tracing},
    sp_core::{H160, U256},
    sp_weights::Weight,
};
use revive_env::Runtime;
use revm::context::CreateScheme;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct Create {
    pub(crate) addr: H160,
    pub(crate) from: H160,
    scheme: CreateScheme,
}

#[derive(Debug)]
pub(crate) struct CreateTracer {
    pub calls: Vec<polkadot_sdk::sp_core::H160>,
    pub is_create: Option<(Code, Option<[u8; 32]>)>,
    pub call_types: Vec<Type>,
    pub data: Vec<ExpectedCreate>,
    creates: Vec<Create>,
}

#[derive(Debug)]
pub enum Type {
    Create { salt: Option<[u8; 32]> },
    Rest,
}

impl CreateTracer {
    fn current_addr(&self) -> H160 {
        self.calls.last().copied().unwrap_or_default()
    }
    pub fn new(data: Vec<ExpectedCreate>) -> Self {
        Self { data, creates: vec![], calls: vec![], is_create: None, call_types: vec![] }
    }
}

impl Tracing for CreateTracer {
    fn instantiate_code(&mut self, code: &Code, salt: Option<&[u8; 32]>) {
        self.is_create = Some((code.to_owned(), salt.to_owned().copied()));
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
        self.call_types.push(if let Some((_, salt)) = self.is_create.take() {
            Type::Create { salt }
        } else {
            Type::Rest
        });
        if self.calls.is_empty() {
            self.calls.push(_from);
        }
        self.calls.push(if _is_delegate_call { self.current_addr() } else { to });
    }

    fn exit_child_span(
        &mut self,
        _output: &polkadot_sdk::pallet_revive::ExecReturnValue,
        _gas_left: Weight,
    ) {
        let addr = self.calls.pop().unwrap_or_default();

        let typ = self.call_types.pop();
        if typ.as_ref().is_some_and(|x| matches!(x, Type::Create { .. })) {
            self.is_create = None;
        }
        if let Some(Type::Create { salt, .. }) = typ {
            let mut create =
                Create { addr, from: self.current_addr(), scheme: CreateScheme::Create };
            if let Some(salt) = salt {
                let salt = RU256::from_be_bytes(
                    B256::from_slice(&alloy_primitives::keccak256::<&[u8]>(&salt)[..]).0,
                );
                create.scheme = CreateScheme::Create2 { salt };
            };
            self.creates.push(create);
        }
    }
}

impl CreateTracer {
    pub fn finalize(
        &mut self,
        dual_compiled_contracts: &DualCompiledContracts,
    ) -> Vec<ExpectedCreate> {
        let mut created = vec![];
        for c in self.creates.iter().cloned() {
            let Some(info) = AccountInfo::<Runtime>::load_contract(&c.addr) else {
                continue;
            };
            let hash = hex::encode(info.code_hash);
            let code = Pallet::<Runtime>::code(&c.addr);
            let cb = Bytes::from(code.clone()).0;
            let bytecode_result = dual_compiled_contracts
                .find_by_evm_bytecode_hash(hash.clone())
                .and_then(|(_, contract)| contract.evm_deployed_bytecode.as_bytes())
                .or_else(|| {
                    dual_compiled_contracts
                        .find_bytecode(&cb)
                        .and_then(|f| f.contract().evm_deployed_bytecode.as_bytes())
                })
                .cloned()
                .unwrap_or_else(|| Bytes::from(code.clone()));
            created.push(ExpectedCreate {
                deployer: c.from.0.into(),
                bytecode: bytecode_result,
                create_scheme: c.scheme.into(),
            })
        }
        for c in created {
            if let Some((index, _)) = self.data.iter().find_position(|expected_create| {
                expected_create.deployer == c.deployer
                    && expected_create.create_scheme.eq(c.create_scheme.clone())
                    && expected_create.bytecode == c.bytecode
            }) {
                self.data.swap_remove(index);
            }
        }
        self.data.clone()
    }
}
