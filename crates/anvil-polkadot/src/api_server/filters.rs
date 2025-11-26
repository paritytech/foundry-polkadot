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
use polkadot_sdk::{
    pallet_revive::evm::{BlockNumberOrTag, Filter, HashesOrTransactionInfos, Log},
    sc_service::TransactionPool,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
use subxt::utils::H256;
use tokio::{sync::Mutex, time::Instant};
use tokio_stream::wrappers::{BroadcastStream, errors::BroadcastStreamRecvError};

/// Default timeout duration for active filters in seconds.
/// Filters that haven't been polled within this duration will be evicted.
pub const ACTIVE_FILTER_TIMEOUT_SECS: u64 = 60 * 5;

pub const LOG_TARGET: &str = "node::filter";

/// Maps filter IDs to tuples of filter and deadline.
type FilterMap = Arc<Mutex<HashMap<String, (EthFilter, Instant)>>>;

/// Type alias for block notification streams.
pub type BlockNotifications = BroadcastStream<H256>;

/// Manages Ethereum style filters for block notifications, logs and pending transactions.
///
/// Maintains active filters and automatically evicts that haven't been polled within the
/// keep alive duration. Each filter is identified by a unique hexa string.
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

    /// Inserts a new Filter and returns its unique identifier.
    pub async fn add_filter(&self, filter: EthFilter) -> String {
        let id = new_id();
        trace!(target: "node::filters", "Adding new filter id {}", id);
        let mut filters = self.active_filters.lock().await;
        filters.insert(id.clone(), (filter, self.next_deadline()));
        id
    }

    /// Poll the filter for changes since the last call.
    ///
    /// This method retrieves any new data from the specified filter and resets its deadline.
    ///
    ///  - Block filters: Returns an array of new block hashes
    ///  - Log filters: Returns an array of logs matching the filter criteria, including both
    ///    historic logs on the first poll and new logs from blocks produced since the last poll.
    pub async fn get_filter_changes(&self, id: &str) -> ResponseResult {
        {
            let mut filters = self.active_filters.lock().await;
            if let Some((filter, deadline)) = filters.get_mut(id) {
                let response = match filter {
                    EthFilter::Blocks(block_filter) => {
                        let blocks = block_filter.drain_blocks().await;
                        Ok(blocks).to_rpc_result()
                    }
                    EthFilter::Logs(logs_filter) => {
                        let logs = logs_filter.drain_logs().await;
                        Ok(logs).to_rpc_result()
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
        warn!(target: LOG_TARGET, "No filter found for {}", id);
        ResponseResult::success(Vec::<()>::new())
    }

    /// Returns the log filter criteria for a given filter ID.
    pub async fn get_log_filter(&self, id: &str) -> Option<Filter> {
        let filters = self.active_filters.lock().await;
        if let Some((EthFilter::Logs(log), _)) = filters.get(id) {
            return Some(log.filter.clone());
        }
        None
    }

    /// Returns the keepalive duration for filters.
    pub fn keep_alive(&self) -> Duration {
        self.keep_alive
    }

    /// Removes and returns the filter associated with the given identifier.
    pub async fn uninstall_filter(&self, id: &str) -> Option<EthFilter> {
        trace!(target: LOG_TARGET, "Uninstalling filter id {}", id);
        self.active_filters.lock().await.remove(id).map(|(f, _)| f)
    }

    /// Evicts all filters that have exceeded their keepalive deadline.
    ///
    /// This method is typically called periodically by the eviction task to clean up
    /// stale filters that haven't been polled recently. Evicted filters are permanently
    /// removed and cannot be recovered.
    pub async fn evict(&self) {
        trace!(target: LOG_TARGET, "Evicting stale filters");
        let now = Instant::now();
        let mut active_filters = self.active_filters.lock().await;
        active_filters.retain(|id, (_, deadline)| {
            if now > *deadline {
                trace!(target: LOG_TARGET,?id, "Evicting stale filter");
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

/// Background task that periodically evicts stale filters.
///
/// This task runs an infinite loop that calls `Filters::evict()` at regular intervals
/// based on the filter keepalive duration. It ensures that filters which haven't been
/// polled are automatically removed to prevent memory leaks.
///
/// The task should be spawned once when the filter system is initialized and will
/// run for the lifetime of the application.
pub async fn eviction_task(filters: Filters) {
    let start = filters.next_deadline();
    let mut interval = tokio::time::interval_at(start, filters.keep_alive());
    loop {
        interval.tick().await;
        filters.evict().await;
    }
}

/// Implements the Ethereum JSON-RPC filter specification, supporting block
/// log and pending transactions filtering capabilities. Each filter type
/// has different polling behavior and data delivery semantics.
pub enum EthFilter {
    /// Block filter that streams new block hashes.
    ///
    /// Emits the hash (H256) of each new block as it's added to the chain.
    /// Subscribers receive notifications through the broadcast channel. When polled,
    /// returns all block hashes produced since the last poll.
    Blocks(BlockFilter),
    /// Log filter that tracks contract event logs.
    ///
    /// Filters logs based on block range, addresses, and topics. Combines historic
    /// logs (from the initial query range) with real-time logs from newly produced
    /// blocks. The filter applies topic matching with OR logic between topic alternatives
    /// and validates block ranges for incoming blocks.
    Logs(LogsFilter),
    /// Pending transactions filter that tracks new transactions.
    ///
    /// Returns mined transactions since last poll + transactions that are
    /// ready but have not been mined yet.
    PendingTransactions(PendingTransactionsFilter),
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
                        target: LOG_TARGET,
                        "Block filter lagged, skipped {} block notifications",
                        count
                    );
                }
            }
        }

        new_blocks
    }
}

/// Filter for tracking and collecting contract event logs.
///
/// Combines historic log queries with real-time log streaming to provide
/// a complete view of logs matching the filter criteria. On creation, it optionally
/// fetches historic logs based on the specified block range. Subsequently, it monitors
/// new blocks and queries them for matching logs.
///
/// The filter validates that incoming blocks are within the specified range (from_block
/// to to_block) before querying them, ensuring efficient operation and correct semantics.
pub struct LogsFilter {
    /// Stream of new block notifications
    blocks: BlockNotifications,
    /// Client for querying Ethereum RPC endpoints
    eth_client: EthRpcClient,
    /// Filter criteria (addresses, topics, block range)
    filter: Filter,
    /// Historic logs fetched at filter creation time, returned on first poll
    historic: Option<Vec<Log>>,
}

impl LogsFilter {
    /// Creates a new log filter with the specified criteria.
    ///
    /// If the filter specifies a block range (from_block, to_block) or specific block hash,
    /// this constructor will immediately query for historic logs matching the criteria.
    /// These historic logs are stored and returned on the first call to `get_filter_changes`.
    ///
    /// For filters without explicit block constraints, only real-time logs from future
    /// blocks will be collected.
    pub async fn new(
        block_notifier: BlockNotifications,
        eth_rpc_client: EthRpcClient,
        filter: Filter,
    ) -> Result<Self> {
        let historic = if filter.from_block.is_some()
            || filter.to_block.is_some()
            || filter.block_hash.is_some()
        {
            eth_rpc_client.logs(Some(filter.clone())).await.ok()
        } else {
            None
        };
        Ok(Self { blocks: block_notifier, eth_client: eth_rpc_client, filter, historic })
    }

    /// Drains all accumulated logs since the last poll.
    ///
    /// This method:
    /// 1. Takes any historic logs (returned only on first call)
    /// 2. Drains all pending block notifications without blocking
    /// 3. For each new block, checks if it's within the filter's block range
    /// 4. Queries each relevant block for logs matching the filter criteria
    /// 5. Returns the combined set of logs
    async fn drain_logs(&mut self) -> Vec<Log> {
        let mut logs = self.historic.take().unwrap_or_default();
        let mut block_hashes = vec![];
        while let Some(result) = self.blocks.next().now_or_never().flatten() {
            match result {
                Ok(block_hash) => block_hashes.push(block_hash),
                Err(BroadcastStreamRecvError::Lagged(blocks)) => {
                    // Channel overflowed - some blocks were skipped
                    warn!(target: LOG_TARGET, "Logs filter lagged, skipped {} block notifications", blocks);
                    // Continue draining what's left in the channel
                    continue;
                }
            }
        }

        // For each block that we were notified about check for logs
        for substrate_hash in block_hashes {
            // This can be optimized if we also submit the block number
            // from subscribe_and_cache_new_blocks
            if !self.is_block_in_range(&substrate_hash).await {
                continue;
            }
            let mut block_filter = self.filter.clone();
            block_filter.from_block = None;
            block_filter.to_block = None;
            block_filter.block_hash = self.eth_client.resolve_ethereum_hash(&substrate_hash).await;
            if let Ok(block_logs) = self.eth_client.logs(Some(block_filter)).await {
                logs.extend(block_logs);
            }
        }
        logs
    }

    /// Validates both lower bound (from_block) and upper bound (to_block) constraints.
    /// Block tags (like "latest", "pending") are always considered in range.
    async fn is_block_in_range(&self, substrate_hash: &H256) -> bool {
        let Ok(Some(block)) = self.eth_client.block_by_hash(substrate_hash).await else {
            return false; // Can't get block, skip it
        };

        let block_number = block.number();
        // Check lower limit (from_block)
        if let Some(from_block) = &self.filter.from_block {
            match from_block {
                BlockNumberOrTag::U256(limit) => {
                    if block_number < limit.as_u32() {
                        return false;
                    }
                }
                BlockNumberOrTag::BlockTag(_) => {}
            }
        }
        // Check upper limit (to_block)
        if let Some(to_block) = &self.filter.to_block {
            match to_block {
                BlockNumberOrTag::U256(limit) => {
                    if block_number > limit.as_u32() {
                        return false;
                    }
                }
                BlockNumberOrTag::BlockTag(_) => {}
            }
        }
        true
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
                    warn!(target: LOG_TARGET, "Failed to extract transaction info from ready pool");
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
                            target: LOG_TARGET,
                            "Failed to fetch transactions for block {:?}: {}",
                            block_hash, e
                        );
                    }
                },
                Err(BroadcastStreamRecvError::Lagged(blocks)) => {
                    // Channel overflowed - some blocks were skipped
                    warn!(target: LOG_TARGET, "Logs filter lagged, skipped {} block notifications", blocks);
                    // Continue draining what's left in the channel
                    continue;
                }
            }
        }

        // New from pool: transactions in ready pool we haven't seen before
        let new_from_pool: HashSet<H256> =
            current_ready.difference(&self.already_seen).copied().collect();
        let excluded: HashSet<H256> = self.already_seen.union(&new_from_pool).copied().collect();
        let new_from_blocks: HashSet<H256> =
            included_transactions.difference(&excluded).copied().collect();
        let new_pending: Vec<H256> = new_from_pool.union(&new_from_blocks).copied().collect();
        // Remove mined transactions from already_seen
        for tx_hash in &included_transactions {
            self.already_seen.remove(tx_hash);
        }

        // Only track transactions that are still pending (not mined)
        let still_pending: HashSet<H256> =
            current_ready.difference(&included_transactions).copied().collect();
        self.already_seen.extend(still_pending);
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
            // Considering that we called evm_block with hydrated false we will
            // never receive TransactionInfos but handled it anyways.
            HashesOrTransactionInfos::TransactionInfos(infos) => {
                infos.iter().map(|ti| ti.hash).collect()
            }
        };
        Ok(tx_hashes)
    }
}
