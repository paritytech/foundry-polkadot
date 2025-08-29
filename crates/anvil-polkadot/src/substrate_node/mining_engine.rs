use super::{error::Error, service::TransactionPoolHandle};
use alloy_primitives::U256;
use futures::{
    channel::oneshot,
    stream::{unfold, FusedStream},
    task::AtomicWaker,
    SinkExt, StreamExt,
};
use parking_lot::RwLock;
use polkadot_sdk::{
    sc_consensus_manual_seal::{CreatedBlock, EngineCommand, Error as BlockProducingError},
    sc_service::TransactionPool,
    sp_core, *,
};
use std::{pin::Pin, sync::Arc};
use tokio::time::{interval_at, Duration, Instant, MissedTickBehavior};

use substrate_runtime::Runtime;
pub type Hash = <Runtime as frame_system::Config>::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiningMode {
    /// We are only producing blocks as an answer to the
    /// mine family of RPCs
    None,
    /// Create a new block every tick seconds.
    Interval { tick: u64 },
    /// Create a new block every time there is a transaction.
    AutoMining,
    /// A mix of the two mining modes above. We create a block
    /// either every tick seconds or anytime there is a new
    /// transaction
    MixedMining { tick: u64 },
}

pub struct MiningEngine {
    /// Coordination mechanism between the MiningEngine and the background
    /// task that runs the "polling" loop from `run_mining_engine`.
    /// Calls to inner.wake() will unpark the background task and force a
    /// receck of the current mining mode and rebuild of the polled streams.
    inner: Arc<MinnerInner>,
    pub mining_mode: Arc<RwLock<MiningMode>>,
    transaction_pool: Arc<TransactionPoolHandle>,
}

impl MiningMode {
    pub fn get_mode(block_time: Option<Duration>, mixed_mining: bool, no_mining: bool) -> Self {
        block_time.map_or_else(
            || if no_mining { Self::None } else { Self::AutoMining },
            |time| {
                if mixed_mining {
                    Self::MixedMining { tick: time.as_secs() }
                } else {
                    Self::Interval { tick: time.as_secs() }
                }
            },
        )
    }
}

impl MiningEngine {
    pub fn new(mining_mode: MiningMode, transaction_pool: Arc<TransactionPoolHandle>) -> Self {
        Self {
            inner: Default::default(),
            mining_mode: Arc::new(RwLock::new(mining_mode)),
            transaction_pool,
        }
    }

    pub async fn seal_now(
        &self,
        mut seal_command_sender: futures::channel::mpsc::Sender<EngineCommand<sp_core::H256>>,
    ) -> Result<CreatedBlock<Hash>, BlockProducingError> {
        let (sender, receiver) = oneshot::channel();
        let seal_command = EngineCommand::SealNewBlock {
            create_empty: true,
            finalize: true,
            parent_hash: None,
            sender: Some(sender),
        };
        seal_command_sender.send(seal_command).await?;
        match receiver.await {
            Ok(Ok(rx)) => Ok(rx),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(e.into()),
        }
    }

    pub fn wake(&self) {
        self.inner.wake();
    }

    pub fn is_automine(&self) -> bool {
        matches!(*self.mining_mode.read(), MiningMode::AutoMining)
    }

    // ---------------- RPC ----------------------

    pub async fn mine(
        &self,
        num_blocks: Option<U256>,
        interval: Option<U256>,
        seal_command_sender: futures::channel::mpsc::Sender<EngineCommand<sp_core::H256>>,
    ) -> Result<(), Error> {
        info!("anvil_polkadot_mine");
        let interval = interval.map(|i| i.to::<u64>());
        let blocks = num_blocks.unwrap_or(U256::from(1));
        if blocks.is_zero() {
            return Ok(());
        }
        for _ in 0..blocks.to::<u64>() {
            // After we invent the time machine skip forward in time
            // instead of sleeping so we can match the anvil behavior
            if let Some(interval) = interval {
                tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
            }
            self.seal_now(seal_command_sender.clone()).await?;
        }
        Ok(())
    }

    pub fn set_interval_mining(&self, interval: u64) -> Result<(), Error> {
        let new_mode =
            if interval == 0 { MiningMode::None } else { MiningMode::Interval { tick: interval } };
        *self.mining_mode.write() = new_mode;
        self.wake();
        Ok(())
    }

    pub fn get_interval_mining(&self) -> Result<u64, Error> {
        let mode = *self.mining_mode.read();
        match mode {
            MiningMode::Interval { tick } | MiningMode::MixedMining { tick } => Ok(tick),
            _ => Err(Error::MiningModeMismatch),
        }
    }

    pub fn get_auto_mine(&self) -> Result<bool, Error> {
        Ok(self.is_automine())
    }

    pub fn set_auto_mine(&self, enabled: bool) -> Result<(), Error> {
        let mining_mode = match (self.is_automine(), enabled) {
            (true, true) => Some(MiningMode::AutoMining),
            (true, false) => Some(MiningMode::None),
            (false, true) => Some(MiningMode::AutoMining),
            (false, false) => None,
        };
        if let Some(mining_mode) = mining_mode {
            *self.mining_mode.write() = mining_mode;
            self.wake();
        }
        Ok(())
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

// --------------- MiningEngine runner
type SealCommandStream = Pin<Box<dyn FusedStream<Item = ()> + Send>>;

fn build_interval_stream(interval: u64) -> SealCommandStream {
    let interval = Duration::from_secs(interval);
    let mut interval_ticker = interval_at(Instant::now() + interval, interval);
    interval_ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    let stream = unfold(interval_ticker, |mut interval_tick| async {
        interval_tick.tick().await;
        Some(((), interval_tick))
    });
    Box::pin(stream.fuse())
}

fn build_auto_stream(engine: &Arc<MiningEngine>) -> SealCommandStream {
    let stream = engine.transaction_pool.import_notification_stream().map(|_| ());
    Box::pin(stream.fuse())
}

fn build_stream_for_mode(
    mode: MiningMode,
    engine: &Arc<MiningEngine>,
) -> (Option<SealCommandStream>, Option<SealCommandStream>) {
    let interval_stream = match mode {
        MiningMode::Interval { tick } | MiningMode::MixedMining { tick } => Some(tick),
        _ => None,
    }
    .map(build_interval_stream);
    let auto_stream = matches!(mode, MiningMode::AutoMining | MiningMode::MixedMining { .. })
        .then(|| build_auto_stream(engine));
    (interval_stream, auto_stream)
}

async fn wait_for_mode_change(
    engine: &Arc<MiningEngine>,
    current: Option<MiningMode>,
) -> MiningMode {
    futures::future::poll_fn(|cx| {
        let mode = *engine.mining_mode.read();
        if current.as_ref().is_none_or(|m| *m != mode) {
            return std::task::Poll::Ready(mode);
        }
        engine.inner.register(cx);
        std::task::Poll::Pending
    })
    .await
}

async fn next_or_pending(stream: Option<&mut SealCommandStream>) -> Option<()> {
    match stream {
        Some(s) => s.next().await,
        None => futures::future::pending().await,
    }
}

async fn poll_and_seal(
    engine: Arc<MiningEngine>,
    command_seal_sender: futures::channel::mpsc::Sender<EngineCommand<sp_core::H256>>,
    stream: Option<&mut SealCommandStream>,
) -> bool {
    if next_or_pending(stream).await.is_some() {
        match engine.seal_now(command_seal_sender).await {
            Ok(block) => {
                debug!(hash=?block.hash, "sealed");
            }
            Err(BlockProducingError::Canceled(_) | BlockProducingError::SendError(_)) => {
                return false; // fatal: break outer loop
            }
            Err(e) => {
                error!(?e, "block production failed");
            }
        }
        true
    } else {
        true
    }
}
pub async fn run_mining_engine(
    engine: Arc<MiningEngine>,
    command_sink: futures::channel::mpsc::Sender<EngineCommand<sp_core::H256>>,
) {
    let mut current_mode = None;
    let mut interval_mining_stream: Option<SealCommandStream> = None;
    let mut auto_mining_stream: Option<SealCommandStream> = None;

    loop {
        tokio::select! {
            new_mode = wait_for_mode_change(&engine, current_mode) => {
                current_mode = Some(new_mode);
                let (interval_stream, auto_stream) = build_stream_for_mode(new_mode, &engine);
                interval_mining_stream = interval_stream;
                auto_mining_stream = auto_stream;
            }
            // Poll the interval stream if it exists
            continue_loop = poll_and_seal(engine.clone(), command_sink.clone(), interval_mining_stream.as_mut()) => {
                if !continue_loop {
                    break;
                }
            }
            // Poll the automining stream if it exists
            continue_loop = poll_and_seal(engine.clone(), command_sink.clone(), auto_mining_stream.as_mut()) => {
                if !continue_loop {
                    break;
                }
            }

        }
    }
}
