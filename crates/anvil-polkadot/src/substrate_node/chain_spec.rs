use crate::substrate_node::genesis::GenesisConfig;
use codec::{Decode, Encode};
use polkadot_sdk::{
    sc_chain_spec::{ChainSpec, GetExtension, json_patch},
    sc_executor,
    sc_executor::HostFunctions,
    sc_network::config::MultiaddrWithPeerId,
    sc_service::{ChainType, GenericChainSpec, Properties},
    sc_telemetry::TelemetryEndpoints,
    sp_core::{
        storage::Storage,
        traits::{CallContext, CodeExecutor, Externalities, FetchRuntimeCode, RuntimeCode},
    },
    sp_genesis_builder::Result as BuildResult,
    sp_io::{self, hashing::blake2_256},
    sp_runtime::BuildStorage,
    sp_state_machine::BasicExternalities,
};
use serde_json::Value;
use std::borrow::Cow;

pub fn development_chain_spec(
    genesis_config: GenesisConfig,
) -> Result<DevelopmentChainSpec, String> {
    let inner = GenericChainSpec::builder(&genesis_config.code, Default::default())
        .with_name("Development")
        .with_id("dev")
        .with_chain_type(ChainType::Development)
        .with_properties(props())
        .build();
    Ok(DevelopmentChainSpec { inner, genesis_config })
}

/// This is a wrapper around the general Substrate ChainSpec type that allows manual changes to the
/// genesis block.
#[derive(Clone)]
pub struct DevelopmentChainSpec<E = Option<()>, EHF = ()> {
    inner: GenericChainSpec<E, EHF>,
    genesis_config: GenesisConfig,
}

impl<E, EHF> BuildStorage for DevelopmentChainSpec<E, EHF>
where
    EHF: HostFunctions,
    GenericChainSpec<E, EHF>: BuildStorage,
{
    fn assimilate_storage(&self, storage: &mut Storage) -> Result<(), String> {
        storage.top.extend(self.genesis_config.as_storage_key_value());

        // We need to initialise the storage used when calling into the runtime for the genesis
        // config, so that the customised items (like block number and timestamp) will be
        // seen even in the code that processes the genesis config patch.
        let temp_storage = storage.clone();

        GenesisBuilderRuntimeCaller::<EHF>::new(&self.genesis_config.code[..])
            .get_storage_for_patch(
                self.genesis_config.runtime_genesis_config_patch(),
                temp_storage,
            )?
            .assimilate_storage(storage)?;

        Ok(())
    }
}

impl<E, EHF> ChainSpec for DevelopmentChainSpec<E, EHF>
where
    E: GetExtension + serde::Serialize + Clone + Send + Sync + 'static,
    EHF: HostFunctions,
{
    fn boot_nodes(&self) -> &[MultiaddrWithPeerId] {
        self.inner.boot_nodes()
    }

    fn name(&self) -> &str {
        self.inner.name()
    }

    fn id(&self) -> &str {
        self.inner.id()
    }

    fn chain_type(&self) -> ChainType {
        self.inner.chain_type()
    }

    fn telemetry_endpoints(&self) -> &Option<TelemetryEndpoints> {
        self.inner.telemetry_endpoints()
    }

    fn protocol_id(&self) -> Option<&str> {
        self.inner.protocol_id()
    }

    fn fork_id(&self) -> Option<&str> {
        self.inner.fork_id()
    }

    fn properties(&self) -> Properties {
        self.inner.properties()
    }

    fn add_boot_node(&mut self, addr: MultiaddrWithPeerId) {
        self.inner.add_boot_node(addr)
    }

    fn extensions(&self) -> &dyn GetExtension {
        self.inner.extensions() as &dyn GetExtension
    }

    fn extensions_mut(&mut self) -> &mut dyn GetExtension {
        self.inner.extensions_mut() as &mut dyn GetExtension
    }

    fn as_json(&self, raw: bool) -> Result<String, String> {
        self.inner.as_json(raw)
    }

    fn as_storage_builder(&self) -> &dyn BuildStorage {
        self
    }

    fn cloned_box(&self) -> Box<dyn ChainSpec> {
        Box::new(Self { inner: self.inner.clone(), genesis_config: self.genesis_config.clone() })
    }

    fn set_storage(&mut self, storage: Storage) {
        self.inner.set_storage(storage);
    }

    fn code_substitutes(&self) -> std::collections::BTreeMap<String, Vec<u8>> {
        self.inner.code_substitutes()
    }
}

fn props() -> Properties {
    let mut properties = Properties::new();
    properties.insert("tokenDecimals".to_string(), 12.into());
    properties.insert("tokenSymbol".to_string(), "DOT".into());
    properties
}

// This mostly copies the upstream `GenesisConfigBuilderRuntimeCaller`, but with the ability of
// injecting genesis state even before the genesis config builders in the runtime are run via
// `GenesisBuilder_build_state`
struct GenesisBuilderRuntimeCaller<'a, EHF = ()>
where
    EHF: HostFunctions,
{
    code: Cow<'a, [u8]>,
    code_hash: Vec<u8>,
    executor: sc_executor::WasmExecutor<(sp_io::SubstrateHostFunctions, EHF)>,
}

impl<'a, EHF> FetchRuntimeCode for GenesisBuilderRuntimeCaller<'a, EHF>
where
    EHF: HostFunctions,
{
    fn fetch_runtime_code(&self) -> Option<Cow<'_, [u8]>> {
        Some(self.code.as_ref().into())
    }
}

impl<'a, EHF> GenesisBuilderRuntimeCaller<'a, EHF>
where
    EHF: HostFunctions,
{
    fn new(code: &'a [u8]) -> Self {
        GenesisBuilderRuntimeCaller {
            code: code.into(),
            code_hash: blake2_256(code).to_vec(),
            executor: sc_executor::WasmExecutor::<(sp_io::SubstrateHostFunctions, EHF)>::builder()
                .with_allow_missing_host_functions(true)
                .build(),
        }
    }

    fn get_storage_for_patch(
        &self,
        patch: Value,
        genesis_storage: Storage,
    ) -> core::result::Result<Storage, String> {
        let mut config = self.get_named_preset(None)?;
        json_patch::merge(&mut config, patch);
        self.get_storage_for_config(config, genesis_storage)
    }

    fn call(
        &self,
        ext: &mut dyn Externalities,
        method: &str,
        data: &[u8],
    ) -> sc_executor::error::Result<Vec<u8>> {
        self.executor
            .call(
                ext,
                &RuntimeCode { heap_pages: None, code_fetcher: self, hash: self.code_hash.clone() },
                method,
                data,
                CallContext::Offchain,
            )
            .0
    }

    fn get_named_preset(&self, id: Option<&String>) -> core::result::Result<Value, String> {
        let mut t = BasicExternalities::new_empty();
        let call_result = self
            .call(&mut t, "GenesisBuilder_get_preset", &id.encode())
            .map_err(|e| format!("wasm call error {e}"))?;

        let named_preset = Option::<Vec<u8>>::decode(&mut &call_result[..])
            .map_err(|e| format!("scale codec error: {e}"))?;

        if let Some(named_preset) = named_preset {
            Ok(serde_json::from_slice(&named_preset[..]).expect("returned value is json. qed."))
        } else {
            Err(format!("The preset with name {id:?} is not available."))
        }
    }

    fn get_storage_for_config(
        &self,
        config: Value,
        genesis_storage: Storage,
    ) -> core::result::Result<Storage, String> {
        // This is the key difference compared to the upstream variant, we don't initialise the
        // storage as empty.
        let mut ext = BasicExternalities::new(genesis_storage);

        let json_pretty_str = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("json to string failed: {e}"))?;

        let call_result = self
            .call(&mut ext, "GenesisBuilder_build_state", &json_pretty_str.encode())
            .map_err(|e| format!("wasm call error {e}"))?;

        BuildResult::decode(&mut &call_result[..])
            .map_err(|e| format!("scale codec error: {e}"))?
            .map_err(|e| format!("{e} for blob:\n{json_pretty_str}"))?;

        Ok(ext.into_storages())
    }
}
