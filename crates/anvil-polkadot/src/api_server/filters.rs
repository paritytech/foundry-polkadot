use crate::api_server::error::ToRpcResponseResult;
use anvil_core::eth::subscription::SubscriptionId;
use anvil_rpc::response::ResponseResult;
use futures::{Stream, StreamExt};
use std::{collections::HashMap, sync::Arc, task::Poll, time::Duration};
use subxt::utils::H256;
use tokio::{sync::Mutex, time::Instant};
use tokio_stream::wrappers::BroadcastStream;

pub const ACTIVE_FILTER_TIMEOUT_SECS: u64 = 60 * 5;
type FilterMap = Arc<Mutex<HashMap<String, (EthFilter, Instant)>>>;
pub type BlockNotifications = BroadcastStream<H256>;

#[derive(Clone, Debug)]
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
                let response = filter
                    .next()
                    .await
                    .unwrap_or_else(|| ResponseResult::success(Vec::<()>::new()));
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

#[derive(Debug)]
pub enum EthFilter {
    Blocks(BlockNotifications),
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
        }
    }
}
