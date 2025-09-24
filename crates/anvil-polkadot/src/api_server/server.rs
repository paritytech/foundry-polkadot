use crate::{
    api_server::{
        error::{Error, Result, ToRpcResponseResult},
        revive_conversions::{
            convert_to_generic_transaction, AlloyU256, ReviveAddress, ReviveBlockId,
        },
        ApiRequest,
    },
    logging::LoggingManager,
    macros::node_info,
    substrate_node::{
        mining_engine::MiningEngine,
        service::TransactionPoolHandle,
    },
};
use alloy_eips::BlockId;
use alloy_network::AnyRpcTransaction;
use alloy_primitives::{Address, B256, U256, U64};
use alloy_rpc_types::{anvil::MineOptions, TransactionRequest, txpool::{TxpoolContent, TxpoolInspect, TxpoolStatus}};
use alloy_serde::WithOtherFields;
use anvil_core::eth::{EthRequest, Params as MineParams};
use anvil_rpc::response::ResponseResult;
use futures::{channel::mpsc, stream, StreamExt};
use indexmap::IndexMap;
use parity_scale_codec::{DecodeLimit, Encode};
use polkadot_sdk::{
    frame_support::MAX_EXTRINSIC_DEPTH,
    pallet_revive::{self, evm::{Account, Block, BlockNumberOrTagOrHash, BlockTag, Bytes, ReceiptInfo, TransactionSigned}},
    pallet_revive_eth_rpc::{
        client::Client as EthRpcClient,
        subxt_client::{self, SrcChainConfig},
        EthRpcError, ReceiptExtractor, ReceiptProvider, SubxtBlockInfoProvider,
    },
    sc_service::{InPoolTransaction, RpcHandlers},
    sc_transaction_pool_api::TransactionPool,
    sp_core::{self, keccak_256, H256},
    sp_runtime,
};
use serde_json::Value;
use sqlx::sqlite::SqlitePoolOptions;
use std::{sync::Arc, time::Duration};
use substrate_runtime::{RuntimeCall, UncheckedExtrinsic};
use subxt::{
    backend::rpc::{RawRpcFuture, RawRpcSubscription, RawValue, RpcClient, RpcClientT},
    ext::{
        jsonrpsee::core::traits::ToRpcParams,
        subxt_rpcs::{Error as SubxtRpcError, LegacyRpcMethods},
    },
    utils::H160,
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
            let id = Value::from(subscription.subscription_id().to_owned())
                .as_str()
                .map(|s| s.to_string());
            let raw_stream = stream::unfold(subscription, |mut sub| async move {
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
    logging_manager: LoggingManager,
    mining_engine: Arc<MiningEngine>,
    eth_rpc_client: EthRpcClient,
    wallet: Wallet,
    tx_pool: Arc<TransactionPoolHandle>,
}

impl ApiServer {
    pub async fn new(
        mining_engine: Arc<MiningEngine>,
        rpc_handlers: RpcHandlers,
        req_receiver: mpsc::Receiver<ApiRequest>,
        logging_manager: LoggingManager,
        tx_pool: Arc<TransactionPoolHandle>,
    ) -> Self {
        let rpc_client = RpcClient::new(InMemoryRpcClient(rpc_handlers));
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

        Self {
            req_receiver,
            logging_manager,
            mining_engine,
            eth_rpc_client,
            wallet: Wallet {
                accounts: vec![
                    Account::from(subxt_signer::eth::dev::baltathar()),
                    Account::from(subxt_signer::eth::dev::alith()),
                ],
            },
            tx_pool,
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
            EthRequest::SetLogging(enabled) => self.set_logging(enabled).to_rpc_result(),
            //------- Mining---------
            EthRequest::Mine(blocks, interval) => self.mine(blocks, interval).await.to_rpc_result(),
            EthRequest::SetIntervalMining(interval) => {
                self.set_interval_mining(interval).to_rpc_result()
            }
            EthRequest::GetIntervalMining(()) => self.get_interval_mining().to_rpc_result(),
            EthRequest::GetAutoMine(()) => self.get_auto_mine().to_rpc_result(),
            EthRequest::SetAutomine(enabled) => self.set_auto_mine(enabled).to_rpc_result(),
            EthRequest::EvmMine(mine) => self.evm_mine(mine).await.to_rpc_result(),
            //------- TimeMachine---------
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
            //------- Revive---------
            EthRequest::EthChainId(()) => self.eth_chain_id().to_rpc_result(),
            EthRequest::EthNetworkId(()) => self.network_id().to_rpc_result(),
            EthRequest::NetListening(()) => self.net_listening().to_rpc_result(),
            EthRequest::EthSyncing(()) => self.syncing().to_rpc_result(),
            EthRequest::EthGetTransactionReceipt(tx_hash) => {
                self.transaction_receipt(tx_hash).await.to_rpc_result()
            }
            EthRequest::EthGetBalance(addr, block) => {
                self.get_balance(addr, block).await.to_rpc_result()
            }
            EthRequest::EthGetStorageAt(addr, slot, block) => {
                self.get_storage_at(addr, slot, block).await.to_rpc_result()
            }
            EthRequest::EthGetCodeAt(addr, block) => {
                self.get_code(addr, block).await.to_rpc_result()
            }
            EthRequest::EthGetBlockByHash(hash, full) => {
                self.get_block_by_hash(hash, full).await.to_rpc_result()
            }
            EthRequest::EthEstimateGas(call, block, _overrides) => {
                self.estimate_gas(call, block).await.to_rpc_result()
            }
            EthRequest::EthSendTransaction(request) => {
                self.send_transaction(*request.clone()).await.to_rpc_result()
            }
            // Txpool methods
            EthRequest::TxPoolStatus(_) => self.txpool_status().await,
            EthRequest::TxPoolInspect(_) => self.txpool_inspect().await,
            EthRequest::TxPoolContent(_) => self.txpool_content().await,
            // Anvil drop transaction methods
            EthRequest::DropTransaction(tx_hash) => self.anvil_drop_transaction(tx_hash).await,
            EthRequest::DropAllTransactions() => self.anvil_drop_all_transactions().await,
            EthRequest::RemovePoolTransactions(address) => {
                self.anvil_remove_pool_transactions(address).await
            }
            _ => Err::<(), _>(Error::RpcUnimplemented).to_rpc_result(),
        };

        if let ResponseResult::Error(err) = &res {
            node_info!("\nRPC request failed:");
            node_info!("    Request: {:?}", req);
            node_info!("    Error: {}\n", err);
        }

        res
    }

    fn set_logging(&self, enabled: bool) -> Result<()> {
        node_info!("anvil_setLoggingEnabled");

        self.logging_manager.set_enabled(enabled);
        Ok(())
    }

    // Mining related RPCs.
    async fn mine(&self, blocks: Option<U256>, interval: Option<U256>) -> Result<()> {
        node_info!("anvil_mine");

        if blocks.is_some_and(|b| u64::try_from(b).is_err()) {
            return Err(Error::InvalidParams("The number of blocks is too large".to_string()))
        }
        if interval.is_some_and(|i| u64::try_from(i).is_err()) {
            return Err(Error::InvalidParams("The interval between blocks is too large".to_string()))
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

    async fn evm_mine(&self, mine: Option<MineParams<Option<MineOptions>>>) -> Result<String> {
        node_info!("evm_mine");

        self.mining_engine.evm_mine(mine.and_then(|p| p.params)).await?;
        Ok("0x0".to_string())
    }

    // TimeMachine RPCs
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
            return Err(Error::InvalidParams("The timestamp is too big".to_string()))
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
            return Err(Error::InvalidParams("The timestamp is too big".to_string()))
        }
        let time = timestamp.to::<u64>();
        Ok(self.mining_engine.set_time(Duration::from_secs(time)))
    }

    // Revive RPCs
    fn eth_chain_id(&self) -> Result<U64> {
        node_info!("eth_chainId");
        Ok(U256::from(self.eth_rpc_client.chain_id()).to::<U64>())
    }

    fn network_id(&self) -> Result<u64> {
        node_info!("eth_networkId");
        Ok(self.eth_rpc_client.chain_id())
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

    async fn get_balance(&self, addr: Address, block: Option<BlockId>) -> Result<U256> {
        node_info!("eth_getBalance");
        let hash = self
            .eth_rpc_client
            .block_hash_for_tag(ReviveBlockId::from(block).inner())
            .await
            .map_err(Error::Revive)?;
        let runtime_api = self.eth_rpc_client.runtime_api(hash);
        let balance =
            runtime_api.balance(ReviveAddress::from(addr).inner()).await.map_err(Error::Revive)?;
        Ok(AlloyU256::from(balance).inner())
    }

    async fn get_storage_at(
        &self,
        addr: Address,
        slot: U256,
        block: Option<BlockId>,
    ) -> Result<Bytes> {
        let hash = self
            .eth_rpc_client
            .block_hash_for_tag(ReviveBlockId::from(block).inner())
            .await
            .map_err(Error::Revive)?;
        let runtime_api = self.eth_rpc_client.runtime_api(hash);
        let bytes = runtime_api
            .get_storage(ReviveAddress::from(addr).inner(), slot.to_be_bytes())
            .await
            .map_err(Error::Revive)?;
        Ok(bytes.unwrap_or_default().into())
    }

    async fn get_code(&self, address: Address, block: Option<BlockId>) -> Result<Bytes> {
        let hash = self
            .eth_rpc_client
            .block_hash_for_tag(ReviveBlockId::from(block).inner())
            .await
            .map_err(Error::Revive)?;
        let code = self
            .eth_rpc_client
            .runtime_api(hash)
            .code(ReviveAddress::from(address).inner())
            .await
            .map_err(Error::Revive)?;
        Ok(code.into())
    }

    async fn get_block_by_hash(
        &self,
        block_hash: B256,
        hydrated_transactions: bool,
    ) -> Result<Option<Block>> {
        let Some(block) = self
            .eth_rpc_client
            .block_by_hash(&H256::from_slice(block_hash.as_slice()))
            .await
            .map_err(Error::Revive)?
        else {
            return Ok(None);
        };
        let block = self.eth_rpc_client.evm_block(block, hydrated_transactions).await;
        Ok(Some(block))
    }

    async fn estimate_gas(
        &self,
        request: WithOtherFields<TransactionRequest>,
        block: Option<alloy_rpc_types::BlockId>,
    ) -> Result<sp_core::U256> {
        node_info!("eth_estimateGas");

        let hash = self
            .eth_rpc_client
            .block_hash_for_tag(ReviveBlockId::from(block).inner())
            .await
            .map_err(Error::Revive)?;
        let runtime_api = self.eth_rpc_client.runtime_api(hash);
        let dry_run = runtime_api
            .dry_run(convert_to_generic_transaction(request.into_inner()))
            .await
            .map_err(Error::Revive)?;
        Ok(dry_run.eth_gas)
    }

    async fn gas_price(&self) -> Result<sp_core::U256> {
        let hash = self
            .eth_rpc_client
            .block_hash_for_tag(BlockTag::Latest.into())
            .await
            .map_err(Error::Revive)?;

        let runtime_api = self.eth_rpc_client.runtime_api(hash);
        runtime_api.gas_price().await.map_err(Error::Revive)
    }

    pub async fn get_transaction_count(
        &self,
        address: H160,
        block: BlockNumberOrTagOrHash,
    ) -> Result<sp_core::U256> {
        let hash = self.eth_rpc_client.block_hash_for_tag(block).await.map_err(Error::Revive)?;
        let runtime_api = self.eth_rpc_client.runtime_api(hash);
        let nonce = runtime_api.nonce(address).await.map_err(Error::Revive)?;
        Ok(nonce)
    }

    async fn send_raw_transaction(&self, transaction: Bytes) -> Result<H256> {
        let hash = H256(keccak_256(&transaction.0));
        let call = subxt_client::tx().revive().eth_transact(transaction.0);
        self.eth_rpc_client
            .submit(call)
            .await
            .map_err(|err| {
                node_info!("submit call failed: {err:?}");
                err
            })
            .map_err(Error::Revive)?;

        node_info!("send_raw_transaction hash: {hash:?}");
        Ok(hash)
    }

    pub(crate) async fn send_transaction(
        &self,
        transaction_req: WithOtherFields<TransactionRequest>,
    ) -> Result<H256> {
        let mut transaction = convert_to_generic_transaction(transaction_req.clone().into_inner());
        node_info!("{transaction:#?}");
        let Some(from) = transaction.from else {
            node_info!("Transaction must have a sender");
            return Err(Error::EthRpc(EthRpcError::InvalidTransaction));
        };

        let account = self
            .wallet
            .accounts
            .iter()
            .find(|account| account.address() == from)
            .ok_or(Error::EthRpc(EthRpcError::AccountNotFound(from)))?;

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
                Some(sp_core::U256::from_big_endian(&self.eth_chain_id()?.to_be_bytes::<8>()));
        }

        let tx = transaction
            .try_into_unsigned()
            .map_err(|_| Error::EthRpc(EthRpcError::InvalidTransaction))?;
        let payload = account.sign_transaction(tx).signed_payload();
        self.send_raw_transaction(Bytes(payload)).await
    }

    /// Returns transaction pool status - IMPLEMENTED
    async fn txpool_status(&self) -> ResponseResult {
        node_info!("txpool_status");
        let pool_status = self.tx_pool.status();
        // Convert Substrate PoolStatus to Ethereum TxpoolStatus format
        let status =
            TxpoolStatus { pending: pool_status.ready as u64, queued: pool_status.future as u64 };
        ResponseResult::Success(serde_json::to_value(status).unwrap_or_default())
    }

    /// Returns transaction summaries - NOT IMPLEMENTED
    async fn txpool_inspect(&self) -> ResponseResult {
        node_info!("txpool_inspect");
        // TODO: Convert Substrate transactions to TxpoolInspectSummary format
        let inspect = TxpoolInspect::default();
        ResponseResult::Success(serde_json::to_value(inspect).unwrap_or_default())
    }

    /// Returns full transaction details - NOT IMPLEMENTED
    async fn txpool_content(&self) -> ResponseResult {
        node_info!("txpool_content");
        // TODO: Convert Substrate transactions to AnyRpcTransaction format
        let content = TxpoolContent::<AnyRpcTransaction>::default();
        ResponseResult::Success(serde_json::to_value(content).unwrap_or_default())
    }

    /// Helper function to find transaction by ETH hash
    fn find_transaction_by_eth_hash(&self, target_hash: B256) -> Option<H256> {
        for tx in self.tx_pool.ready() {
            if let Ok(ext) = UncheckedExtrinsic::decode_all_with_depth_limit(
                MAX_EXTRINSIC_DEPTH,
                &mut &(tx.data.encode()[..]),
            ) {
                if let sp_runtime::generic::UncheckedExtrinsic {
                    function: RuntimeCall::Revive(pallet_revive::Call::eth_transact { payload }),
                    ..
                } = ext.0
                {
                    if let Ok(_signed_tx) = TransactionSigned::decode(&payload.to_vec()) {
                        // Calculate the Ethereum transaction hash manually
                        let eth_hash = keccak_256(&payload);
                        let eth_hash_b256 = B256::from_slice(&eth_hash);
                        if eth_hash_b256 == target_hash {
                            return Some(*tx.hash());
                        }
                    }
                }
            }
        }
        None
    }

    /// Drop specific transaction by hash - IMPLEMENTED - NEED TESTING!
    async fn anvil_drop_transaction(&self, tx_hash: B256) -> ResponseResult {
        node_info!("anvil_dropTransaction");

        if let Some(substrate_hash) = self.find_transaction_by_eth_hash(tx_hash) {
            let mut invalid_txs = IndexMap::new();
            invalid_txs.insert(substrate_hash, None);

            let removed = self.tx_pool.report_invalid(None, invalid_txs).await;
            ResponseResult::Success(serde_json::Value::Bool(!removed.is_empty()))
        } else {
            ResponseResult::Success(serde_json::Value::Bool(false))
        }
    }

    /// Drop all transactions from pool - IMPLEMENTED
    async fn anvil_drop_all_transactions(&self) -> ResponseResult {
        node_info!("anvil_dropAllTransactions");

        // Get all transactions from both queues
        let ready_txs = self.tx_pool.ready();
        let future_txs = self.tx_pool.futures();

        let mut invalid_txs = IndexMap::new();

        // Mark all ready transactions for removal
        for tx in ready_txs {
            invalid_txs.insert(*tx.hash(), None);
        }

        // Mark all future transactions for removal
        for tx in future_txs {
            invalid_txs.insert(*tx.hash(), None);
        }

        // Remove all transactions using report_invalid API
        let removed = self.tx_pool.report_invalid(None, invalid_txs).await;

        ResponseResult::Success(serde_json::Value::Bool(!removed.is_empty()))
    }

    /// Remove transactions from specific address - NOT IMPLEMENTED
    async fn anvil_remove_pool_transactions(&self, _address: Address) -> ResponseResult {
        node_info!("anvil_removePoolTransactions");

        // TODO: Convert ETH Address to Substrate AccountId format
        // Then filter transactions by sender and remove via report_invalid
        ResponseResult::Success(serde_json::Value::Bool(true))
    }
}
