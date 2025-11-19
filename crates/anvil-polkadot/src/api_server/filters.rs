use crate::api_server::error::{Result, ToRpcResponseResult};
use anvil_core::eth::subscription::SubscriptionId;
use anvil_rpc::response::ResponseResult;
use futures::{FutureExt, Stream, StreamExt};
use pallet_revive_eth_rpc::client::Client as EthRpcClient;
use polkadot_sdk::pallet_revive::evm::{BlockNumberOrTag, Filter, Log};
use std::{collections::HashMap, sync::Arc, task::Poll, time::Duration};
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
                    EthFilter::Logs(logs_filter) => {
                        let logs = logs_filter.drain_logs().await;
                        Ok(logs).to_rpc_result()
                    }
                    _ => filter
                        .next()
                        .await
                        .unwrap_or_else(|| ResponseResult::success(Vec::<()>::new())),
                };
                *deadline = self.next_deadline();
                return response;
            }
        }
        warn!(target: "node::filter", "No filter found for {}", id);
        ResponseResult::success(Vec::<()>::new())
    }

    pub async fn get_log_filter(&self, id: &str) -> Option<Filter> {
        let filters = self.active_filters.lock().await;
        if let Some((EthFilter::Logs(log), _)) = filters.get(id) {
            return Some(log.filter.clone());
        }
        None
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
    Blocks(BlockNotifications),
    Logs(Box<LogsFilter>),
}

impl Stream for EthFilter {
    type Item = ResponseResult;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let pin = self.get_mut();
        match pin {
            Self::Blocks(block_notifications) => {
                let mut new_blocks = Vec::new();
                while let Poll::Ready(Some(result)) = block_notifications.poll_next_unpin(cx) {
                    match result {
                        Ok(block_hash) => new_blocks.push(block_hash),
                        Err(lagged) => {
                            // BroadcastStream handles lagging for us
                            // Just log and continue
                            warn!(target: "node::filter", "Block filter lagged, skipped messages {:?}", lagged);
                            continue;
                        }
                    }
                }
                Poll::Ready(Some(Ok(new_blocks).to_rpc_result()))
            }
            // handled directly in get_filter_changes
            Self::Logs(_) => Poll::Pending,
        }
    }
}

pub struct LogsFilter {
    blocks: BlockNotifications,
    eth_client: EthRpcClient,
    filter: Filter,
    historic: Option<Vec<Log>>,
}

impl LogsFilter {
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

    async fn drain_logs(&mut self) -> Vec<Log> {
        let mut logs = self.historic.take().unwrap_or_default();
        let mut block_hashes = vec![];
        while let Some(result) = self.blocks.next().now_or_never().flatten() {
            match result {
                Ok(block_hash) => block_hashes.push(block_hash),
                Err(BroadcastStreamRecvError::Lagged(blocks)) => {
                    // Channel overflowed - some blocks were skipped
                    warn!(target: "node::filter", "Logs filter lagged, skipped {} block notifications", blocks);
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
