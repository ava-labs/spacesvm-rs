use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
    num::NonZeroUsize,
    sync::Arc,
    time::{self, Duration},
};

use avalanche_types::{
    choices::status::{self, Status},
    ids,
    rpcchainvm::{
        self,
        concensus::snowman::{Block as SnowmanBlock, Initializer},
    },
};
use chrono::Utc;
use lru::LruCache;
use semver::Version;
use tokio::sync::{mpsc, Mutex, RwLock};

use crate::{
    api,
    block::{self, builder},
    chain::{self, storage, tx::Transaction, vm::Vm},
    genesis::Genesis,
    mempool, network,
};

const PUBLIC_API_ENDPOINT: &str = "/public";
const VERSION: &str = env!("CARGO_PKG_VERSION");

// TODO: make configurable
const MEMPOOL_SIZE: usize = 1024;
const BLOCKS_LRU_SIZE: usize = 8192;
const BUILD_INTERVAL: Duration = Duration::from_millis(500);

pub struct ChainVmInterior {
    pub ctx: Option<rpcchainvm::context::Context>,
    pub bootstrapped: bool,
    pub version: Version,
    pub genesis: Genesis,
    pub preferred: ids::Id,
    pub last_accepted: block::Block,
    pub to_engine: Option<mpsc::Sender<rpcchainvm::common::message::Message>>,
    pub state: block::state::State,

    /// The block that is currently preferred by the consensus engine.
    pub preferred_block_id: ids::Id,
}

impl Default for ChainVmInterior {
    fn default() -> Self {
        Self {
            ctx: None,
            bootstrapped: false,
            version: Version::new(0, 0, 0),
            genesis: Genesis::default(),
            preferred: ids::Id::empty(),
            last_accepted: block::Block::default(),
            to_engine: None,
            preferred_block_id: ids::Id::empty(),
            state: block::state::State::default(),
        }
    }
}

#[derive(Clone)]
pub struct ChainVm {
    pub inner: Arc<RwLock<ChainVmInterior>>,

    /// LRU cache of accepted blocks.
    // note: the use of mutex vs rwlock here is deliberate as read access
    // still requires DerefMut.
    pub accepted_blocks: Arc<Mutex<LruCache<ids::Id, block::Block>>>,
    pub mempool: Arc<RwLock<mempool::Mempool>>,

    /// Map of blocks which have been verified but pending
    /// consensus accept/reject.
    pub verified_blocks: Arc<RwLock<HashMap<ids::Id, block::Block>>>,

    pub app_sender: Option<Box<dyn rpcchainvm::common::appsender::AppSender + Send + Sync>>,

    /// Timed builder manages block creation and gossiping loops.
    pub builder: Option<block::builder::Timed>,
    pub db: Option<Box<dyn rpcchainvm::database::Database + Sync + Send>>,
    pub network: Option<Arc<RwLock<network::Push>>>,

    pub builder_stop_rx: crossbeam_channel::Receiver<()>,
    pub builder_stop_tx: crossbeam_channel::Sender<()>,
    pub done_build_rx: crossbeam_channel::Receiver<()>,
    pub done_build_tx: crossbeam_channel::Sender<()>,
    pub done_gossip_rx: crossbeam_channel::Receiver<()>,
    pub done_gossip_tx: crossbeam_channel::Sender<()>,
    pub mempool_pending_rx: crossbeam_channel::Receiver<()>,
    pub stop_rx: crossbeam_channel::Receiver<()>,
    pub stop_tx: crossbeam_channel::Sender<()>,
}

impl ChainVm {
    /// Returns initialized ChainVm Boxed as rpcchainvm::vm::Vm trait.
    pub fn new() -> Self {
        let (stop_tx, stop_rx): (
            crossbeam_channel::Sender<()>,
            crossbeam_channel::Receiver<()>,
        ) = crossbeam_channel::bounded(1);

        let (builder_stop_tx, builder_stop_rx): (
            crossbeam_channel::Sender<()>,
            crossbeam_channel::Receiver<()>,
        ) = crossbeam_channel::bounded(1);

        let (done_build_tx, done_build_rx): (
            crossbeam_channel::Sender<()>,
            crossbeam_channel::Receiver<()>,
        ) = crossbeam_channel::bounded(1);

        let (done_gossip_tx, done_gossip_rx): (
            crossbeam_channel::Sender<()>,
            crossbeam_channel::Receiver<()>,
        ) = crossbeam_channel::bounded(1);

        let mempool = mempool::Mempool::new(MEMPOOL_SIZE);
        let mempool_pending_rx = mempool.subscribe_pending();

        Self {
            inner: Arc::new(RwLock::new(ChainVmInterior::default())),

            accepted_blocks: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(BLOCKS_LRU_SIZE).unwrap(),
            ))),
            mempool: Arc::new(RwLock::new(mempool)),
            verified_blocks: Arc::new(RwLock::new(HashMap::new())),

            app_sender: None,
            db: None,
            network: None,
            builder: None,

            builder_stop_rx,
            builder_stop_tx,
            done_build_rx,
            done_build_tx,
            mempool_pending_rx,
            done_gossip_rx,
            done_gossip_tx,
            stop_rx,
            stop_tx,
        }
    }
}

impl avalanche_types::rpcchainvm::vm::Vm for ChainVm {}

#[tonic::async_trait]
impl crate::chain::vm::Vm for ChainVm {
    async fn is_bootstrapped(&self) -> bool {
        log::debug!("vm::is_bootstrapped called");

        let vm = self.inner.read().await;
        return vm.bootstrapped;
    }

    async fn submit(&self, mut txs: Vec<chain::tx::tx::Transaction>) -> Result<()> {
        log::debug!("vm::submit called");

        storage::submit(&self.db.as_ref().unwrap().clone(), &mut txs)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

        for tx in txs.iter_mut() {
            let mut mempool = self.mempool.write().await;
            let _ = mempool
                .add(tx.to_owned())
                .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
        }
        Ok(())
    }

    /// Sends a signal to the consensus engine that a new block
    /// is ready to be created.
    async fn notify_block_ready(&self) {
        log::info!("vm::notify_block_ready called");

        let vm = self.inner.write().await;

        if let Some(engine) = &vm.to_engine {
            if let Err(_) = engine
                .send(rpcchainvm::common::message::Message::PendingTxs)
                .await
            {
                log::warn!("dropping message to consensus engine");
            };
            return;
        }

        log::error!("consensus engine channel failed to initialized");
        return;
    }
}

#[tonic::async_trait]
impl rpcchainvm::common::apphandler::AppHandler for ChainVm {
    async fn app_request(
        &self,
        _node_id: &ids::node::Id,
        _request_id: u32,
        _deadline: time::Instant,
        _request: &[u8],
    ) -> Result<()> {
        log::debug!("vm::app_request called");

        Err(Error::new(
            ErrorKind::Unsupported,
            "app request not implemented",
        ))
    }

    async fn app_request_failed(&self, _node_id: &ids::node::Id, _request_id: u32) -> Result<()> {
        log::debug!("vm::app_request_failed called");

        Err(Error::new(
            ErrorKind::Unsupported,
            "app request failed not implemented",
        ))
    }

    async fn app_response(
        &self,
        _node_id: &ids::node::Id,
        _request_id: u32,
        _response: &[u8],
    ) -> Result<()> {
        log::debug!("vm::app_response called");

        Err(Error::new(
            ErrorKind::Unsupported,
            "app response not implemented",
        ))
    }

    async fn app_gossip(&self, _node_id: &ids::node::Id, _msg: &[u8]) -> Result<()> {
        log::debug!("vm::app_gossip called");

        Err(Error::new(
            ErrorKind::Unsupported,
            "app gossip not implemented",
        ))
    }
}

#[tonic::async_trait]
impl rpcchainvm::common::vm::Connector for ChainVm {
    async fn connected(&self, _id: &ids::node::Id) -> Result<()> {
        log::debug!("vm::connected called");

        // no-op
        Ok(())
    }

    async fn disconnected(&self, _id: &ids::node::Id) -> Result<()> {
        log::debug!("vm::disconnected called");

        // no-op
        Ok(())
    }
}

#[tonic::async_trait]
impl rpcchainvm::health::Checkable for ChainVm {
    async fn health_check(&self) -> Result<Vec<u8>> {
        Ok("200".as_bytes().to_vec())
    }
}

#[tonic::async_trait]
impl rpcchainvm::common::vm::Vm for ChainVm {
    /// Initialize this Vm.
    async fn initialize(
        &mut self,
        ctx: Option<rpcchainvm::context::Context>,
        db_manager: Box<dyn rpcchainvm::database::manager::Manager + Send + Sync>,
        genesis_bytes: &[u8],
        _upgrade_bytes: &[u8],
        _config_bytes: &[u8],
        to_engine: mpsc::Sender<rpcchainvm::common::message::Message>,
        _fxs: &[rpcchainvm::common::vm::Fx],
        app_sender: Box<dyn rpcchainvm::common::appsender::AppSender + Send + Sync>,
    ) -> Result<()> {
        log::debug!("vm::initialize called");

        let mut vm = self.inner.write().await;
        vm.ctx = ctx;
        vm.to_engine = Some(to_engine);

        let version =
            Version::parse(VERSION).map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
        vm.version = version;

        let current = db_manager.current().await?;
        self.db = Some(current.db.clone());
        vm.state = block::state::State::new(current.db.clone());

        self.app_sender = Some(app_sender);

        self.network = Some(Arc::new(RwLock::new(network::Push::new(
            self.app_sender.as_ref().unwrap().clone(),
            self.db.as_ref().unwrap().clone(),
            self.mempool.clone(),
        ))));

        // builder must be initialized network
        if self.builder.is_none() {
            self.builder = Some(block::builder::Timed {
                mempool_pending_ch: self.mempool_pending_rx.clone(),
                vm_mempool: self.mempool.clone(),
                vm_network: Some(self.network.as_ref().unwrap().clone()),
                vm_engine_tx: vm.to_engine.as_ref().unwrap().clone(),
                vm_builder_stop_rx: self.builder_stop_rx.clone(),
                vm_stop_rx: self.stop_rx.clone(),
                build_block_timer: block::builder::Timer::new(),
                build_interval: BUILD_INTERVAL,
                status: Arc::new(RwLock::new(block::builder::Status::DontBuild)),
            });
        }

        // Try to load last accepted
        let resp = vm.state.has_last_accepted().await;
        if resp.is_err() {
            log::error!("could not determine last accepted block");
            return Err(Error::new(ErrorKind::Other, resp.unwrap_err()));
        }
        let has = resp.unwrap();

        // Parse genesis data
        let genesis = Genesis::from_json(genesis_bytes)?;
        vm.genesis = genesis;

        // Check if last accepted block exists
        if has {
            let block_id = vm
                .state
                .get_last_accepted()
                .await
                .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

            let block = vm
                .state
                .get_block(block_id)
                .await
                .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

            vm.preferred = block_id;
            vm.last_accepted = block;
            log::info!("initialized vm from last accepted block id: {:?}", block_id)
        } else {
            let mut genesis_block =
                crate::block::Block::new(ids::Id::empty(), 0, genesis_bytes, 0, vm.state.clone());

            let bytes = genesis_block
                .to_bytes()
                .await
                .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

            genesis_block
                .init(&bytes, status::Status::Accepted)
                .await
                .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

            let genesis_block_id = genesis_block.id;
            vm.state
                .set_last_accepted(genesis_block.clone())
                .await
                .map_err(|e| {
                    Error::new(ErrorKind::Other, format!("failed to accept block: {:?}", e))
                })?;

            vm.last_accepted = genesis_block;
            vm.preferred = genesis_block_id;
            log::info!("initialized from genesis block: {}", genesis_block_id);
        }

        // TODO: Must be a better way :)
        let b1 = self.builder.as_ref().unwrap().to_owned();
        let mut b2 = b1.clone();
        tokio::spawn(async move {
            b1.gossip().await;
        });

        tokio::spawn(async move {
            b2.build().await;
        });

        Ok(())
    }

    /// Called when the node is shutting down.
    async fn shutdown(&self) -> Result<()> {
        log::debug!("vm::shutdown called");

        // wait for gossiper and builder to be shutdown
        self.done_build_rx.recv().unwrap();
        self.done_gossip_rx.recv().unwrap();

        // grpc servers are shutdown via broadcast channel
        if let Some(db) = self.db.clone() {
            db.close().await?;
        }
        Ok(())
    }

    /// Communicates to Vm the next state phase.
    async fn set_state(&self, snow_state: rpcchainvm::snow::State) -> Result<()> {
        log::debug!("vm::set_state called");

        let mut vm = self.inner.write().await;

        match snow_state.try_into() {
            // Initializing is called by chains manager when it is creating the chain.
            Ok(rpcchainvm::snow::State::Initializing) => {
                log::debug!("set_state: initializing");
                vm.bootstrapped = false;
                Ok(())
            }
            Ok(rpcchainvm::snow::State::StateSyncing) => {
                log::debug!("set_state: state syncing");
                Err(Error::new(ErrorKind::Other, "state sync is not supported"))
            }
            // Bootstrapping is called by the bootstrapper to signal bootstrapping has started.
            Ok(rpcchainvm::snow::State::Bootstrapping) => {
                log::debug!("set_state: bootstrapping");
                vm.bootstrapped = false;
                Ok(())
            }
            // NormalOp os called when consensus has started signalling bootstrap phase is complete.
            Ok(rpcchainvm::snow::State::NormalOp) => {
                log::debug!("set_state: normal op");
                vm.bootstrapped = true;
                Ok(())
            }
            Err(_) => Err(Error::new(ErrorKind::Other, "unknown state")),
        }
    }

    /// Returns the version of the VM this node is running.
    async fn version(&self) -> Result<String> {
        log::debug!("vm::shutdown called");

        Ok(String::from(VERSION))
    }

    /// Creates the HTTP handlers for custom Vm network calls
    /// for "ext/vm/[vmId]"
    async fn create_static_handlers(
        &self,
    ) -> std::io::Result<
        std::collections::HashMap<String, rpcchainvm::common::http_handler::HttpHandler>,
    > {
        log::debug!("vm::create_static_handlers called");

        // Initialize the jsonrpc public service and handler
        let service = api::service::Service::new(self.clone());
        let mut handler = jsonrpc_core::IoHandler::new();
        handler.extend_with(api::Service::to_delegate(service));

        let http_handler = rpcchainvm::common::http_handler::HttpHandler::new_from_u8(0, handler)
            .map_err(|_| Error::from(ErrorKind::InvalidData))?;

        let mut handlers = HashMap::new();
        handlers.insert(String::from(PUBLIC_API_ENDPOINT), http_handler);
        Ok(handlers)
    }

    /// Creates the HTTP handlers for custom chain network calls
    /// for "ext/vm/[chainId]"
    async fn create_handlers(
        &self,
    ) -> std::io::Result<
        std::collections::HashMap<
            String,
            avalanche_types::rpcchainvm::common::http_handler::HttpHandler,
        >,
    > {
        log::debug!("vm::create_handlers called");

        Ok(HashMap::new())
    }
}

#[tonic::async_trait]
impl rpcchainvm::snowman::block::Getter for ChainVm {
    /// Attempt to load a block.
    async fn get_block(
        &self,
        id: ids::Id,
    ) -> Result<Box<dyn rpcchainvm::concensus::snowman::Block + Send + Sync>> {
        log::debug!("vm::get_block called");

        let accepted_blocks = &mut self.accepted_blocks.lock().await;
        // has block been accepted by the vm and cached.
        if let Some(cached) = accepted_blocks.get(&id) {
            return Ok(Box::new(cached.to_owned()));
        }

        // has block been verified, but not yet accepted
        let verified_blocks = self.verified_blocks.read().await;
        if let Some(block) = verified_blocks.get(&id) {
            return Ok(Box::new(block.to_owned()));
        }

        // check on disk state
        let mut vm = self.inner.write().await;
        let block =
            vm.state.get_block(id).await.map_err(|e| {
                Error::new(ErrorKind::Other, format!("failed to get block: {:?}", e))
            })?;

        // If block on disk, it must've been accepted
        let block = vm
            .state
            .parse_block(Some(block.to_owned()), vec![], Status::Accepted)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, format!("failed to get block: {:?}", e)))?;

        Ok(Box::new(block))
    }
}

#[tonic::async_trait]
impl rpcchainvm::snowman::block::Parser for ChainVm {
    /// Attempt to create a block from a stream of bytes.
    async fn parse_block(
        &self,
        bytes: &[u8],
    ) -> Result<Box<dyn rpcchainvm::concensus::snowman::Block + Send + Sync>> {
        log::debug!("vm::get_block called");

        let mut vm = self.inner.write().await;
        let new_block = vm
            .state
            .parse_block(None, bytes.to_vec(), Status::Processing)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, format!("failed to parse block: {:?}", e)))?;

        log::debug!("parsed block id: {}", new_block.id);

        match vm.state.get_block(new_block.id).await {
            Ok(old_block) => {
                log::debug!("returning previously parsed block id: {}", old_block.id);
                return Ok(Box::new(old_block));
            }
            Err(_) => return Ok(Box::new(new_block)),
        };
    }
}

#[tonic::async_trait]
impl rpcchainvm::snowman::block::ChainVm for ChainVm {
    /// Attempt to create a new block.
    async fn build_block(
        &self,
    ) -> Result<Box<dyn rpcchainvm::concensus::snowman::Block + Send + Sync>> {
        log::info!("vm::build_block called");

        let mempool = self.mempool.read().await;
        if mempool.len() == 0 {
            return Err(Error::new(ErrorKind::Other, "no pending blocks"));
        }

        let vm = self.inner.read().await;
        let parent = vm
            .state
            .clone()
            .get_block(vm.preferred)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

        let next_time = Utc::now().timestamp() as u64;

        // new block
        let mut block = crate::block::Block::new(
            parent.id,
            parent.height + 1,
            &[],
            next_time,
            vm.state.clone(),
        );

        let txs = Vec::new();
        loop {
            match mempool.pop_back() {
                Some(tx) => {
                    log::debug!("writing tx{:?}\n", tx);
                    // verify
                    if let Some(db) = self.db.clone() {
                        tx.execute(db, block.clone())
                            .await
                            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
                    }
                }
                _ => break,
            }
        }

        block.txs = txs;

        // Compute block hash and marshaled representation
        let bytes = block.to_bytes().await;
        block
            .init(&bytes.unwrap(), status::Status::Processing)
            .await
            .unwrap();

        // Verify block to ensure it is formed correctly (don't save) <- TODO
        block
            .verify()
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

        let mut builder = self.builder.as_ref().unwrap().to_owned();
        builder.handle_generate_block().await;

        self.notify_block_ready().await;

        Ok(Box::new(block))
    }

    /// Notify the Vm of the currently preferred block.
    async fn set_preference(&self, id: ids::Id) -> Result<()> {
        log::debug!("vm::set_preference called");

        let mut vm = self.inner.write().await;
        vm.preferred_block_id = id;

        Ok(())
    }

    // Returns the Id of the last accepted block.
    async fn last_accepted(&self) -> Result<ids::Id> {
        log::info!("vm::last_accepted called");

        let vm = self.inner.read().await;
        Ok(vm.last_accepted.id)
    }

    /// Attempts to issue a transaction into consensus.
    async fn issue_tx(
        &self,
    ) -> Result<Box<dyn rpcchainvm::concensus::snowman::Block + Send + Sync>> {
        log::debug!("vm::issue_tx called");

        Err(Error::new(
            ErrorKind::Unsupported,
            "issue tx not implemented",
        ))
    }
}
