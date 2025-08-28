use super::service::TransactionPoolHandle;
use chrono::Local;
use futures::{
    stream::{unfold, FusedStream},
    task::AtomicWaker,
    SinkExt, StreamExt,
};
use parking_lot::{Mutex, RwLock};
use polkadot_sdk::{sc_consensus_manual_seal::EngineCommand, sp_core};
use std::{pin::Pin, sync::Arc};
use tokio::time::{interval, Duration, MissedTickBehavior};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiningMode {
    /// We are only producing blocks as an answer to the
    /// mine family of RPCs
    None,
    /// Create a new block every <tick> seconds.
    Interval { tick: u64 },
}

pub struct MiningEngine {
    inner: Arc<MinnerInner>,
    pub mining_mode: Arc<RwLock<MiningMode>>,
    transaction_pool: Arc<TransactionPoolHandle>,
    manual_command_sender: Mutex<futures::channel::mpsc::Sender<EngineCommand<sp_core::H256>>>,
}

impl MiningEngine {
    pub fn new(
        mining_mode: MiningMode,
        transaction_pool: Arc<TransactionPoolHandle>,
        receiver: futures::channel::mpsc::Sender<EngineCommand<sp_core::H256>>,
    ) -> Self {
        Self {
            inner: Default::default(),
            mining_mode: Arc::new(RwLock::new(mining_mode)),
            transaction_pool,
            manual_command_sender: Mutex::new(receiver),
        }
    }

    pub fn seal_now(&self) {
        use chrono::Local;
        // Maybe take parameters fir create_empty and finalize?
        let seal_command = EngineCommand::SealNewBlock {
            create_empty: true,
            finalize: true,
            parent_hash: None,
            sender: None,
        };
        let mut sender_guard = self.manual_command_sender.lock();
        // Error handling?
        let _err = sender_guard.try_send(seal_command);
        // Remove this later?
        info!("---->seal command sent at: {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
    }

    pub fn wake(&self) {
        self.inner.wake();
    }
}

#[derive(Default)]
struct MinnerInner {
    waker: AtomicWaker,
}

impl MinnerInner {
    pub fn wake(&self) {
        self.waker.wake();
    }

    pub fn register(&self, cx: &std::task::Context<'_>) {
        self.waker.register(cx.waker());
    }
}

pub async fn run_mining_engine(
    engine: Arc<MiningEngine>,
    mut command_sink: futures::channel::mpsc::Sender<EngineCommand<sp_core::H256>>,
) {
    let mut current_mode = None;
    let mut interval_mining_stream: Option<
        Pin<Box<dyn FusedStream<Item = EngineCommand<sp_core::H256>> + Send>>,
    > = None;

    loop {
        let mut rebuild_streams_future = futures::stream::poll_fn(|cx| {
            let mode = { *engine.mining_mode.read() };
            let mode_changed = current_mode.as_ref().map_or(true, |m| *m != mode);

            if mode_changed {
                current_mode = Some(mode);
                std::task::Poll::Ready(Some(()))
            } else {
                engine.inner.register(cx);
                std::task::Poll::Pending
            }
        });
        tokio::select! {
            _ = rebuild_streams_future.next() => {
                let mode = current_mode.clone().unwrap_or(MiningMode::None);
                interval_mining_stream = if let Some(tick) = match mode {
                    MiningMode::Interval {tick} => Some(tick),
                    _ => None,
                } {
                    let stream = unfold(interval(Duration::from_secs(tick)), |mut interval| async {
                        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
                        interval.tick().await;
                        Some((EngineCommand::SealNewBlock { create_empty: true, finalize: true, parent_hash: None, sender: None }, interval))
                    }).fuse();
                    Some(Box::pin(stream))
                }
                else {
                    None
                }
            },
            // Poll the interval stream if it exists
            maybe_cmd = async {
                match interval_mining_stream.as_mut() {
                    Some(stream) => stream.next().await,
                    // return a future that never resolves
                    None => futures::future::pending().await,
                }
            } => {
                if let Some(seal_command) = maybe_cmd {
                    info!("---->Interval miner ticked at: {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
                    if command_sink.send(seal_command).await.is_err() {
                        break;
                    }
                }
            },
        }
    }
}
