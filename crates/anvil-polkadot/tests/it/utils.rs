use alloy_eips::BlockId;
use alloy_primitives::{Address, B256, U256, hex};
use alloy_rpc_types::TransactionRequest;
use alloy_serde::WithOtherFields;
use anvil_core::eth::EthRequest;
use anvil_polkadot::{
    api_server::{
        self, ApiHandle, create_revive_rpc_client,
        revive_conversions::{AlloyU256, ReviveAddress},
    },
    config::{AnvilNodeConfig, SubstrateNodeConfig},
    init_tracing,
    logging::LoggingManager,
    opts::SubstrateCli,
    spawn,
    substrate_node::{in_mem_rpc::InMemoryRpcClient, service::Service},
};
use anvil_rpc::{error::RpcError, response::ResponseResult};
use eyre::{Result, WrapErr};
use futures::{StreamExt, channel::oneshot};
use pallet_revive_eth_rpc::{SubxtBlockInfoProvider, client::Client as EthRpcClient};
use parity_scale_codec::Decode;
use polkadot_sdk::{
    pallet_revive::evm::{self, Block, ReceiptInfo},
    polkadot_sdk_frame::traits::Header,
    sc_cli::CliConfiguration,
    sc_client_api::{BlockBackend, BlockchainEvents},
    sc_service::TaskManager,
    sp_core::{H256, storage::StorageKey, twox_128},
};
use serde_json::{Value, json};
use std::{fmt::Debug, time::Duration};
use subxt::{
    OnlineClient, PolkadotConfig, backend::rpc::RpcClient, ext::subxt_rpcs::LegacyRpcMethods,
    utils::H160,
};
use tempfile::TempDir;

const NATIVE_TO_ETH_RATIO: u128 = 1000000;
pub const EXISTENTIAL_DEPOSIT: u128 = substrate_runtime::currency::DOLLARS * NATIVE_TO_ETH_RATIO;

#[derive(Clone)]
pub struct BlockWaitTimeout {
    block_number: u32,
    timeout: Duration,
}

impl BlockWaitTimeout {
    pub fn new(block_number: u32, timeout: Duration) -> Self {
        Self { block_number, timeout }
    }
}

pub struct TestNode {
    pub service: Service,
    pub api: ApiHandle,
    pub eth_rpc_client: EthRpcClient,
    _temp_dir: Option<TempDir>,
    _task_manager: TaskManager,
}

impl TestNode {
    pub async fn new(
        anvil_config: AnvilNodeConfig,
        mut substrate_config: SubstrateNodeConfig,
    ) -> Result<Self> {
        let handle = tokio::runtime::Handle::current();

        let mut temp_dir = None;
        match substrate_config
            .base_path()
            .expect("We are in dev mode and failed to create a temp dir")
        {
            None => {
                let temp = tempfile::tempdir().expect("Failed to create temp dir");
                let db_path = temp.path().join("db");
                temp_dir = Some(temp);
                substrate_config.set_base_path(Some(db_path));
            }
            Some(_) if substrate_config.shared_params().is_dev() => {
                let temp = tempfile::tempdir().expect("Failed to create temp dir");
                let db_path = temp.path().join("db");
                temp_dir = Some(temp);
                substrate_config.set_base_path(Some(db_path));
            }
            Some(_) => {}
        }

        let substrate_client = SubstrateCli {};
        let config = substrate_config.create_configuration(&substrate_client, handle.clone())?;
        let logging_manager = if anvil_config.enable_tracing {
            init_tracing(anvil_config.silent)
        } else {
            LoggingManager::default()
        };

        // Initialized api can tap into the pallet revive's client via the network, but we should
        // also enable the access to the client programatically, to be able to call into
        // methods that are relevant to the overall testing but not exposed as anvil RPC
        // methods.
        let (service, task_manager, api) = spawn(anvil_config, config, logging_manager).await?;

        let rpc_client = RpcClient::new(InMemoryRpcClient(service.rpc_handlers.clone()));
        let subxt_api = OnlineClient::<PolkadotConfig>::from_rpc_client(rpc_client.clone()).await?;
        let rpc = LegacyRpcMethods::<PolkadotConfig>::new(rpc_client.clone());
        let block_provider = SubxtBlockInfoProvider::new(subxt_api.clone(), rpc.clone()).await?;

        let eth_rpc_client = create_revive_rpc_client(
            subxt_api,
            rpc_client,
            rpc,
            block_provider,
            service.spawn_handle.clone(),
        )
        .await?;

        Ok(Self { service, api, _temp_dir: temp_dir, _task_manager: task_manager, eth_rpc_client })
    }

    pub async fn eth_rpc(&mut self, req: EthRequest) -> Result<ResponseResult> {
        let (tx, rx) = oneshot::channel();
        self.api
            .try_send(api_server::ApiRequest { req: req.clone(), resp_sender: tx })
            .map_err(|e| eyre::eyre!("failed to send EthRequest {:?}: {}", req, e))?;

        rx.await.map_err(|e| eyre::eyre!("ApiRequest receiver dropped: {}", e))
    }

    pub async fn substrate_rpc(&self, method: &str, params: Value) -> Result<Value> {
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
}

impl TestNode {
    pub async fn block_hash_by_number(&self, n: u32) -> eyre::Result<H256> {
        self.service
            .client
            .block_hash(n)
            .wrap_err("client.block_hash failed")?
            .ok_or_else(|| eyre::eyre!("no hash for block {}", n))
    }

    pub async fn nonce(&self, n: u32, address: H160) -> eyre::Result<evm::U256> {
        let hash = self.block_hash_by_number(n).await?;
        let runtime_api = self.eth_rpc_client.runtime_api(hash);
        let nonce = runtime_api.nonce(address).await?;
        Ok(nonce)
    }

    pub fn create_storage_key(pallet: &str, item: &str) -> StorageKey {
        let mut key = Vec::new();
        key.extend_from_slice(&twox_128(pallet.as_bytes()));
        key.extend_from_slice(&twox_128(item.as_bytes()));
        StorageKey(key)
    }

    /// Execute an ethereum transaction.
    pub async fn send_transaction(
        &mut self,
        transaction: TransactionRequest,
        timeout: Option<BlockWaitTimeout>,
    ) -> Result<H256, RpcError> {
        let tx_hash = unwrap_response::<H256>(
            self.eth_rpc(EthRequest::EthSendTransaction(Box::new(WithOtherFields::new(
                transaction,
            ))))
            .await
            .unwrap(),
        )?;

        if let Some(BlockWaitTimeout { block_number, timeout }) = timeout {
            self.wait_for_block_with_timeout(block_number, timeout).await.unwrap();
        }
        Ok(tx_hash)
    }

    pub async fn state_get_storage(
        &self,
        key: StorageKey,
        at: Option<H256>,
    ) -> Result<Option<String>> {
        let key_hex = format!("0x{}", hex::encode(&key.0));
        let result = match at {
            Some(hash) => self.substrate_rpc("state_getStorageAt", json!([key_hex, hash])).await?,
            None => self.substrate_rpc("state_getStorage", json!([key_hex])).await?,
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

    async fn wait_for_block_with_number(&self, n: u32) {
        let mut import_stream = self.service.client.import_notification_stream();

        while let Some(notification) = import_stream.next().await {
            let block_number = *notification.header.number();
            if block_number >= n {
                break;
            }
        }
    }

    pub async fn best_block_number(&self) -> u32 {
        let num = self
            .substrate_rpc("chain_getHeader", json!([]))
            .await
            .unwrap()
            .get("number")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_owned();
        u32::from_str_radix(num.trim_start_matches("0x"), 16).unwrap()
    }

    pub async fn wait_for_block_with_timeout(
        &self,
        n: u32,
        timeout: std::time::Duration,
    ) -> eyre::Result<()> {
        tokio::time::timeout(timeout, self.wait_for_block_with_number(n))
            .await
            .map_err(|e| e.into())
    }
}

impl TestNode {
    pub async fn get_balance(&mut self, address: H160, block: Option<BlockId>) -> U256 {
        unwrap_response::<U256>(
            self.eth_rpc(EthRequest::EthGetBalance(
                Address::from(ReviveAddress::new(address)),
                block,
            ))
            .await
            .unwrap(),
        )
        .unwrap()
    }

    pub async fn get_transaction_receipt(&mut self, tx_hash: H256) -> ReceiptInfo {
        unwrap_response::<Option<ReceiptInfo>>(
            self.eth_rpc(EthRequest::EthGetTransactionReceipt(B256::from(
                tx_hash.to_fixed_bytes(),
            )))
            .await
            .unwrap(),
        )
        .unwrap()
        .unwrap()
    }

    pub async fn get_block_by_hash(&mut self, hash: H256) -> Block {
        unwrap_response::<Block>(
            self.eth_rpc(EthRequest::EthGetBlockByHash(hash.as_fixed_bytes().into(), false))
                .await
                .unwrap(),
        )
        .unwrap()
    }

    // Initialize with some balance a random account and return its address.
    //
    // Returns the initialized random account address and transaction hash.
    // When a block wait time is provided, it is assumed that automine was
    // previously enabled on the node.
    pub async fn eth_transfer_to_unitialized_random_account(
        &mut self,
        from: Address,
        transfer_amount: U256,
        block_wait_timeout: Option<BlockWaitTimeout>,
    ) -> (Address, H256) {
        let dest_addr = Address::random();
        let dest_h160 = H160::from_slice(dest_addr.as_slice());
        let from_h160 = H160::from_slice(from.as_slice());

        // Create a random account with some balance.
        let from_initial_balance = self.get_balance(from_h160, None).await;
        let dest_initial_balance = self.get_balance(dest_h160, None).await;
        assert_eq!(dest_initial_balance, U256::ZERO);

        let transaction =
            TransactionRequest::default().value(transfer_amount).from(from).to(dest_addr);
        let is_automine = block_wait_timeout.is_some();
        let tx_hash = self.send_transaction(transaction, block_wait_timeout).await.unwrap();

        if is_automine {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let receipt_info = self.get_transaction_receipt(tx_hash).await;

            // Assert on balances after first transfer.
            let from_balance = self.get_balance(from_h160, None).await;
            let dest_balance = self.get_balance(dest_h160, None).await;
            assert_eq!(
                from_balance,
                from_initial_balance
                    - AlloyU256::from(receipt_info.effective_gas_price * receipt_info.gas_used)
                        .inner()
                    - transfer_amount
                    - U256::from(EXISTENTIAL_DEPOSIT),
                "signer's balance should have changed"
            );
            assert_eq!(
                dest_balance,
                dest_initial_balance + transfer_amount,
                "dest's balance should have changed"
            );
        }

        (dest_addr, tx_hash)
    }
}

pub fn assert_with_tolerance<T>(actual: T, expected: T, tolerance: T, message: &str)
where
    T: PartialOrd + std::ops::Sub<Output = T> + Debug + Copy,
{
    let diff = if actual > expected { actual - expected } else { expected - actual };

    if diff > tolerance {
        panic!(
            "{message}\nExpected: {expected:?} ± {tolerance:?}\nActual: {actual:?}\nDifference: {diff:?}",
        );
    }
}

pub fn unwrap_response<T>(response: ResponseResult) -> Result<T, RpcError>
where
    T: serde::de::DeserializeOwned,
{
    match response {
        ResponseResult::Success(value) => Ok(serde_json::from_value(value).unwrap()),
        ResponseResult::Error(err) => Err(err),
    }
}
