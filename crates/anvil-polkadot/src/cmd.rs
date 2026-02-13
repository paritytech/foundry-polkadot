use crate::config::{AccountGenerator, AnvilNodeConfig, DEFAULT_MNEMONIC, SubstrateNodeConfig};
use alloy_genesis::Genesis;
use alloy_primitives::{U256, utils::Unit};
use alloy_signer_local::coins_bip39::{English, Mnemonic};
use anvil_server::ServerConfig;
use clap::Parser;
use foundry_common::shell;
use foundry_config::Chain;
use rand_08::{SeedableRng, rngs::StdRng};
use std::{net::IpAddr, path::PathBuf, time::Duration};

#[derive(Clone, Debug, Parser)]
pub struct NodeArgs {
    /// Port number to listen on.
    #[arg(long, short, default_value = "8545", value_name = "NUM")]
    pub port: u16,

    /// Number of dev accounts to generate and configure.
    #[arg(long, short, default_value = "10", value_name = "NUM")]
    pub accounts: u64,

    /// The balance of every dev account in Ether.
    #[arg(long, default_value = "10000", value_name = "NUM")]
    pub balance: u64,

    /// The timestamp of the genesis block.
    #[arg(long, value_name = "NUM")]
    pub timestamp: Option<u64>,

    /// The number of the genesis block.
    #[arg(long, value_name = "NUM")]
    pub number: Option<u64>,

    /// BIP39 mnemonic phrase used for generating accounts.
    /// Cannot be used if `mnemonic_random` or `mnemonic_seed` are used.
    #[arg(long, short, conflicts_with_all = &["mnemonic_seed", "mnemonic_random"])]
    pub mnemonic: Option<String>,

    /// Automatically generates a BIP39 mnemonic phrase, and derives accounts from it.
    /// Cannot be used with other `mnemonic` options.
    /// You can specify the number of words you want in the mnemonic.
    /// [default: 12]
    #[arg(long, conflicts_with_all = &["mnemonic", "mnemonic_seed"], default_missing_value = "12", num_args(0..=1))]
    pub mnemonic_random: Option<usize>,

    /// Generates a BIP39 mnemonic phrase from a given seed
    /// Cannot be used with other `mnemonic` options.
    ///
    /// CAREFUL: This is NOT SAFE and should only be used for testing.
    /// Never use the private keys generated in production.
    #[arg(long = "mnemonic-seed-unsafe", conflicts_with_all = &["mnemonic", "mnemonic_random"])]
    pub mnemonic_seed: Option<u64>,

    /// Sets the derivation path of the child key to be derived.
    ///
    /// [default: m/44'/60'/0'/0/]
    #[arg(long)]
    pub derivation_path: Option<String>,

    /// Block time in seconds for interval mining.
    #[arg(short, long, visible_alias = "blockTime", value_name = "SECONDS", value_parser = duration_from_secs_f64)]
    pub block_time: Option<Duration>,

    /// Writes output of `anvil` as json to user-specified file.
    #[arg(long, value_name = "FILE", value_hint = clap::ValueHint::FilePath)]
    pub config_out: Option<PathBuf>,

    /// Disable auto and interval mining, and mine on demand instead.
    #[arg(long, visible_alias = "no-mine", conflicts_with = "block_time")]
    pub no_mining: bool,

    #[arg(long, visible_alias = "mixed-mining", requires = "block_time")]
    pub mixed_mining: bool,

    /// The hosts the server will listen on.
    #[arg(
        long,
        value_name = "IP_ADDR",
        env = "ANVIL_IP_ADDR",
        default_value = "127.0.0.1",
        help_heading = "Server options",
        value_delimiter = ','
    )]
    pub host: Vec<IpAddr>,

    /// Initialize the genesis block with the given `genesis.json` file.
    #[arg(long, value_name = "PATH", value_parser= read_genesis_file)]
    pub init: Option<Genesis>,

    #[arg(long, help = IPC_HELP, value_name = "PATH", visible_alias = "ipcpath")]
    pub ipc: Option<Option<String>>,

    /// Max number of blocks to keep in memory for the eth revive rpc
    #[arg(long, visible_alias = "transaction-block-keeper")]
    pub revive_rpc_block_limit: Option<usize>,

    #[command(flatten)]
    pub evm: AnvilEvmArgs,

    #[command(flatten)]
    pub server_config: ServerConfig,
}

/// The default IPC endpoint
const IPC_HELP: &str = "Launch an ipc server at the given path or default path = `/tmp/anvil.ipc`";

impl NodeArgs {
    pub fn into_node_config(self) -> eyre::Result<(AnvilNodeConfig, SubstrateNodeConfig)> {
        let genesis_balance = Unit::ETHER.wei().saturating_mul(U256::from(self.balance));

        let anvil_config = AnvilNodeConfig::default()
            .with_blocktime(self.block_time)
            .with_no_mining(self.no_mining)
            .with_mixed_mining(self.mixed_mining, self.block_time)
            .with_account_generator(self.account_generator())?
            .with_genesis_balance(genesis_balance)
            .with_genesis_timestamp(self.timestamp)
            .with_genesis_block_number(self.number)
            .with_port(self.port)
            .with_base_fee(self.evm.block_base_fee_per_gas)
            .with_server_config(self.server_config)
            .with_host(self.host)
            .set_silent(shell::is_quiet())
            .set_config_out(self.config_out)
            .with_chain_id(self.evm.chain_id)
            .with_genesis(self.init)
            .with_auto_impersonate(self.evm.auto_impersonate)
            .with_ipc(self.ipc)
            .with_revive_rpc_block_limit(self.revive_rpc_block_limit);

        let substrate_node_config = SubstrateNodeConfig::new(&anvil_config);

        Ok((anvil_config, substrate_node_config))
    }

    fn account_generator(&self) -> AccountGenerator {
        let mut rng_gen = AccountGenerator::new(self.accounts as usize).phrase(DEFAULT_MNEMONIC);

        if let Some(ref mnemonic) = self.mnemonic {
            rng_gen = rng_gen.phrase(mnemonic);
        } else if let Some(count) = self.mnemonic_random {
            let mut rng = rand_08::thread_rng();
            let mnemonic = match Mnemonic::<English>::new_with_count(&mut rng, count) {
                Ok(mnemonic) => mnemonic.to_phrase(),
                Err(_) => DEFAULT_MNEMONIC.to_string(),
            };
            rng_gen = rng_gen.phrase(mnemonic);
        } else if let Some(seed) = self.mnemonic_seed {
            let mut seed = StdRng::seed_from_u64(seed);
            let mnemonic = Mnemonic::<English>::new(&mut seed).to_phrase();
            rng_gen = rng_gen.phrase(mnemonic);
        }
        if let Some(ref derivation) = self.derivation_path {
            rng_gen = rng_gen.derivation_path(derivation);
        }
        rng_gen
    }
}

/// Anvil's EVM related arguments.
#[derive(Clone, Debug, Parser)]
#[command(next_help_heading = "EVM options")]
pub struct AnvilEvmArgs {
    /// The base fee in a block.
    #[arg(
        long,
        visible_alias = "base-fee",
        value_name = "FEE",
        help_heading = "Environment config"
    )]
    pub block_base_fee_per_gas: Option<u64>,

    /// The chain ID.
    #[arg(long, alias = "chain", help_heading = "Environment config")]
    pub chain_id: Option<Chain>,

    /// Enables automatic impersonation on startup. This allows any transaction sender to be
    /// simulated as different accounts, which is useful for testing contract behavior.
    #[arg(long, visible_alias = "auto-unlock")]
    pub auto_impersonate: bool,
}

/// Clap's value parser for genesis. Loads a genesis.json file.
fn read_genesis_file(path: &str) -> Result<Genesis, String> {
    foundry_common::fs::read_json_file(path.as_ref()).map_err(|err| err.to_string())
}

fn duration_from_secs_f64(s: &str) -> Result<Duration, String> {
    let s = s.parse::<f64>().map_err(|e| e.to_string())?;
    if s == 0.0 {
        return Err("Duration must be greater than 0".to_string());
    }
    Duration::try_from_secs_f64(s).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, net::Ipv4Addr};

    #[test]
    fn can_parse_host() {
        let args = NodeArgs::parse_from(["anvil"]);
        assert_eq!(args.host, vec![IpAddr::V4(Ipv4Addr::LOCALHOST)]);

        let args = NodeArgs::parse_from([
            "anvil", "--host", "::1", "--host", "1.1.1.1", "--host", "2.2.2.2",
        ]);
        assert_eq!(
            args.host,
            ["::1", "1.1.1.1", "2.2.2.2"].map(|ip| ip.parse::<IpAddr>().unwrap()).to_vec()
        );

        let args = NodeArgs::parse_from(["anvil", "--host", "::1,1.1.1.1,2.2.2.2"]);
        assert_eq!(
            args.host,
            ["::1", "1.1.1.1", "2.2.2.2"].map(|ip| ip.parse::<IpAddr>().unwrap()).to_vec()
        );

        unsafe {
            env::set_var("ANVIL_IP_ADDR", "1.1.1.1");
        };
        let args = NodeArgs::parse_from(["anvil"]);
        assert_eq!(args.host, vec!["1.1.1.1".parse::<IpAddr>().unwrap()]);
        unsafe {
            env::set_var("ANVIL_IP_ADDR", "::1,1.1.1.1,2.2.2.2");
        };
        let args = NodeArgs::parse_from(["anvil"]);
        assert_eq!(
            args.host,
            ["::1", "1.1.1.1", "2.2.2.2"].map(|ip| ip.parse::<IpAddr>().unwrap()).to_vec()
        );
    }
}
