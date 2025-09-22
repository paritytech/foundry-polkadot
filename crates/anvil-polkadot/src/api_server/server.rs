use crate::{
    api_server::{
        convert::{
            from_address_to_h160, from_alloy_u256_to_sp_u256, from_h160_to_address,
            to_block_number_or_tag_or_hash,
        },
        error::{Error, Result},
        ApiRequest,
    },
    config::AnvilNodeConfig,
    logging::LoggingManager,
    macros::node_info,
    substrate_node::{
        error::ToRpcResponseResult, mining_engine::MiningEngine, service::TransactionPoolHandle,
    },
};
use alloy_primitives::U256;
use anvil::eth::sign::DevSigner;
use anvil_core::eth::EthRequest;
use anvil_rpc::{error::RpcError, response::ResponseResult};
use futures::{channel::mpsc, StreamExt};
use polkadot_sdk::{
    pallet_revive::evm::Account,
    pallet_revive_eth_rpc::{
        client::Client as EthRpcClient, subxt_client::SrcChainConfig, ReceiptExtractor,
        ReceiptProvider, SubxtBlockInfoProvider,
    },
    sc_service::RpcHandlers,
    sp_core::H256,
};
use sqlx::sqlite::SqlitePoolOptions;
use std::{sync::Arc, time::Duration};
use subxt::{
    backend::rpc::{RawRpcFuture, RawRpcSubscription, RawValue, RpcClient, RpcClientT},
    ext::{
        jsonrpsee::core::traits::ToRpcParams,
        subxt_rpcs::{Error as SubxtRpcError, LegacyRpcMethods},
    },
    OnlineClient,
};
use subxt_signer::ecdsa::dev;

use futures::Stream;
use std::pin::Pin;

pub struct Wallet {
    pub(crate) accounts: Vec<Account>,
}

pub struct ApiServer {
    req_receiver: mpsc::Receiver<ApiRequest>,
    logging_manager: LoggingManager,
    mining_engine: Arc<MiningEngine>,
    pub(crate) eth_rpc_client: EthRpcClient,
    pub(crate) wallet: Wallet,
    pub(crate) tx_pool: Arc<TransactionPoolHandle>,
}

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
        use serde_json::Value;
        println!("{:?}", sub);
        Box::pin(async move {
            let subscription = self
                .0
                .handle()
                .subscribe_unbounded(sub, Params(params))
                .await
                .map_err(|err| SubxtRpcError::Client(Box::new(err)))?;

            let id: Value = Value::from(subscription.subscription_id().to_owned());
            let stream = async_stream::stream! {
                let mut sub = subscription;
                loop {
                    match sub.next::<Box<RawValue>>().await {
                        Some(Ok((notification, _sub_id))) => {
                            yield Ok(notification);
                        }
                        Some(Err(e)) => {
                            yield Err(SubxtRpcError::Client(Box::new(e)));
                            break;
                        }
                        None => {
                            // Subscription ended
                            break;
                        }
                    }
                }
            };
            // Try creating RawRpcSubscription directly
            Ok(RawRpcSubscription {
                stream: Box::pin(stream),
                id: id.as_str().map(|s| s.to_string()),
            })
        })
    }
}

impl ApiServer {
    pub async fn new(
        mining_engine: Arc<MiningEngine>,
        rpc_handlers: RpcHandlers,
        req_receiver: mpsc::Receiver<ApiRequest>,
        logging_manager: LoggingManager,
        tx_pool: Arc<TransactionPoolHandle>,
    ) -> Self {
        use alloy_primitives::address;
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

        let acc = Account::from(subxt_signer::eth::Keypair::from(dev::alice()));
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
            EthRequest::EthChainId(()) => self.eth_chain_id().to_rpc_result(),
            EthRequest::EthNetworkId(()) => self.network_id().to_rpc_result(),
            EthRequest::NetListening(()) => self.net_listening().to_rpc_result(),
            EthRequest::EthSyncing(()) => self.syncing().to_rpc_result(),
            EthRequest::EthGetTransactionReceipt(tx_hash) => {
                self.transaction_receipt(tx_hash).await.to_rpc_result()
            }
            EthRequest::Mine(blocks, interval) => {
                if blocks.is_some_and(|b| u64::try_from(b).is_err()) {
                    return ResponseResult::Error(RpcError::invalid_params(
                        "The number of blocks is too large",
                    ));
                }
                if interval.is_some_and(|i| u64::try_from(i).is_err()) {
                    return ResponseResult::Error(RpcError::invalid_params(
                        "The interval between blocks is too large",
                    ));
                }
                self.mining_engine
                    .mine(blocks.map(|b| b.to()), interval.map(|i| Duration::from_secs(i.to())))
                    .await
                    .to_rpc_result()
            }
            EthRequest::SetIntervalMining(interval) => self
                .mining_engine
                .set_interval_mining(Duration::from_secs(interval))
                .to_rpc_result(),
            EthRequest::GetIntervalMining(()) => {
                self.mining_engine.get_interval_mining().to_rpc_result()
            }
            EthRequest::GetAutoMine(()) => self.mining_engine.get_auto_mine().to_rpc_result(),
            EthRequest::SetAutomine(enabled) => {
                self.mining_engine.set_auto_mine(enabled).to_rpc_result()
            }
            EthRequest::EvmMine(mine) => {
                self.mining_engine.evm_mine(mine.and_then(|p| p.params)).await.to_rpc_result()
            }
            EthRequest::EvmMineDetailed(_mine) => ResponseResult::Error(RpcError::internal_error()),
            //------- TimeMachine---------
            EthRequest::EvmSetBlockTimeStampInterval(time) => self
                .mining_engine
                .set_block_timestamp_interval(Duration::from_secs(time))
                .to_rpc_result(),
            EthRequest::EvmRemoveBlockTimeStampInterval(()) => {
                self.mining_engine.remove_block_timestamp_interval().to_rpc_result()
            }
            EthRequest::EvmSetNextBlockTimeStamp(time) => {
                if time >= U256::from(u64::MAX) {
                    return ResponseResult::Error(RpcError::invalid_params(
                        "The timestamp is too big",
                    ))
                }
                let time = time.to::<u64>();
                self.mining_engine
                    .set_next_block_timestamp(Duration::from_secs(time))
                    .to_rpc_result()
            }
            EthRequest::EvmIncreaseTime(time) => self
                .mining_engine
                .increase_time(Duration::from_secs(time.try_into().unwrap_or(0)))
                .to_rpc_result(),
            EthRequest::EvmSetTime(timestamp) => {
                if timestamp >= U256::from(u64::MAX) {
                    return ResponseResult::Error(RpcError::invalid_params(
                        "The timestamp is too big",
                    ))
                }
                // Make sure here we are not traveling back in time.
                let time = timestamp.to::<u64>();
                self.mining_engine.set_time(Duration::from_secs(time)).to_rpc_result()
            }
            EthRequest::EthEstimateGas(call, block, _overrides) => {
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
            _ => Err::<(), _>(Error::RpcUnimplemented).to_rpc_result(),
        };

        if let ResponseResult::Error(err) = &res {
            node_info!("\nRPC request failed:");
            node_info!("    Request: {:?}", res);
            node_info!("    Error: {}\n", err);
        }

        res
    }

    fn set_logging(&self, enabled: bool) -> Result<()> {
        node_info!("anvil_setLoggingEnabled");
        self.logging_manager.set_enabled(enabled);
        Ok(())
    }
}
