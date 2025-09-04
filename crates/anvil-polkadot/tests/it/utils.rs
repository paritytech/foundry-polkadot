use alloy_primitives::hex;
use anvil_core::eth::EthRequest;
use anvil_polkadot::{
    api_server::{self, ApiHandle},
    cmd::NodeArgs,
    opts::Anvil,
    spawn_anvil_tasks,
    substrate_node::service::{self, Service},
};
use anvil_rpc::response::ResponseResult;
use eyre::{Result, WrapErr};
use futures::channel::oneshot;
use parity_scale_codec::Decode;
use polkadot_sdk::{
    sc_cli::CliConfiguration,
    sc_client_api::{BlockBackend, HeaderBackend},
    sp_core::{storage::StorageKey, twox_128, H256},
};
use serde_json::{json, Value};

pub struct TestNode {
    pub service: Service,
    pub api: ApiHandle,
    pub _runtime_handle: tokio::runtime::Handle,
}

impl TestNode {
    pub async fn new() -> Result<Self> {
        let handle = tokio::runtime::Handle::current();

        let mut anvil_args = Anvil {
            global: foundry_cli::opts::GlobalArgs::default(),
            node: NodeArgs::default(),
            cmd: None,
        };

        anvil_args.node.no_mining = true;
        anvil_args.node.mixed_mining = false;
        anvil_args.node.port = 0; // auto-assign

        let (anvil_config, mut substrate_config) = anvil_args.node.clone().into_node_config()?;
        let anvil_config = anvil_config.set_silent(true);

        let temp_dir = tempfile::tempdir()?;
        let db_path = temp_dir.path().join("db");

        substrate_config.set_base_path(Some(db_path));
        let config = substrate_config.create_configuration(&anvil_args, handle.clone())?;
        let service = service::new(&anvil_config, config)?;
        let api = spawn_anvil_tasks(anvil_config, &service).await?;
        tokio::task::yield_now().await;

        Ok(Self { service, api, _runtime_handle: handle.clone() })
    }

    /// Send an `EthRequest` to your API server and await `ResponseResult`.
    pub async fn call_eth(&mut self, req: EthRequest) -> Result<ResponseResult> {
        let (tx, rx) = oneshot::channel();
        self.api
            .try_send(api_server::ApiRequest {
                req: req.clone(), // Clone for error reporting
                resp_sender: tx,
            })
            .map_err(|e| eyre::eyre!("failed to send EthRequest {:?}: {}", req, e))?;

        rx.await.map_err(|e| eyre::eyre!("ApiRequest receiver dropped: {}", e))
    }

    async fn call_rpc(&self, method: &str, params: Value) -> Result<Value> {
        // Use the in-memory RPC handler from the service
        let rpc = &self.service.rpc_handlers;

        let request = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        });

        let (response, _receiver) = rpc
            .rpc_query(&request.to_string())
            .await
            .wrap_err(format!("RPC call failed for method: {method}"))?;

        let response_value: Value =
            serde_json::from_str(&response).wrap_err("Failed to parse RPC response")?;

        if let Some(error) = response_value.get("error") {
            return Err(eyre::eyre!("RPC error: {}", error));
        }

        response_value
            .get("result")
            .cloned()
            .ok_or_else(|| eyre::eyre!("No result in RPC response"))
    }

    /// Call RPC method with no parameters
    async fn call_rpc_no_params(&self, method: &str) -> Result<Value> {
        self.call_rpc(method, json!([])).await
    }
}

impl TestNode {
    pub async fn system_chain(&self) -> Result<String> {
        let result = self.call_rpc_no_params("system_chain").await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    pub async fn get_best_block_number(&self) -> Result<u32> {
        let best_number = self.service.client.info().best_number;
        Ok(best_number)
    }

    pub async fn block_hash_by_number(&self, n: u32) -> eyre::Result<H256> {
        self.service
            .client
            .block_hash(n)
            .wrap_err("client.block_hash failed")?
            .ok_or_else(|| eyre::eyre!("no hash for block {}", n))
    }

    pub fn create_storage_key(pallet: &str, item: &str) -> StorageKey {
        let mut key = Vec::new();
        key.extend_from_slice(&twox_128(pallet.as_bytes()));
        key.extend_from_slice(&twox_128(item.as_bytes()));
        StorageKey(key)
    }

    pub async fn state_get_storage(
        &self,
        key: StorageKey,
        at: Option<H256>,
    ) -> Result<Option<String>> {
        let key_hex = format!("0x{}", hex::encode(&key.0));
        let result = match at {
            Some(hash) => self.call_rpc("state_getStorageAt", json!([key_hex, hash])).await?,
            None => self.call_rpc("state_getStorage", json!([key_hex])).await?,
        };

        Ok(result.as_str().map(|s| s.to_string()))
    }

    pub async fn get_decoded_timestamp(&self, at: Option<H256>) -> u64 {
        let storage_key = Self::create_storage_key("Timestamp", "Now");
        let encoded_value = self.state_get_storage(storage_key, at).await.unwrap().unwrap();
        let bytes =
            hex::decode(encoded_value.strip_prefix("0x").unwrap_or(&encoded_value)).unwrap();
        let mut input = &bytes[..];
        Decode::decode(&mut input).unwrap()
    }
}
