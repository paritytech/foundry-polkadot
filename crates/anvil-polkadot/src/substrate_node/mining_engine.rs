use parking_lot::{Mutex, RwLock};
use polkadot_sdk::{sc_consensus_manual_seal::EngineCommand, sp_core};
use std::sync::Arc;

use super::service::TransactionPoolHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiningMode {
    /// We are only producing blocks as an answer to the
    /// mine family of RPCs
    None,
}

pub struct MiningEngine {
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
}
