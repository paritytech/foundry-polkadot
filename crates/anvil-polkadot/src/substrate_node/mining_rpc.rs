use super::mining_engine::{MiningEngine, MiningMode};
use alloy_primitives::U256;
use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use std::sync::Arc;

#[rpc(client, server)]
pub trait AnvilPolkadotMiningRpc {
    // --- Mining Control ---
    #[method(name = "getAutomine")]
    async fn get_auto_mine(&self) -> RpcResult<bool>;
    #[method(name = "setAutomine")]
    async fn set_auto_mine(&self, enabled: bool) -> RpcResult<()>;
    #[method(name = "eth_getIntervalMining")]
    async fn get_interval_mining(&self) -> RpcResult<Option<u64>>;
    #[method(name = "eth_setIntervalMining")]
    async fn set_interval_mining(&self, interval: u64) -> RpcResult<()>;
    #[method(name = "setBlockTimestampInterval")]
    async fn set_block_timestamp_interval(&self, interval: u64) -> RpcResult<()>;
    #[method(name = "removeBlockTimestampInterval")]
    async fn remove_block_timestamp_interval(&self) -> RpcResult<()>;

    // --- Manual Mining ---
    // Corrected signature to match the original
    #[method(name = "mine")]
    async fn mine(&self, num_blocks: Option<U256>, interval: Option<U256>) -> RpcResult<()>;
    // Corrected signature
    //#[method(name = "evm_mine")]
    //async fn evm_mine(&self, options: Option<Params<Option<MineOptions>>>) -> RpcResult<()>;
    //// Corrected signature
    //#[method(name = "evm_mine_detailed")]
    //async fn evm_mine_detailed(&self, options: Option<Params<Option<MineOptions>>>) ->
    // RpcResult<()>;

    // --- Timestamp and Time ---
    #[method(name = "evm_increaseTime")]
    async fn increase_time(&self, increase_by_secs: U256) -> RpcResult<U256>;
    #[method(name = "evm_setNextBlockTimestamp")]
    async fn set_next_block_timestamp(&self, timestamp: U256) -> RpcResult<()>;
    #[method(name = "evm_setTime")]
    async fn set_time(&self, timestamp: U256) -> RpcResult<U256>;
}

pub struct RpcApiServer {
    mining_engine: Arc<MiningEngine>,
}

impl RpcApiServer {
    pub fn new(mining_engine: Arc<MiningEngine>) -> Self {
        Self { mining_engine }
    }
}

#[async_trait::async_trait]
impl AnvilPolkadotMiningRpcServer for RpcApiServer {
    async fn get_auto_mine(&self) -> RpcResult<bool> {
        Ok(self.mining_engine.is_automine())
    }

    async fn set_auto_mine(&self, enabled: bool) -> RpcResult<()> {
        if self.mining_engine.is_automine() {
            if enabled {
                return Ok(());
            }
            *self.mining_engine.mining_mode.write() = MiningMode::None;
            self.mining_engine.wake();
        } else if enabled{
            *self.mining_engine.mining_mode.write() = MiningMode::AutoMining;
            self.mining_engine.wake();
        }
        Ok(())
    }

    async fn get_interval_mining(&self) -> RpcResult<Option<u64>> {
        let mode = self.mining_engine.mining_mode.read();
        if let MiningMode::Interval { tick: interval } = *mode {
            return Ok(Some(interval))
        }
        Ok(None)
    }

    async fn set_interval_mining(&self, interval: u64) -> RpcResult<()> {
        let new_mode =
            if interval <= 0 { MiningMode::None } else { MiningMode::Interval { tick: interval } };
        *self.mining_engine.mining_mode.write() = new_mode;
        self.mining_engine.wake();
        Ok(())
    }

    async fn set_block_timestamp_interval(&self, _interval: u64) -> RpcResult<()> {
        todo!()
    }

    async fn remove_block_timestamp_interval(&self) -> RpcResult<()> {
        todo!()
    }

    async fn mine(&self, num_blocks: Option<U256>, interval: Option<U256>) -> RpcResult<()> {
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
            self.mining_engine.seal_now();
        }
        Ok(())
    }

    async fn increase_time(&self, _increase_by_secs: U256) -> RpcResult<U256> {
        todo!()
    }

    async fn set_next_block_timestamp(&self, _timestamp: U256) -> RpcResult<()> {
        todo!()
    }

    async fn set_time(&self, _timestamp: U256) -> RpcResult<U256> {
        todo!()
    }
}
