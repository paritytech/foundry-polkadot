use crate::{
    cmd::NodeArgs,
    substrate_node::{chain_spec, genesis::GenesisConfig},
};
use alloy_primitives::U256;
use clap::{Parser, Subcommand};
use foundry_cli::opts::GlobalArgs;
use foundry_common::version::{LONG_VERSION, SHORT_VERSION};
use polkadot_sdk::{sc_cli, sc_service};
use subxt_signer::eth::Keypair;

#[derive(Parser)]
#[command(name = "anvil-polkadot", version = SHORT_VERSION, long_version = LONG_VERSION, next_display_order = None)]
pub struct Anvil {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(flatten)]
    pub node: NodeArgs,

    #[command(subcommand)]
    pub cmd: Option<AnvilSubcommand>,
}

#[derive(Subcommand)]
pub enum AnvilSubcommand {
    /// Generate shell completions script.
    #[command(visible_alias = "com")]
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },

    /// Generate Fig autocompletion spec.
    #[command(visible_alias = "fig")]
    GenerateFigSpec,
}

pub struct SubstrateCli {
    // Used to inject the anvil config into the chain spec
    genesis_config: GenesisConfig,
    // Signing accounts
    signers: Vec<Keypair>,
    // Genesis Balance
    balance: U256,
}

impl SubstrateCli {
    pub fn new(genesis_config: GenesisConfig, signers: Vec<Keypair>, balance: U256) -> Self {
        Self { genesis_config, signers, balance }
    }
}

// Implementation of the SubstrateCli, which enables us to launch an in-process substrate node.
impl sc_cli::SubstrateCli for SubstrateCli {
    fn impl_name() -> String {
        "Anvil Substrate Node".into()
    }

    fn impl_version() -> String {
        SHORT_VERSION.into()
    }

    fn description() -> String {
        "Anvil Substrate Node".into()
    }

    fn author() -> String {
        "Anvil Polkadot Developers".into()
    }

    fn support_url() -> String {
        "https://github.com/paritytech/foundry-polkadot/issues".into()
    }

    fn copyright_start_year() -> i32 {
        2025
    }

    fn executable_name() -> String {
        "anvil-polkadot".into()
    }

    fn load_spec(&self, _: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        Ok(Box::new(chain_spec::development_chain_spec(
            self.genesis_config.clone(),
            &self.signers,
            self.balance,
        )?))
    }
}
