use crate::substrate_node::{error::Error, service::TransactionPoolHandle};
use alloy_primitives::U256;
use alloy_rpc_types::anvil::MineOptions;
use anvil::eth::backend::time::TimeManager;
use futures::{
    channel::{mpsc::Sender, oneshot},
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
type Hash = <Runtime as frame_system::Config>::Hash;

#[derive(Debug, thiserror::Error)]
pub enum MiningError {
    #[error("Block production failed: {0:?}")]
    BlockProducing(BlockProducingError),
    #[error("Current mining mode can not answer this query.")]
    MiningModeMismatch,
    #[error("Current timestamp is newer.")]
    Timestamp,
}

impl From<polkadot_sdk::sc_consensus_manual_seal::Error> for MiningError {
    fn from(err: polkadot_sdk::sc_consensus_manual_seal::Error) -> Self {
        Self::BlockProducing(err)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiningMode {
    /// We are only producing blocks as an answer to the
    /// mine family of RPCs
    None,
    /// Create a new block every tick milliseconds.
    Interval { tick: u64 },
    /// Create a new block every time there is a transaction.
    AutoMining,
    /// A mix of the two mining modes above. We create a block
    /// either every tick milliseconds or anytime there is a new
    /// transaction
    MixedMining { tick: u64 },
}

impl MiningMode {
    pub fn new(block_time: Option<Duration>, mixed_mining: bool, no_mining: bool) -> Self {
        block_time.map_or_else(
            || if no_mining { Self::None } else { Self::AutoMining },
            |time| {
                if mixed_mining {
                    Self::MixedMining { tick: time.as_millis() as u64 }
                } else {
                    Self::Interval { tick: time.as_millis() as u64 }
                }
            },
        )
    }
}

pub struct MiningEngine {
    /// Coordination mechanism between the MiningEngine and the background
    /// task that runs the "polling" loop from `run_mining_engine`.
    /// Calls to waker.wake() will unpark the background task and force a
    /// recheck of the current mining mode and rebuild of the polled streams.
    waker: Arc<AtomicWaker>,
    mining_mode: Arc<RwLock<MiningMode>>,
    transaction_pool: Arc<TransactionPoolHandle>,
    time_manager: Arc<TimeManager>,
    seal_command_sender: Sender<EngineCommand<sp_core::H256>>,
}

impl MiningEngine {
    pub fn new(
        mining_mode: MiningMode,
        transaction_pool: Arc<TransactionPoolHandle>,
        time_manager: Arc<TimeManager>,
        seal_command_sender: Sender<EngineCommand<sp_core::H256>>,
    ) -> Self {
        Self {
            waker: Default::default(),
            mining_mode: Arc::new(RwLock::new(mining_mode)),
            transaction_pool,
            time_manager,
            seal_command_sender,
        }
    }

    pub async fn mine(
        &self,
        num_blocks: Option<U256>,
        interval: Option<U256>,
    ) -> Result<(), Error> {
        info!("anvil_polkadot_mine");
        let interval = interval.map(|i| i.to::<u64>());
        let blocks = num_blocks.unwrap_or(U256::from(1));
        if blocks.is_zero() {
            return Ok(());
        }
        for _ in 0..blocks.to::<u64>() {
            if let Some(interval) = interval {
                self.time_manager.increase_time(interval);
            }
            self.seal_now().await.map_err(|e| Error::Mining(e.into()))?;
        }
        Ok(())
    }

    pub async fn evm_mine(&self, opts: Option<MineOptions>) -> Result<String, Error> {
        info!("evm_mine");
        self.do_evm_mine(opts).await?;
        Ok("0x0".to_string())
    }

    pub fn set_interval_mining(&self, interval: u64) -> Result<(), Error> {
        let interval = interval.saturating_mul(1000);
        let new_mode =
            if interval == 0 { MiningMode::None } else { MiningMode::Interval { tick: interval } };
        *self.mining_mode.write() = new_mode;
        self.wake();
        Ok(())
    }

    pub fn get_interval_mining(&self) -> Result<u64, Error> {
        let mode = *self.mining_mode.read();
        match mode {
            MiningMode::Interval { tick } | MiningMode::MixedMining { tick } => {
                Ok(tick.saturating_div(1000))
            }
            _ => Err(Error::Mining(MiningError::MiningModeMismatch)),
        }
    }

    pub fn get_auto_mine(&self) -> Result<bool, Error> {
        Ok(self.is_automine())
    }

    pub fn set_auto_mine(&self, enabled: bool) -> Result<(), Error> {
        let mining_mode = match (self.is_automine(), enabled) {
            (true, true) => None,
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

    pub fn set_next_block_timestamp(&self, time_in_seconds: u64) -> Result<(), Error> {
        self.time_manager
            // this will convert the time_in_seconds in milliseconds. It is transparent
            // to the user
            .set_next_block_timestamp(time_in_seconds)
            .map_err(|_| Error::Mining(MiningError::Timestamp))
    }

    pub fn increase_time(&self, time_in_seconds: u64) -> Result<i64, Error> {
        Ok(self.time_manager.increase_time(time_in_seconds) as i64)
    }

    pub fn set_time(&self, timestamp: u64) -> Result<u64, Error> {
        let now = self.time_manager.current_call_timestamp();
        self.time_manager.reset(timestamp);
        let offset = timestamp.saturating_sub(now);
        Ok(Duration::from_millis(offset).as_millis() as u64)
    }

    pub fn set_block_timestamp_interval(&self, interval_in_seconds: u64) -> Result<(), Error> {
        self.time_manager.set_block_timestamp_interval(interval_in_seconds);
        Ok(())
    }

    pub fn remove_block_timestamp_interval(&self) -> Result<bool, Error> {
        Ok(self.time_manager.remove_block_timestamp_interval())
    }

    //---------- Helpers ---------------

    async fn seal_now(&self) -> Result<CreatedBlock<Hash>, BlockProducingError> {
        let (sender, receiver) = oneshot::channel();
        let seal_command = EngineCommand::SealNewBlock {
            create_empty: true,
            finalize: true,
            parent_hash: None,
            sender: Some(sender),
        };
        self.seal_command_sender.clone().send(seal_command).await?;
        match receiver.await {
            Ok(Ok(rx)) => Ok(rx),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(e.into()),
        }
    }

    fn wake(&self) {
        self.waker.wake();
    }

    fn is_automine(&self) -> bool {
        matches!(*self.mining_mode.read(), MiningMode::AutoMining)
    }

    async fn do_evm_mine(&self, opts: Option<MineOptions>) -> Result<u64, Error> {
        let mut blocks_to_mine = 1u64;

        if let Some(opts) = opts {
            let timestamp = match opts {
                MineOptions::Timestamp(timestamp) => timestamp,
                MineOptions::Options { timestamp, blocks } => {
                    if let Some(blocks) = blocks {
                        blocks_to_mine = blocks;
                    }
                    timestamp
                }
            };
            if let Some(timestamp) = timestamp {
                // timestamp was explicitly provided to be the next timestamp
                self.time_manager
                    .set_next_block_timestamp(timestamp)
                    .map_err(|_| Error::Mining(MiningError::Timestamp))?;
            }
        }

        for _ in 0..blocks_to_mine {
            self.seal_now().await.map_err(|e| Error::Mining(e.into()))?;
        }

        Ok(blocks_to_mine)
    }
}

// --------------- MiningEngine runner
type SealCommandStream = Pin<Box<dyn FusedStream<Item = ()> + Send>>;

fn build_interval_stream(interval: u64) -> SealCommandStream {
    let interval = Duration::from_millis(interval);
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

fn build_streams_for_mode(
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
        engine.waker.register(cx.waker());
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

async fn poll_and_seal(engine: Arc<MiningEngine>, stream: Option<&mut SealCommandStream>) -> bool {
    if next_or_pending(stream).await.is_some() {
        match engine.seal_now().await {
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
    }
    true
}

pub async fn run_mining_engine(engine: Arc<MiningEngine>) {
    let mut current_mode = None;
    let mut interval_mining_stream: Option<SealCommandStream> = None;
    let mut auto_mining_stream: Option<SealCommandStream> = None;

    loop {
        tokio::select! {
            new_mode = wait_for_mode_change(&engine, current_mode) => {
                current_mode = Some(new_mode);
                let (interval_stream, auto_stream) = build_streams_for_mode(new_mode, &engine);
                interval_mining_stream = interval_stream;
                auto_mining_stream = auto_stream;
            }
            // Poll the interval stream if it exists
            continue_loop = poll_and_seal(engine.clone(), interval_mining_stream.as_mut()) => {
                if !continue_loop {
                    break;
                }
            }
            // Poll the automining stream if it exists
            continue_loop = poll_and_seal(engine.clone(), auto_mining_stream.as_mut()) => {
                if !continue_loop {
                    break;
                }
            }

        }
    }
}
