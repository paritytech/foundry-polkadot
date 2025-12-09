use foundry_compilers::{
    ProjectPathsConfig, error::SolcError, multi::MultiCompilerLanguage, resolc::ResolcSettings,
    solc::SolcSettings,
};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};

use crate::{Config, SolcReq};

/// Polkadot execution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PolkadotMode {
    Evm,
    Pvm,
}

impl FromStr for PolkadotMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "evm" => Ok(Self::Evm),
            "pvm" => Ok(Self::Pvm),
            "" => Ok(Self::Evm), // Default when --polkadot with no value
            _ => Err(format!("Invalid polkadot mode: {s}. Use 'evm' or 'pvm'")),
        }
    }
}

impl Display for PolkadotMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Evm => write!(f, "evm"),
            Self::Pvm => write!(f, "pvm"),
        }
    }
}

/// Filename for resolc cache
pub const RESOLC_SOLIDITY_FILES_CACHE_FILENAME: &str = "resolc-solidity-files-cache.json";

/// Name of the subdirectory for solc artifacts in dual compilation mode
pub const SOLC_ARTIFACTS_SUBDIR: &str = "solc";

pub const CONTRACT_SIZE_LIMIT: usize = 250_000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Default, Deserialize)]
/// Resolc Config
pub struct PolkadotConfig {
    /// Enable compilation using resolc
    pub resolc_compile: bool,

    /// Use pallet-revive runtime backend
    pub polkadot: Option<PolkadotMode>,

    /// The resolc compiler
    pub resolc: Option<SolcReq>,

    /// The optimization mode string for resolc
    pub optimizer_mode: Option<char>,

    /// The emulated EVM linear heap memory static buffer size in bytes
    pub heap_size: Option<u32>,

    /// The contracts total stack size in bytes
    pub stack_size: Option<u32>,

    /// Generate source based debug information in the output code file
    pub debug_information: Option<bool>,
}

impl PolkadotConfig {
    /// Returns the `ProjectPathsConfig` sub set of the config.
    pub fn project_paths(config: &Config) -> ProjectPathsConfig<MultiCompilerLanguage> {
        let builder = ProjectPathsConfig::builder()
            .cache(config.cache_path.join(RESOLC_SOLIDITY_FILES_CACHE_FILENAME))
            .sources(&config.src)
            .tests(&config.test)
            .scripts(&config.script)
            .libs(config.libs.iter())
            .remappings(config.get_all_remappings())
            .allowed_path(&config.root)
            .allowed_paths(&config.libs)
            .allowed_paths(&config.allow_paths)
            .include_paths(&config.include_paths)
            .artifacts(&config.out);

        builder.build_with_root(&config.root)
    }

    pub fn resolc_settings(config: &Config) -> Result<SolcSettings, SolcError> {
        config.solc_settings().map(|mut s| {
            s.extra_settings = ResolcSettings::new(
                config.polkadot.optimizer_mode,
                config.polkadot.heap_size,
                config.polkadot.stack_size,
                config.polkadot.debug_information,
            );
            s
        })
    }
}
