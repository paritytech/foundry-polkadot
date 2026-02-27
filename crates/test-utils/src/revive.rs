use eyre::Result;
use std::{process, thread::sleep, time::Duration};
use subxt::{OnlineClient, PolkadotConfig};
use tempfile::TempDir;

const NODE_BINARY: &str = "anvil-polkadot";
const MAX_ATTEMPTS: u32 = 15;
const RPC_URL: &str = "http://127.0.0.1:8545";
const RETRY_DELAY: Duration = Duration::from_secs(1);
pub static GENESIS_JSON: &str = include_str!("../test-data/genesis.json");

const WALLETS: [(&str, &str); 1] = [(
    "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
    "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
)];

/// Spawn and manage an instance of a compatible contracts enabled chain node.
#[allow(dead_code)]
pub struct AnvilPolkadotNode {
    node: process::Child,
    tmp_dir: tempfile::TempDir,
}

impl Drop for AnvilPolkadotNode {
    fn drop(&mut self) {
        self.kill()
    }
}

impl AnvilPolkadotNode {
    pub async fn start() -> Result<Self> {
        let tmp_dir = TempDir::with_prefix("cargo-contract.cli.test.node")?;
        std::fs::write(tmp_dir.path().join("genesis.json"), GENESIS_JSON).unwrap();
        let mut node = process::Command::new(NODE_BINARY)
            .env("RUST_LOG", "error")
            .args([
                "--init",
                tmp_dir.path().join("genesis.json").to_str().unwrap(),
                "--ipc",
                tmp_dir.path().join("anvil.ipc").to_str().unwrap(),
                "--balance",
                u64::MAX.to_string().as_str(),
            ])
            .spawn()?;
        // wait for rpc to be initialized
        let mut attempts = 1;
        loop {
            sleep(RETRY_DELAY);
            tracing::debug!(
                "Connecting to contracts enabled node, attempt {}/{}",
                attempts,
                MAX_ATTEMPTS
            );
            match OnlineClient::<PolkadotConfig>::new().await {
                Result::Ok(_) => return Ok(Self { node, tmp_dir }),
                Err(err) => {
                    if attempts < MAX_ATTEMPTS {
                        attempts += 1;
                        continue;
                    }
                    let err = eyre::eyre!(
                        "Failed to connect to node rpc after {} attempts: {}",
                        attempts,
                        err
                    );
                    tracing::error!("{}", err);
                    node.kill()?;
                    return Err(err);
                }
            }
        }
    }

    fn kill(&mut self) {
        tracing::debug!("Killing contracts node process {}", self.node.id());
        if let Err(err) = self.node.kill() {
            tracing::error!("Error killing contracts node process {}: {}", self.node.id(), err)
        }
    }

    pub fn http_endpoint() -> &'static str {
        RPC_URL
    }

    pub fn dev_accounts() -> impl Iterator<Item = (&'static str, &'static str)> {
        WALLETS.iter().copied()
    }
}
