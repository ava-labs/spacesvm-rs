use std::sync::Arc;

use avalanche_types::subnet;
use crossbeam_channel::TryRecvError;
use tokio::sync::{broadcast, RwLock};

use crate::vm;

// #[derive(Clone)]
pub struct Builder {
    vm_inner: Arc<RwLock<vm::inner::Inner>>,
    status: Status,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    /// Indicates the Vm should proceed to build a block.
    MayBuild,

    /// Indicates the Vm has sent a request to the engine to build a block.
    Building,
}

/// Directs the engine when to build blocks and gossip transactions.
impl Builder {
    pub fn new(vm_inner: Arc<RwLock<vm::inner::Inner>>) -> Self {
        Self {
            vm_inner,
            status: Status::MayBuild,
        }
    }

    /// Signal the consensus engine to build a block from pending transactions.
    async fn signal_txs_ready(&self) {
        log::info!("sending pending txs to consensus engine");
        let inner = self.vm_inner.read().await;
        // if inner.block_status == Status::Building {
        //     log::info!("block status is already building");
        //     return;
        // }

        if let Some(engine) = &inner.to_engine {
            engine
                .send(subnet::rpc::common::message::Message::PendingTxs)
                .await
                .unwrap();
            log::info!("sent to engine!!!!!");
        }
    }

    pub async fn set_status(&self, status: Status) {
        let mut vm = self.vm_inner.write().await;
        vm.block_status = status;
    }

    // Helper function initialize builder
    pub async fn init(&self) -> (crossbeam_channel::Receiver<()>, broadcast::Receiver<()>) {
        let vm = self.vm_inner.read().await;
        (vm.stop_rx.clone(), vm.mempool.subscribe_pending())
    }

    /// Ensures that new transactions passed to mempool are
    /// considered for the next block.
    pub async fn build(&self) {
        log::info!("starting build loops");

        let (stop_ch, mut mempool_pending_ch) = self.init().await;

        while stop_ch.try_recv() == Err(TryRecvError::Empty) {
            let _ = mempool_pending_ch.recv().await;
            log::info!("build: pending mempool signal received");
            self.signal_txs_ready().await;
        }
    }
}
