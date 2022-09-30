use std::{
    borrow::BorrowMut,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use avalanche_types::rpcchainvm;
use chan::chan_select;
use crossbeam_channel::TryRecvError;
use tokio::{
    sync::{mpsc, RwLock},
    time::sleep,
};

use crate::{mempool, network};

// TODO: make configurable
const GOSSIP_INTERVAL: Duration = Duration::from_secs(1);
const REGOSSIP_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct Timed {
    /// status signals the phase of block building the Vm is currently in.
    /// [DontBuild] indicates there's no need to build a block.
    /// [MayBuild] indicates the Vm should proceed to build a block.
    /// [Building] indicates the Vm has sent a request to the engine to build a block.
    pub status: Arc<RwLock<Status>>,

    /// Build timer.
    pub build_block_timer: Timer,

    /// Interval duration used to build a block.
    pub build_interval: Duration,

    // cloned from vm
    pub vm_mempool: Arc<RwLock<mempool::Mempool>>,
    pub vm_network: Option<Arc<RwLock<network::Push>>>,
    pub vm_engine_tx: mpsc::Sender<rpcchainvm::common::message::Message>,
    pub vm_stop_rx: crossbeam_channel::Receiver<()>,
    pub vm_builder_stop_rx: crossbeam_channel::Receiver<()>,
    pub mempool_pending_ch: crossbeam_channel::Receiver<()>,
}

#[derive(PartialEq)]
pub enum Status {
    /// Indicates there's no need to build a block.
    DontBuild,

    /// Indicates the Vm should proceed to build a block.
    MayBuild,

    /// Indicates the Vm has sent a request to the engine to build a block.
    Building,
}

#[derive(Clone)]
pub struct Timer {
    /// Timeout Tx channel is used to reset ticker threads.
    timeout_tx: crossbeam_channel::Sender<()>,

    /// Timeout Rx channel listens.
    timeout_rx: crossbeam_channel::Receiver<()>,

    /// New timer creation stops when true.
    finished: Arc<AtomicBool>,

    /// Notifies the timer to invoke the callback.
    should_execute: Arc<AtomicBool>,

    /// Duration for timer tick event.
    duration: Arc<RwLock<Duration>>,
}

impl Timer {
    pub fn new() -> Self {
        let (timeout_tx, timeout_rx): (
            crossbeam_channel::Sender<()>,
            crossbeam_channel::Receiver<()>,
        ) = crossbeam_channel::bounded(1);
        Self {
            finished: Arc::new(AtomicBool::new(false)),
            should_execute: Arc::new(AtomicBool::new(false)),
            timeout_tx,
            timeout_rx,
            duration: Arc::new(RwLock::new(Duration::from_secs(1))),
        }
    }
}

/// Directs the engine when to build blocks and gossip transactions.
impl Timed {
    /// Sets the initial timeout on the two stage timer if the process
    /// has not already begun from an earlier notification. If [buildStatus] is anything
    /// other than [DontBuild], then the attempt has already begun and this notification
    /// can be safely skipped.
    async fn signal_txs_ready(&mut self) {
        if *self.status.read().await == Status::DontBuild {
            return;
        }

        self.mark_building().await
    }

    /// Signal the avalanchego engine to build a block from pending transactions
    async fn mark_building(&mut self) {
        match self
            .vm_engine_tx
            .send(rpcchainvm::common::message::Message::PendingTxs)
            .await
        {
            Ok(_) => {
                let mut status = self.status.write().await;
                *status = Status::Building;
            }
            Err(e) => log::error!("dropping message to consensus engine: {}", e.to_string()),
        }
    }

    /// Should be called immediately after [build_block].
    // [HandleGenerateBlock] invocation could lead to quiescence, building a block with
    // some delay, or attempting to build another block immediately
    pub async fn handle_generate_block(&mut self) {
        let mut status = self.status.write().await;

        if self.need_to_build().await {
            *status = Status::MayBuild;
            self.dispatch_timer_duration(self.build_interval).await;
        } else {
            *status = Status::DontBuild;
        }
    }

    // Returns true if there are outstanding transactions to be issued
    // into a block.
    async fn need_to_build(&self) -> bool {
        let mempool = self.vm_mempool.read().await;
        return mempool.len() > 0;
    }

    /// Parses the block current status and
    pub async fn build_block_parse_status(&mut self) {
        let mut mark_building = false;
        match &*self.status.read().await {
            Status::DontBuild => {
                // no op
            }
            Status::MayBuild => mark_building = true,
            Status::Building => {
                // If the status has already been set to building, there is no need
                // to send an additional request to the consensus engine until the call
                // to BuildBlock resets the block status.
            }
        }

        if mark_building {
            self.mark_building().await;
        }
    }

    /// Defines the duration until we check block status.
    async fn dispatch_timer_duration(&self, duration: Duration) {
        let mut timer = self.build_block_timer.duration.write().await;
        *timer = duration;
        self.build_block_timer
            .should_execute
            .store(true, Ordering::Relaxed);
        self.dispatch_reset();
    }

    /// Cancel the currently dispatch timer scheduled event.
    fn dispatch_cancel(&self) {
        self.build_block_timer
            .should_execute
            .store(false, Ordering::Relaxed);
        self.dispatch_reset();
    }

    /// Stops execution of the dispatch timer.
    fn dispatch_stop(&self) {
        self.build_block_timer
            .finished
            .store(true, Ordering::Relaxed);
        self.dispatch_reset();
    }

    /// Calls the timeout channel which will result in a new timer event.
    fn dispatch_reset(&self) {
        let _ = self.build_block_timer.timeout_tx.send(());
    }

    /// Manages a dispatch timer lifecycle.
    async fn dispatch_timer(&mut self) {
        let (tx, ticker_rx): (
            crossbeam_channel::Sender<()>,
            crossbeam_channel::Receiver<()>,
        ) = crossbeam_channel::bounded(1);
        let cleared = Arc::new(AtomicBool::new(false));
        let reset = Arc::new(AtomicBool::new(false));
        let mut ticker_duration = Duration::from_secs(0);

        while !self.build_block_timer.finished.load(Ordering::Relaxed) {
            // cleared is true after tick
            if cleared.load(Ordering::Relaxed)
                && self
                    .build_block_timer
                    .should_execute
                    .load(Ordering::Relaxed)
            {
                self.build_block_parse_status().await;
            }

            // start a new ticker thread which sends a single tick signal.
            let ticker_tx = tx.clone();
            if reset.load(Ordering::Relaxed) {
                tokio::spawn(async move {
                    let time = Instant::now();
                    thread::sleep(ticker_duration);
                    let _ = ticker_tx.send(()).unwrap();
                    log::debug!("Tick duration: {:?}", time.elapsed());
                });
            }

            reset.store(false, Ordering::Relaxed);
            cleared.store(false, Ordering::Relaxed);

            let timeout_ch = self.build_block_timer.timeout_rx.clone();

            loop {
                // select will block until one of the channels is received
                crossbeam_channel::select! {
                    recv(timeout_ch) -> _ => {
                        // reset timer duration
                        if self.build_block_timer.should_execute.load(Ordering::Relaxed) {
                            let duration = self.build_block_timer.duration.read().await;
                            ticker_duration = *duration;
                        }
                        reset.store(true, Ordering::Relaxed);
                        log::debug!("timeout");
                        break
                    }

                    // ticker
                    recv(ticker_rx) -> _ => {
                        cleared.store(true, Ordering::Relaxed);
                        log::debug!("tick");
                        break
                     }
                }
            }
        }
    }

    /// Ensures that new transactions passed to mempool are
    /// considered for the next block.
    pub async fn build(&mut self) {
        log::debug!("starting build loops");

        println!("tick build start");
        let mempool_pending_ch = self.mempool_pending_ch.clone();
        let stop_ch = self.vm_stop_rx.clone();

        while stop_ch.try_recv() == Err(TryRecvError::Empty) {
            println!("tick build");
            let _ = mempool_pending_ch.recv().unwrap();
            self.signal_txs_ready().await;
        }
    }

    pub async fn regossip(&self) {
        // testing only
        let maybe_network = &self.vm_network;
        if maybe_network.is_none() {
            return;
        }

        log::debug!("starting regossip loop");
        let stop_ch = self.vm_stop_rx.clone();

        while stop_ch.try_recv() == Err(TryRecvError::Empty) {
            sleep(REGOSSIP_INTERVAL).await;
            println!("tick");

            let mut network = self.vm_network.as_ref().unwrap().write().await;
            let _ = network.regossip_txs().await;
        }

        log::debug!("shutdown regossip loop");
    }

    pub async fn gossip(&self) {
        // testing only
        let maybe_network = &self.vm_network;
        if maybe_network.is_none() {
            return;
        }

        log::debug!("starting gossip loops");

        let stop_ch = self.vm_stop_rx.clone();
        let maybe_network = &self.vm_network;

        while stop_ch.try_recv() == Err(TryRecvError::Empty) {
            sleep(GOSSIP_INTERVAL).await;
            println!("tick gossip");
            let mempool = &mut self.vm_mempool.write().await;
            let new_txs = mempool.new_txs().unwrap();

            let mut network = self.vm_network.as_ref().unwrap().write().await;
            let _ = network.gossip_new_txs(new_txs).await;
        }
    }
}
