use crate::{
    api_server::{
        error::{Error, Result, ToRpcResponseResult},
        txpool_helpers::extract_tx_info,
    },
    substrate_node::service::TransactionPoolHandle,
};
use anvil_core::eth::subscription::SubscriptionId;
use anvil_rpc::response::ResponseResult;
use futures::{FutureExt, StreamExt};
use pallet_revive_eth_rpc::client::Client as EthRpcClient;
use polkadot_sdk::{pallet_revive::evm::HashesOrTransactionInfos, sc_service::TransactionPool};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
use subxt::utils::H256;
use tokio::{sync::Mutex, time::Instant};
use tokio_stream::wrappers::{BroadcastStream, errors::BroadcastStreamRecvError};

pub const ACTIVE_FILTER_TIMEOUT_SECS: u64 = 60 * 5;
type FilterMap = Arc<Mutex<HashMap<String, (EthFilter, Instant)>>>;
pub type BlockNotifications = BroadcastStream<H256>;

#[derive(Clone)]
pub struct Filters {
    /// Currently active filters
    active_filters: FilterMap,
    /// Lifetime of a filter
    keep_alive: Duration,
}

impl Filters {
    /// Creates a new Filters instance with custom keepalive duration
    pub fn with_keepalive(keep_alive: Duration) -> Self {
        Self { active_filters: Arc::new(Mutex::new(HashMap::default())), keep_alive }
    }

    /// Inserts a new Filter
    pub async fn add_filter(&self, filter: EthFilter) -> String {
        let id = new_id();
        trace!(target: "node::filters", "Adding new filter id {}", id);
        let mut filters = self.active_filters.lock().await;
        filters.insert(id.clone(), (filter, self.next_deadline()));
        id
    }

    /// Poll the filter for updates.
    pub async fn get_filter_changes(&self, id: &str) -> ResponseResult {
        {
            let mut filters = self.active_filters.lock().await;
            if let Some((filter, deadline)) = filters.get_mut(id) {
                let response = match filter {
                    EthFilter::Blocks(block_filter) => {
                        let blocks = block_filter.drain_blocks().await;
                        Ok(blocks).to_rpc_result()
                    }
                    EthFilter::PendingTransactions(tx_filter) => {
                        let txs = tx_filter.drain_transactions().await;
                        Ok(txs).to_rpc_result()
                    }
                };
                *deadline = self.next_deadline();
                return response;
            }
        }
        warn!(target: "node::filter", "No filter found for {}", id);
        ResponseResult::success(Vec::<()>::new())
    }

    /// The lifetime of filters
    pub fn keep_alive(&self) -> Duration {
        self.keep_alive
    }

    /// Removes the filter associated with the given id.
    pub async fn uninstall_filter(&self, id: &str) -> Option<EthFilter> {
        trace!(target: "node::filter", "Uninstalling filter id {}", id);
        self.active_filters.lock().await.remove(id).map(|(f, _)| f)
    }

    pub async fn evict(&self) {
        trace!(target: "node::filter", "Evicting stale filters");
        let now = Instant::now();
        let mut active_filters = self.active_filters.lock().await;
        active_filters.retain(|id, (_, deadline)| {
            if now > *deadline {
                trace!(target: "node::filter",?id, "Evicting stale filter");
                return false;
            }
            true
        });
    }

    fn next_deadline(&self) -> Instant {
        Instant::now() + self.keep_alive()
    }
}

impl Default for Filters {
    fn default() -> Self {
        Self {
            active_filters: Arc::new(Mutex::new(HashMap::default())),
            keep_alive: Duration::from_secs(ACTIVE_FILTER_TIMEOUT_SECS),
        }
    }
}

fn new_id() -> String {
    SubscriptionId::random_hex().to_string()
}

pub async fn eviction_task(filters: Filters) {
    let start = filters.next_deadline();
    let mut interval = tokio::time::interval_at(start, filters.keep_alive());
    loop {
        interval.tick().await;
        filters.evict().await;
    }
}

pub enum EthFilter {
    Blocks(BlockFilter),
    PendingTransactions(Box<PendingTransactionsFilter>),
}

/// Filter for tracking new block hashes.
pub struct BlockFilter {
    block_notifications: BlockNotifications,
}

impl BlockFilter {
    pub fn new(block_notifier: BlockNotifications) -> Self {
        Self { block_notifications: block_notifier }
    }

    /// Drains all new block hashes since the last poll.
    ///
    /// Returns all block hashes that were broadcast since the last call to this method.
    /// Handles lagged notifications gracefully by logging and continuing.
    async fn drain_blocks(&mut self) -> Vec<H256> {
        let mut new_blocks = Vec::new();

        while let Some(result) = self.block_notifications.next().now_or_never().flatten() {
            match result {
                Ok(block_hash) => new_blocks.push(block_hash),
                Err(BroadcastStreamRecvError::Lagged(count)) => {
                    warn!(
                        target: "node::filter",
                        "Block filter lagged, skipped {} block notifications",
                        count
                    );
                }
            }
        }

        new_blocks
    }
}

/// Filter for pending transactions
///
/// Monitors the transaction pool and returns newly pending transaction hashes
/// when polled. Transactions that have been included in old blocks are automatically filtered out.
///
/// The filter maintains state of previously seen transactions to ensure each
/// transaction is reported only once, even if it remains in the pending pool
/// across multiple polls.
pub struct PendingTransactionsFilter {
    /// Set of transaction hashes already reported to the client
    already_seen: HashSet<H256>,
    /// Stream of new block notifications for detecting mined transactions
    block_notifications: BroadcastStream<H256>,
    /// Reference to the transaction pool for querying ready transactions
    tx_pool: Arc<TransactionPoolHandle>,
    /// Ethereum RPC client for fetching block transaction data
    eth_rpc_client: EthRpcClient,
}

impl PendingTransactionsFilter {
    pub fn new(
        block_notifier: BroadcastStream<H256>,
        tx_pool: Arc<TransactionPoolHandle>,
        eth_rpc_client: EthRpcClient,
    ) -> Self {
        Self {
            already_seen: tx_pool
                .ready()
                .filter_map(|tx| extract_tx_info(&tx.data).map(|(_, _, tx_info)| tx_info.hash))
                .collect(),
            block_notifications: block_notifier,
            tx_pool,
            eth_rpc_client,
        }
    }

    /// Drains all new pending transaction hashes since the last poll.
    ///
    /// This method:
    /// 1. Queries the current ready transaction pool
    /// 2. Drains block notifications to identify mined transactions
    /// 3. Returns only new transactions (not previously seen and not mined)
    ///
    /// The filter state is updated to remember all currently pending transactions,
    /// ensuring they won't be reported again on subsequent polls.
    async fn drain_transactions(&mut self) -> Vec<H256> {
        // Get current ready transactions
        let current_ready: HashSet<H256> = self
            .tx_pool
            .ready()
            .filter_map(|tx| {
                extract_tx_info(&tx.data).map(|(_, _, tx_info)| tx_info.hash).or_else(|| {
                warn!(target: "node::filter", "Failed to extract transaction info from ready pool");
                None
            })
            })
            .collect();

        // Get transactions that have been included in blocks already
        let mut included_transactions = HashSet::new();
        while let Some(result) = self.block_notifications.next().now_or_never().flatten() {
            match result {
                Ok(block_hash) => match self.fetch_block_transactions(&block_hash).await {
                    Ok(tx_hashes) => included_transactions.extend(tx_hashes),
                    Err(e) => {
                        warn!(
                            target: "node::filter",
                            "Failed to fetch transactions for block {:?}: {}",
                            block_hash, e
                        );
                    }
                },
                Err(BroadcastStreamRecvError::Lagged(blocks)) => {
                    // Channel overflowed - some blocks were skipped
                    warn!(target: "node::filter", "Logs filter lagged, skipped {} block notifications", blocks);
                    // Continue draining what's left in the channel
                    continue;
                }
            }
        }
        // New pending = transactions from current pool + transactions from mined blocks
        // (that we haven't reported yet)
        let new_from_pool: HashSet<H256> =
            current_ready.difference(&self.already_seen).copied().collect();
        let new_from_blocks: HashSet<H256> =
            included_transactions.difference(&self.already_seen).copied().collect();

        let new_pending: Vec<H256> = new_from_pool.union(&new_from_blocks).copied().collect();

        // Update seen to include both current pool and mined transactions
        self.already_seen.extend(current_ready);
        self.already_seen.extend(included_transactions);
        new_pending
    }

    /// Fetches all transaction hashes from a given block.
    ///
    /// Takes a substrate block hash, fetches the block, converts it to an EVM block,
    /// and extracts all transaction hashes regardless of whether they're returned
    /// as hashes or full transaction objects.
    async fn fetch_block_transactions(&self, substrate_block_hash: &H256) -> Result<Vec<H256>> {
        let substrate_block =
            self.eth_rpc_client.block_by_hash(substrate_block_hash).await?.ok_or(
                Error::InternalError(format!(
                    "Could not find block with hash: {substrate_block_hash}"
                )),
            )?;
        let block = self
            .eth_rpc_client
            .evm_block(substrate_block, false)
            .await
            .ok_or(Error::InternalError("Could not convert to an evm block".to_string()))?;
        let tx_hashes = match block.transactions {
            HashesOrTransactionInfos::Hashes(hashes) => hashes,
            HashesOrTransactionInfos::TransactionInfos(infos) => {
                infos.iter().map(|ti| ti.hash).collect()
            }
        };
        Ok(tx_hashes)
    }
}
