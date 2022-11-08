use std::sync::Arc;

use avalanche_types::subnet;
use crossbeam_channel::TryRecvError;
use tokio::sync::RwLock;

use crate::vm;

#[derive(Clone)]
pub struct Builder {
    vm_inner: Arc<RwLock<vm::inner::Inner>>,
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
        Self { vm_inner }
    }

    /// Signal the consensus engine to build a block from pending transactions.
    async fn signal_txs_ready(&mut self) {
        log::info!("sending pending txs to consensus engine");
        let mut vm = self.vm_inner.write().await;
        if vm.block_status == Status::Building {
            log::info!("block status is already building");
            return
        }
        match vm
            .to_engine
            .as_ref()
            .expect("builder.vm_inner")
            .send(subnet::rpc::common::message::Message::PendingTxs)
            .await
        {
            Ok(_) => {
                log::info!("sending pending txs: complete");
                vm.block_status = Status::Building;
            }
            Err(e) => {
                log::error!("dropping message to consensus engine: {}", e)
            }
        }
    }

    // Helper function initialize builder
    pub async fn init(
        &self,
    ) -> (
        crossbeam_channel::Receiver<()>,
        crossbeam_channel::Receiver<()>,
    ) {
        let vm = self.vm_inner.read().await;
        (vm.stop_rx.clone(), vm.mempool.subscribe_pending())
    }

    /// Ensures that new transactions passed to mempool are
    /// considered for the next block.
    pub async fn build(&mut self) {
        log::info!("starting build loops");

        let (stop_ch, mempool_pending_ch) = self.init().await;

        while stop_ch.try_recv() == Err(TryRecvError::Empty) {
            mempool_pending_ch.recv().unwrap();
            log::info!("build: pending mempool signal received");
            self.signal_txs_ready().await;
        }
    }
}
