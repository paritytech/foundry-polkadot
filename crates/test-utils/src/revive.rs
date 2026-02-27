use eyre::Result;
use std::{
    process::{self, Stdio},
    time::Duration,
};
use tempfile::TempDir;
use tokio::net::TcpStream;

use crate::util::OutputExt;

const NODE_BINARY: &str = "anvil-polkadot";
const MAX_ATTEMPTS: u32 = 15;
const RPC_URL: &str = "http://127.0.0.1";
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
    port: u16,
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

        // wait for rpc to be initialized
        let mut attempts = 1;
        let mut node = process::Command::new(NODE_BINARY)
            .env("RUST_LOG", "error")
            .args([
                "--init",
                tmp_dir.path().join("genesis.json").to_str().unwrap(),
                "--ipc",
                tmp_dir.path().join("anvil.ipc").to_str().unwrap(),
                "--balance",
                u64::MAX.to_string().as_str(),
                "--port",
                "0",
            ])
            .spawn()?;

        loop {
            let id = node.id();
            let mut port_val: Option<String> = None;
            if port_val.is_none() {
                port_val = tokio::task::spawn_blocking(move || {
                    let lsof_out = process::Command::new("lsof")
                        .args(["-i", "-P"])
                        .stdout(Stdio::piped())
                        .spawn()
                        .unwrap()
                        .stdout;
                    let grepped_output = process::Command::new("grep")
                        .arg(id.to_string().as_str())
                        .stdin(lsof_out.unwrap())
                        .output()
                        .unwrap()
                        .stdout_lossy();
                    grepped_output
                        .lines()
                        .filter(|x| x.contains("IPv4"))
                        .flat_map(|x| x.split_whitespace())
                        .filter(|x| x.contains("localhost") && !x.contains(":9944"))
                        .next()
                        .and_then(|x| x.split(":").last())
                        .map(|x| x.to_owned())
                })
                .await
                .ok()
                .flatten();

                if port_val.is_none() {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    continue;
                }
            };
            let Some(port) = port_val else { continue };

            tracing::debug!(
                "Connecting to contracts enabled node, attempt {}/{}",
                attempts,
                MAX_ATTEMPTS
            );
            match tokio::time::timeout(
                Duration::from_millis(200),
                TcpStream::connect(&format!("{}:{}", "127.0.0.1", port)),
            )
            .await
            {
                Result::Ok(Result::Ok(t)) => {
                    drop(t);
                    return Ok(Self {
                        node,
                        tmp_dir,
                        port: u16::from_str_radix(&port, 10).unwrap(),
                    });
                }
                Err(_) => continue,
                Result::Ok(Err(err)) => {
                    if attempts < MAX_ATTEMPTS {
                        attempts += 1;
                        tokio::time::sleep(RETRY_DELAY).await;
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

    pub fn http_endpoint(&self) -> String {
        format!("{}:{}", RPC_URL, self.port)
    }

    pub fn dev_accounts() -> impl Iterator<Item = (&'static str, &'static str)> {
        WALLETS.iter().copied()
    }
}
