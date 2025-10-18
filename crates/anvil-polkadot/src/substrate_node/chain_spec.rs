use crate::substrate_node::genesis::GenesisConfig;
use alloy_signer_local::PrivateKeySigner;
use polkadot_sdk::{
    frame_support,
    pallet_revive::evm::Account,
    sc_chain_spec::{ChainSpec, GetExtension},
    sc_executor::HostFunctions,
    sc_network::config::MultiaddrWithPeerId,
    sc_service::{ChainType, GenericChainSpec, Properties},
    sc_telemetry::TelemetryEndpoints,
    sp_core::storage::Storage,
    sp_runtime::{AccountId32, BuildStorage},
};
use std::collections::BTreeMap;
use substrate_runtime::{
    BalancesConfig, RuntimeGenesisConfig, WASM_BINARY, genesis_config_presets::ENDOWMENT,
};
use subxt_signer::eth::Keypair;

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
        self.inner.assimilate_storage(storage)?;
        storage.top.extend(self.genesis_config.as_storage_key_value());
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
    properties.insert("tokenSymbol".to_string(), "MINI".into());
    properties
}

pub fn development_chain_spec(
    genesis_config: GenesisConfig,
    signers: &[PrivateKeySigner],
) -> Result<DevelopmentChainSpec, String> {
    let mut balance_map: BTreeMap<AccountId32, u128> = keypairs_from_private_keys(signers)?
        .into_iter()
        .map(|keypair| (Account::from(keypair).substrate_account(), ENDOWMENT))
        .collect();
    if let Some(alloc) = &genesis_config.alloc {
        balance_map.extend(alloc.iter().map(|(eth_addr, gen_account)| {
            let mut substrate_account = AccountId32::new([0xEE; 32]);
            <AccountId32 as AsMut<[u8; 32]>>::as_mut(&mut substrate_account)[..20]
                .copy_from_slice(eth_addr.as_slice());
            let balance = gen_account.balance.try_into().unwrap_or(ENDOWMENT);
            (substrate_account, balance)
        }));
    }
    let balances: Vec<(AccountId32, u128)> = balance_map.into_iter().collect();
    let inner = GenericChainSpec::builder(
        WASM_BINARY.expect("Development wasm not available"),
        Default::default(),
    )
    .with_name("Development")
    .with_id("dev")
    .with_chain_type(ChainType::Development)
    .with_genesis_config_patch(frame_support::build_struct_json_patch!(RuntimeGenesisConfig {
        balances: BalancesConfig { balances }
    }))
    .with_properties(props())
    .build();
    Ok(DevelopmentChainSpec { inner, genesis_config })
}

pub fn keypairs_from_private_keys(accounts: &[PrivateKeySigner]) -> Result<Vec<Keypair>, String> {
    accounts
        .iter()
        .map(|signer| {
            let key =
                subxt_signer::eth::Keypair::from_secret_key(signer.credential().to_bytes().into())
                    .map_err(|e| e.to_string())?;
            Ok(key)
        })
        .collect()
}
