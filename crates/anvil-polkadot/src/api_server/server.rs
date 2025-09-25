use crate::{
    api_server::{
        convert::{
            from_address_to_h160, from_alloy_u256_to_sp_u256, from_sp_u256_to_alloy_u256,
            to_block_number_or_tag_or_hash, try_convert_transaction_request,
        },
        error::{Error, Result, ToRpcResponseResult},
        ApiRequest,
    },
    logging::LoggingManager,
    macros::node_info,
    substrate_node::{
        mining_engine::MiningEngine,
        service::{
            storage::{AccountType, ByteCodeType, CodeInfo, ContractInfo, ReviveAccountInfo},
            BackendWithOverlay, Client, Service,
        },
    },
};
use alloy_primitives::{Address, Bytes, B256, U256, U64};
use alloy_rpc_types::{anvil::MineOptions, request::TransactionRequest};
use alloy_serde::WithOtherFields;
use anvil_core::eth::EthRequest;
use anvil_rpc::response::ResponseResult;
use codec::Encode;
use futures::{channel::mpsc, StreamExt};
use polkadot_sdk::{
    pallet_revive::{
        evm::{Account, Block, BlockNumberOrTagOrHash, BlockTag, ReceiptInfo},
        ReviveApi,
    },
    pallet_revive_eth_rpc::{
        client::{Client as EthRpcClient, SubscriptionType},
        subxt_client::{self, SrcChainConfig},
        EthRpcError, ReceiptExtractor, ReceiptProvider, SubxtBlockInfoProvider,
    },
    parachains_common::{AccountId, Hash},
    sc_client_api::HeaderBackend,
    sc_service::RpcHandlers,
    sp_api::ProvideRuntimeApi,
    sp_core::{self, keccak_256, Hasher, H160, H256, U256 as SU256},
    sp_runtime::traits::BlakeTwo256,
};
use sqlx::sqlite::SqlitePoolOptions;
use std::{sync::Arc, time::Duration};
use substrate_runtime::Balance;

use subxt::{
    backend::rpc::{RawRpcFuture, RawRpcSubscription, RawValue, RpcClient, RpcClientT},
    ext::{
        jsonrpsee::core::traits::ToRpcParams,
        subxt_rpcs::{Error as SubxtRpcError, LegacyRpcMethods},
    },
    OnlineClient,
};

struct InMemoryRpcClient(RpcHandlers);

struct Params(Option<Box<RawValue>>);

impl ToRpcParams for Params {
    fn to_rpc_params(self) -> std::result::Result<Option<Box<RawValue>>, serde_json::Error> {
        Ok(self.0)
    }
}

impl RpcClientT for InMemoryRpcClient {
    fn request_raw<'a>(
        &'a self,
        method: &'a str,
        params: Option<Box<RawValue>>,
    ) -> RawRpcFuture<'a, Box<RawValue>> {
        Box::pin(async move {
            self.0
                .handle()
                .call(method, Params(params))
                .await
                .map_err(|err| SubxtRpcError::Client(Box::new(err)))
        })
    }

    fn subscribe_raw<'a>(
        &'a self,
        sub: &'a str,
        params: Option<Box<RawValue>>,
        _unsub: &'a str,
    ) -> RawRpcFuture<'a, RawRpcSubscription> {
        Box::pin(async move {
            let subscription = self
                .0
                .handle()
                .subscribe_unbounded(sub, Params(params))
                .await
                .map_err(|err| SubxtRpcError::Client(Box::new(err)))?;
            let id = serde_json::Value::from(subscription.subscription_id().to_owned())
                .as_str()
                .map(|s| s.to_string());
            let raw_stream = futures::stream::unfold(subscription, |mut sub| async move {
                match sub.next::<Box<RawValue>>().await {
                    Some(Ok((notification, _sub_id))) => Some((Ok(notification), sub)),
                    Some(Err(e)) => Some((Err(SubxtRpcError::Client(Box::new(e))), sub)),
                    None => None, // Subscription ended, Do something here? :-??
                }
            });
            Ok(RawRpcSubscription { stream: Box::pin(raw_stream), id })
        })
    }
}

pub struct Wallet {
    accounts: Vec<Account>,
}

pub struct ApiServer {
    req_receiver: mpsc::Receiver<ApiRequest>,
    backend: BackendWithOverlay,
    logging_manager: LoggingManager,
    client: Arc<Client>,
    mining_engine: Arc<MiningEngine>,
    eth_rpc_client: EthRpcClient,
    wallet: Wallet,
}

impl ApiServer {
    pub async fn new(
        substrate_service: Service,
        req_receiver: mpsc::Receiver<ApiRequest>,
        logging_manager: LoggingManager,
    ) -> Self {
        let rpc_client = RpcClient::new(InMemoryRpcClient(substrate_service.rpc_handlers.clone()));
        let api =
            OnlineClient::<SrcChainConfig>::from_rpc_client(rpc_client.clone()).await.unwrap();
        let rpc = LegacyRpcMethods::<SrcChainConfig>::new(rpc_client.clone());

        let block_provider = SubxtBlockInfoProvider::new(api.clone(), rpc.clone()).await.unwrap();

        let (pool, keep_latest_n_blocks) = {
            // see sqlite in-memory issue: https://github.com/launchbadge/sqlx/issues/2510
            let pool = SqlitePoolOptions::new()
                .max_connections(1)
                .idle_timeout(None)
                .max_lifetime(None)
                .connect("sqlite::memory:")
                .await
                .unwrap();

            (pool, Some(100))
        };

        let receipt_extractor = ReceiptExtractor::new(api.clone(), None).await.unwrap();

        let receipt_provider = ReceiptProvider::new(
            pool,
            block_provider.clone(),
            receipt_extractor.clone(),
            keep_latest_n_blocks,
        )
        .await
        .unwrap();

        let eth_rpc_client =
            EthRpcClient::new(api, rpc_client, rpc, block_provider, receipt_provider)
                .await
                .unwrap();

        let eth_rpc_client_clone = eth_rpc_client.clone();
        substrate_service.spawn_handle.spawn("block-subscription", "None", async move {
            let eth_rpc_client = eth_rpc_client_clone;
            let fut1 = eth_rpc_client.subscribe_and_cache_new_blocks(SubscriptionType::BestBlocks);
            let fut2 =
                eth_rpc_client.subscribe_and_cache_new_blocks(SubscriptionType::FinalizedBlocks);

            let res = tokio::try_join!(fut1, fut2).map(|_| ());

            if let Err(err) = res {
                panic!("Block subscription task failed: {err:?}",)
            }
        });

        Self {
            req_receiver,
            logging_manager,
            eth_rpc_client,
            backend: BackendWithOverlay::new(
                substrate_service.backend.clone(),
                substrate_service.storage_overrides.clone(),
            ),
            client: substrate_service.client.clone(),
            mining_engine: substrate_service.mining_engine.clone(),
            wallet: Wallet {
                accounts: vec![
                    Account::from(subxt_signer::eth::dev::baltathar()),
                    Account::from(subxt_signer::eth::dev::alith()),
                ],
            },
        }
    }

    pub async fn run(mut self) {
        while let Some(msg) = self.req_receiver.next().await {
            let resp = self.execute(msg.req).await;

            msg.resp_sender.send(resp).expect("Dropped receiver");
        }
    }

    pub async fn execute(&mut self, req: EthRequest) -> ResponseResult {
        let res = match req.clone() {
            EthRequest::Mine(blocks, interval) => self.mine(blocks, interval).await.to_rpc_result(),
            EthRequest::SetIntervalMining(interval) => {
                self.set_interval_mining(interval).to_rpc_result()
            }
            EthRequest::GetIntervalMining(()) => self.get_interval_mining().to_rpc_result(),
            EthRequest::GetAutoMine(()) => self.get_auto_mine().to_rpc_result(),
            EthRequest::SetAutomine(enabled) => self.set_auto_mine(enabled).to_rpc_result(),
            EthRequest::EvmMine(mine) => self.evm_mine(mine).await.to_rpc_result(),
            EthRequest::EvmSetBlockTimeStampInterval(time) => {
                self.set_block_timestamp_interval(time).to_rpc_result()
            }
            EthRequest::EvmRemoveBlockTimeStampInterval(()) => {
                self.remove_block_timestamp_interval().to_rpc_result()
            }
            EthRequest::EvmSetNextBlockTimeStamp(time) => {
                self.set_next_block_timestamp(time).to_rpc_result()
            }
            EthRequest::EvmIncreaseTime(time) => self.increase_time(time).to_rpc_result(),
            EthRequest::EvmSetTime(timestamp) => self.set_time(timestamp).to_rpc_result(),
            EthRequest::SetLogging(enabled) => self.set_logging(enabled).to_rpc_result(),
            EthRequest::SetBalance(address, value) => {
                self.set_balance(address, value).to_rpc_result()
            }
            EthRequest::SetNonce(address, value) => self.set_nonce(address, value).to_rpc_result(),
            EthRequest::SetCode(address, bytes) => self.set_code(address, bytes).to_rpc_result(),
            EthRequest::SetStorageAt(address, key, value) => {
                self.set_storage_at(address, key, value).to_rpc_result()
            }
            EthRequest::SetChainId(chain_id) => self.set_chain_id(chain_id).to_rpc_result(),
            EthRequest::EthChainId(()) => self.eth_chain_id().to_rpc_result(),
            EthRequest::EthNetworkId(()) => self.network_id().to_rpc_result(),
            EthRequest::NetListening(()) => self.net_listening().to_rpc_result(),
            EthRequest::EthSyncing(()) => self.syncing().to_rpc_result(),
            EthRequest::EthGetTransactionReceipt(tx_hash) => {
                self.transaction_receipt(tx_hash).await.to_rpc_result()
            }
            EthRequest::EthEstimateGas(call, block, _state_overrides, _block_overrides) => {
                self.estimate_gas(call, block).await.to_rpc_result()
            }
            EthRequest::EthGetBalance(addr, block) => self
                .get_balance(from_address_to_h160(addr), to_block_number_or_tag_or_hash(block))
                .await
                .to_rpc_result(),
            EthRequest::EthGetStorageAt(addr, slot, block) => self
                .get_storage_at(
                    from_address_to_h160(addr),
                    from_alloy_u256_to_sp_u256(slot),
                    to_block_number_or_tag_or_hash(block),
                )
                .await
                .to_rpc_result(),
            EthRequest::EthGetCodeAt(addr, block) => self
                .get_code(from_address_to_h160(addr), to_block_number_or_tag_or_hash(block))
                .await
                .to_rpc_result(),
            EthRequest::EthGetBlockByHash(hash, full) => self
                .get_block_by_hash(H256::from_slice(hash.as_slice()), full)
                .await
                .to_rpc_result(),
            EthRequest::EthSendTransaction(request) => {
                self.send_transaction(*request).await.to_rpc_result()
            }
            EthRequest::EthGetTransactionCount(addr, block) => self
                .get_transaction_count(
                    from_address_to_h160(addr),
                    to_block_number_or_tag_or_hash(block),
                )
                .await
                .to_rpc_result(),
            _ => Err::<(), _>(Error::RpcUnimplemented).to_rpc_result(),
        };

        if let ResponseResult::Error(err) = &res {
            node_info!("\nRPC request failed:");
            node_info!("    Request: {:?}", req);
            node_info!("    Error: {}\n", err);
        }

        res
    }

    async fn mine(&self, blocks: Option<U256>, interval: Option<U256>) -> Result<()> {
        node_info!("anvil_mine");

        if blocks.is_some_and(|b| u64::try_from(b).is_err()) {
            return Err(Error::InvalidParams("The number of blocks is too large".to_string()));
        }
        if interval.is_some_and(|i| u64::try_from(i).is_err()) {
            return Err(Error::InvalidParams(
                "The interval between blocks is too large".to_string(),
            ));
        }
        self.mining_engine
            .mine(blocks.map(|b| b.to()), interval.map(|i| Duration::from_secs(i.to())))
            .await
            .map_err(Error::Mining)
    }

    fn set_interval_mining(&self, interval: u64) -> Result<()> {
        node_info!("evm_setIntervalMining");

        self.mining_engine.set_interval_mining(Duration::from_secs(interval));
        Ok(())
    }

    fn get_interval_mining(&self) -> Result<Option<u64>> {
        node_info!("anvil_getIntervalMining");

        Ok(self.mining_engine.get_interval_mining())
    }

    fn get_auto_mine(&self) -> Result<bool> {
        node_info!("anvil_getAutomine");

        Ok(self.mining_engine.is_automine())
    }

    fn set_auto_mine(&self, enabled: bool) -> Result<()> {
        node_info!("evm_setAutomine");

        self.mining_engine.set_auto_mine(enabled);
        Ok(())
    }

    async fn evm_mine(
        &self,
        mine: Option<anvil_core::eth::Params<Option<MineOptions>>>,
    ) -> Result<String> {
        node_info!("evm_mine");

        self.mining_engine.evm_mine(mine.and_then(|p| p.params)).await?;
        Ok("0x0".to_string())
    }

    fn set_block_timestamp_interval(&self, time: u64) -> Result<()> {
        node_info!("anvil_setBlockTimestampInterval");

        self.mining_engine.set_block_timestamp_interval(Duration::from_secs(time));
        Ok(())
    }

    fn remove_block_timestamp_interval(&self) -> Result<bool> {
        node_info!("anvil_removeBlockTimestampInterval");

        Ok(self.mining_engine.remove_block_timestamp_interval())
    }

    fn set_next_block_timestamp(&self, time: U256) -> Result<()> {
        node_info!("anvil_setBlockTimestampInterval");

        if time >= U256::from(u64::MAX) {
            return Err(Error::InvalidParams("The timestamp is too big".to_string()));
        }
        let time = time.to::<u64>();
        self.mining_engine
            .set_next_block_timestamp(Duration::from_secs(time))
            .map_err(Error::Mining)
    }

    fn increase_time(&self, time: U256) -> Result<i64> {
        node_info!("evm_increaseTime");

        Ok(self.mining_engine.increase_time(Duration::from_secs(time.try_into().unwrap_or(0))))
    }

    fn set_time(&self, timestamp: U256) -> Result<u64> {
        node_info!("evm_setTime");

        if timestamp >= U256::from(u64::MAX) {
            return Err(Error::InvalidParams("The timestamp is too big".to_string()));
        }
        let time = timestamp.to::<u64>();
        Ok(self.mining_engine.set_time(Duration::from_secs(time)))
    }

    fn set_logging(&self, enabled: bool) -> Result<()> {
        node_info!("anvil_setLoggingEnabled");

        self.logging_manager.set_enabled(enabled);
        Ok(())
    }

    fn set_balance(&self, address: Address, value: U256) -> Result<()> {
        node_info!("anvil_setBalance");

        let latest_block = self.backend.blockchain().info().best_hash;

        let account_id = self.get_account_id(latest_block, address);
        let mut balance_data =
            self.backend.read_balance(latest_block, account_id.clone())?.unwrap_or_default();
        let total_issuance = self.backend.read_total_issuance(latest_block)?;
        let (new_balance, dust) = self.construct_balance_with_dust(latest_block, value);

        let diff = new_balance as i128 - (balance_data.total() as i128);

        if diff < 0 {
            let diff = diff.abs() as Balance;

            balance_data.free = balance_data.free.saturating_sub(diff);
            self.backend.inject_balance(latest_block, account_id, balance_data);
            self.backend.inject_total_issuance(latest_block, total_issuance.saturating_sub(diff));
        } else if diff > 0 {
            let diff = diff.abs() as Balance;

            balance_data.free = balance_data.free.saturating_add(diff);
            self.backend.inject_balance(latest_block, account_id, balance_data);
            self.backend.inject_total_issuance(latest_block, total_issuance.saturating_add(diff));
        }

        let mut account_info = self
            .backend
            .read_revive_account_info(latest_block, address)?
            .unwrap_or_else(|| ReviveAccountInfo { account_type: AccountType::EOA, dust: 0 });

        if account_info.dust != dust {
            account_info.dust = dust;

            self.backend.inject_revive_account_info(latest_block, address, account_info);
        }

        Ok(())
    }

    fn set_nonce(&self, address: Address, value: U256) -> Result<()> {
        node_info!("anvil_setNonce");

        let latest_block = self.backend.blockchain().info().best_hash;

        let account_id = self.get_account_id(latest_block, address);

        let mut account_info = self
            .backend
            .read_system_account_info(latest_block, account_id.clone())
            .unwrap()
            .unwrap_or_default();

        account_info.nonce = value.try_into().map_err(|_| Error::NonceOverflow)?;

        self.backend.inject_system_account_info(latest_block, account_id, account_info);

        Ok(())
    }

    fn set_storage_at(&self, address: Address, key: U256, value: B256) -> Result<()> {
        let latest_block = self.backend.blockchain().info().best_hash;

        let Some(ReviveAccountInfo { account_type: AccountType::Contract(contract_info), .. }) =
            self.backend.read_revive_account_info(latest_block, address)?
        else {
            return Ok(());
        };

        self.backend.inject_child_storage(
            latest_block,
            contract_info.trie_id,
            key.to_be_bytes_vec(),
            value.to_vec(),
        );

        Ok(())
    }

    fn set_code(&self, address: Address, bytes: Bytes) -> Result<()> {
        node_info!("anvil_setCode");

        let latest_block = self.backend.blockchain().info().best_hash;

        let account_id = self.get_account_id(latest_block, address);

        let code_hash = H256(keccak_256(&bytes));

        let account_info = match self.backend.read_revive_account_info(latest_block, address)? {
            None => {
                let contract_info = new_contract_info(&address, code_hash);

                ReviveAccountInfo { account_type: AccountType::Contract(contract_info), dust: 0 }
            }
            Some(ReviveAccountInfo {
                account_type: AccountType::Contract(mut contract_info),
                dust,
            }) => {
                if contract_info.code_hash != code_hash {
                    // Remove the pristine code and code info for the old hash.
                    self.backend.inject_pristine_code(latest_block, contract_info.code_hash, None);
                    self.backend.inject_code_info(latest_block, contract_info.code_hash, None);
                }

                contract_info.code_hash = code_hash;

                ReviveAccountInfo { account_type: AccountType::Contract(contract_info), dust }
            }
            Some(ReviveAccountInfo { account_type: AccountType::EOA, dust }) => {
                let contract_info = new_contract_info(&address, code_hash);

                ReviveAccountInfo { account_type: AccountType::Contract(contract_info), dust }
            }
        };

        self.backend.inject_revive_account_info(latest_block, address, account_info);

        let code_info = CodeInfo {
            owner: <[u8; 32]>::from(account_id).into(),
            deposit: 0,
            refcount: 0,
            code_len: bytes.len() as u32,
            behaviour_version: 0,
            code_type: ByteCodeType::Evm,
        };

        self.backend.inject_pristine_code(latest_block, code_hash, Some(bytes));
        self.backend.inject_code_info(latest_block, code_hash, Some(code_info));

        Ok(())
    }

    fn set_chain_id(&self, chain_id: u64) -> Result<()> {
        node_info!("anvil_setChainId");

        let latest_block = self.backend.blockchain().info().best_hash;
        self.backend.inject_chain_id(latest_block, chain_id);

        Ok(())
    }

    fn eth_chain_id(&self) -> Result<U64> {
        node_info!("eth_chainId");
        let latest_block = self.backend.blockchain().info().best_hash;

        Ok(U256::from(self.chain_id(latest_block)).to::<U64>())
    }

    fn network_id(&self) -> Result<u64> {
        node_info!("eth_networkId");
        let latest_block = self.backend.blockchain().info().best_hash;

        Ok(self.chain_id(latest_block))
    }

    fn net_listening(&self) -> Result<bool> {
        node_info!("net_listening");
        Ok(true)
    }

    fn syncing(&self) -> Result<bool> {
        node_info!("eth_syncing");
        Ok(false)
    }

    async fn transaction_receipt(&self, tx_hash: B256) -> Result<Option<ReceiptInfo>> {
        node_info!("eth_getTransactionReceipt");
        // TODO: do we really need to return Ok(None) if the transaction is still in the pool?
        Ok(self.eth_rpc_client.receipt(&(tx_hash.0.into())).await)
    }

    pub(crate) async fn get_balance(
        &self,
        address: H160,
        block: BlockNumberOrTagOrHash,
    ) -> Result<U256> {
        node_info!("eth_getBalance");
        let hash =
            self.eth_rpc_client.block_hash_for_tag(block).await.map_err(Error::ClientError)?;
        let runtime_api = self.eth_rpc_client.runtime_api(hash);
        let balance = runtime_api.balance(address).await.map_err(Error::ClientError)?;
        Ok(from_sp_u256_to_alloy_u256(balance))
    }

    pub(crate) async fn get_storage_at(
        &self,
        address: H160,
        storage_slot: sp_core::U256,
        block: BlockNumberOrTagOrHash,
    ) -> Result<Bytes> {
        let hash =
            self.eth_rpc_client.block_hash_for_tag(block).await.map_err(Error::ClientError)?;
        let runtime_api = self.eth_rpc_client.runtime_api(hash);
        let bytes = runtime_api
            .get_storage(address, storage_slot.to_big_endian())
            .await
            .map_err(Error::ClientError)?;
        Ok(bytes.unwrap_or_default().into())
    }

    pub(crate) async fn get_code(
        &self,
        address: H160,
        block: BlockNumberOrTagOrHash,
    ) -> Result<Bytes> {
        let hash =
            self.eth_rpc_client.block_hash_for_tag(block).await.map_err(Error::ClientError)?;
        let code = self
            .eth_rpc_client
            .runtime_api(hash)
            .code(address)
            .await
            .map_err(Error::ClientError)?;
        Ok(code.into())
    }

    pub(crate) async fn get_block_by_hash(
        &self,
        block_hash: H256,
        hydrated_transactions: bool,
    ) -> Result<Option<Block>> {
        let Some(block) =
            self.eth_rpc_client.block_by_hash(&block_hash).await.map_err(Error::ClientError)?
        else {
            return Ok(None);
        };
        let block = self.eth_rpc_client.evm_block(block, hydrated_transactions).await;
        Ok(Some(block))
    }

    pub(crate) async fn send_raw_transaction(&self, transaction: Vec<u8>) -> Result<H256> {
        let hash = H256(keccak_256(&transaction));
        let call = subxt_client::tx().revive().eth_transact(transaction);
        self.eth_rpc_client
            .submit(call)
            .await
            .map_err(|err| {
                node_info!("submit call failed: {err:?}");
                err
            })
            .map_err(Error::ClientError)?;

        node_info!("send_raw_transaction hash: {hash:?}");
        Ok(hash)
    }

    pub(crate) async fn send_transaction(
        &self,
        transaction_req: WithOtherFields<TransactionRequest>,
    ) -> Result<H256> {
        let latest_block = self.backend.blockchain().info().best_hash;

        let mut transaction = try_convert_transaction_request(transaction_req.clone().into_inner());
        node_info!("{transaction:#?}");

        let Some(from) = transaction.from else {
            node_info!("Transaction must have a sender");
            return Err(Error::EthRpcError(EthRpcError::InvalidTransaction));
        };

        let account = self
            .wallet
            .accounts
            .iter()
            .find(|account| account.address() == from)
            .ok_or(Error::EthRpcError(EthRpcError::AccountNotFound(from)))?;

        if transaction.gas.is_none() {
            transaction.gas = Some(self.estimate_gas(transaction_req.clone(), None).await?);
        }

        if transaction.gas_price.is_none() {
            transaction.gas_price = Some(self.gas_price().await?);
        }

        if transaction.nonce.is_none() {
            transaction.nonce =
                Some(self.get_transaction_count(from, BlockTag::Latest.into()).await?);
        }

        if transaction.chain_id.is_none() {
            transaction.chain_id =
                Some(from_alloy_u256_to_sp_u256(U256::from(self.chain_id(latest_block))));
        }

        let tx = transaction
            .try_into_unsigned()
            .map_err(|_| Error::EthRpcError(EthRpcError::InvalidTransaction))?;
        let payload = account.sign_transaction(tx).signed_payload();
        self.send_raw_transaction(payload).await
    }

    pub(crate) async fn estimate_gas(
        &self,
        request: WithOtherFields<TransactionRequest>,
        block: Option<alloy_rpc_types::BlockId>,
    ) -> Result<SU256> {
        node_info!("eth_estimateGas");

        let hash = self
            .eth_rpc_client
            .block_hash_for_tag(to_block_number_or_tag_or_hash(block))
            .await
            .map_err(Error::ClientError)?;
        let runtime_api = self.eth_rpc_client.runtime_api(hash);
        let dry_run = runtime_api
            .dry_run(try_convert_transaction_request(request.into_inner()))
            .await
            .map_err(Error::ClientError)?;
        Ok(dry_run.eth_gas)
    }

    async fn gas_price(&self) -> Result<SU256> {
        let hash = self
            .eth_rpc_client
            .block_hash_for_tag(BlockTag::Latest.into())
            .await
            .map_err(Error::ClientError)?;
        let runtime_api = self.eth_rpc_client.runtime_api(hash);
        Ok(runtime_api.gas_price().await.map_err(Error::ClientError)?)
    }

    async fn get_transaction_count(
        &self,
        address: H160,
        block: BlockNumberOrTagOrHash,
    ) -> Result<SU256> {
        let hash =
            self.eth_rpc_client.block_hash_for_tag(block).await.map_err(Error::ClientError)?;
        let runtime_api = self.eth_rpc_client.runtime_api(hash);
        let nonce = runtime_api.nonce(address).await.map_err(Error::ClientError)?;
        Ok(nonce)
    }

    // ----- Helpers

    fn chain_id(&self, at: Hash) -> u64 {
        self.backend.read_chain_id(at).unwrap_or(420_420_420)
    }

    fn get_account_id(&self, block: Hash, address: Address) -> AccountId {
        self.client.runtime_api().account_id(block, from_address_to_h160(address)).unwrap()
    }

    fn construct_balance_with_dust(&self, block: Hash, value: U256) -> (Balance, u32) {
        self.client
            .runtime_api()
            .new_balance_with_dust(block, from_alloy_u256_to_sp_u256(value))
            .unwrap()
            .unwrap()
    }
}

fn new_contract_info(address: &Address, code_hash: H256) -> ContractInfo {
    let address = H160::from_slice(address.as_slice());

    let trie_id = {
        let buf = ("bcontract_trie_v1", address, 0).using_encoded(BlakeTwo256::hash);
        buf.as_ref().to_vec()
    };

    ContractInfo {
        trie_id,
        code_hash,
        storage_bytes: 0,
        storage_items: 0,
        storage_byte_deposit: 0,
        storage_item_deposit: 0,
        storage_base_deposit: 0,
        immutable_data_len: 0,
    }
}
