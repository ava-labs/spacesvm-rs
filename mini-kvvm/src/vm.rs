use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
    sync::Arc,
    time,
};

use avalanche_types::{
    choices::status::{self, Status},
    ids,
    rpcchainvm::{
        self,
        concensus::snowman::{Block, Initializer},
    },
};
use chrono::Utc;
use semver::Version;
use tokio::sync::{mpsc::Sender, RwLock};

use crate::{
    api,
    block::Block as StatefulBlock,
    block::{self, state::State},
    chain::{self, tx::Transaction, vm::Vm},
    genesis::Genesis,
    mempool, network,
};

const PUBLIC_API_ENDPOINT: &str = "/public";
const VERSION: &str = env!("CARGO_PKG_VERSION");

// TODO: make configurable
const MEMPOOL_SIZE: usize = 1024;

pub struct ChainVmInterior {
    pub ctx: Option<rpcchainvm::context::Context>,
    pub bootstrapped: bool,
    pub version: Version,
    pub genesis: Genesis,
    pub state: State,
    pub preferred: ids::Id,
    pub last_accepted: StatefulBlock,
    pub to_engine: Option<Sender<rpcchainvm::common::message::Message>>,
    pub preferred_block_id: Option<ids::Id>,
}

impl Default for ChainVmInterior {
    fn default() -> Self {
        Self {
            ctx: None,
            bootstrapped: false,
            version: Version::new(0, 0, 0),
            genesis: Genesis::default(),
            state: State::default(),
            preferred: ids::Id::empty(),
            last_accepted: StatefulBlock::default(),
            to_engine: None,
            preferred_block_id: None,
        }
    }
}

#[derive(Clone)]
pub struct ChainVm {
    pub db: Box<dyn rpcchainvm::database::Database + Sync + Send>,
    pub app_sender: Option<Box<dyn rpcchainvm::common::appsender::AppSender + Send + Sync>>,
    pub mempool: Arc<RwLock<mempool::Mempool>>,
    pub network: Option<Arc<RwLock<network::Push>>>,
    pub inner: Arc<RwLock<ChainVmInterior>>,
    pub verified_blocks: Arc<RwLock<HashMap<ids::Id, block::Block>>>,
}

impl ChainVm {
    /// Returns initialized ChainVm Boxed as rpcchainvm::vm::Vm trait.
    pub fn new() -> Box<dyn rpcchainvm::vm::Vm + Send + Sync> {
        let inner = Arc::new(RwLock::new(ChainVmInterior::default()));
        let db = rpcchainvm::database::memdb::Database::new();
        let mempool = mempool::Mempool::new(MEMPOOL_SIZE);
        let verified_blocks = Arc::new(RwLock::new(HashMap::new()));

        Box::new(ChainVm {
            db,
            inner,
            mempool: Arc::new(RwLock::new(mempool)),
            verified_blocks,
            app_sender: None,
            network: None,
        })
    }

    pub fn new_with_state(db: &Box<dyn rpcchainvm::database::Database + Sync + Send>) -> Self {
        let mempool = mempool::Mempool::new(MEMPOOL_SIZE);
        let verified_blocks = &Arc::new(RwLock::new(HashMap::new()));
        let inner = ChainVmInterior {
            ctx: None,
            bootstrapped: false,
            version: Version::new(0, 0, 0),
            genesis: Genesis::default(),
            state: State::new(db.clone(), Arc::clone(verified_blocks)),
            preferred: ids::Id::empty(),
            last_accepted: StatefulBlock::default(),
            to_engine: None,
            preferred_block_id: None,
        };
        Self {
            db: db.clone(),
            inner: Arc::new(RwLock::new(inner)),
            mempool: Arc::new(RwLock::new(mempool)),
            verified_blocks: Arc::clone(verified_blocks),
            app_sender: None,
            network: None,
        }
    }
}

impl avalanche_types::rpcchainvm::vm::Vm for ChainVm {}

#[tonic::async_trait]
impl crate::chain::vm::Vm for ChainVm {
    async fn is_bootstrapped(&self) -> bool {
        let vm = self.inner.read().await;
        return vm.bootstrapped;
    }

    async fn submit(&self, mut txs: Vec<chain::tx::tx::Transaction>) -> Result<()> {
        let now = Utc::now().timestamp() as u64;

        // TODO append errors instead of fail
        for tx in txs.iter_mut() {
            tx.init()
                .await
                .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
            if tx.id().await == ids::Id::empty() {
                return Err(Error::new(ErrorKind::Other, "invalid block id"));
            }
            let dummy_block = block::Block::new_dummy(now, tx.to_owned());

            tx.execute(self.db.clone(), dummy_block)
                .await
                .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

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
        let vm = self.inner.read().await;

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
        Err(Error::new(
            ErrorKind::Unsupported,
            "app request not implemented",
        ))
    }

    async fn app_request_failed(&self, _node_id: &ids::node::Id, _request_id: u32) -> Result<()> {
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
        Err(Error::new(
            ErrorKind::Unsupported,
            "app response not implemented",
        ))
    }

    async fn app_gossip(&self, _node_id: &ids::node::Id, _msg: &[u8]) -> Result<()> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "app gossip not implemented",
        ))
    }
}

#[tonic::async_trait]
impl rpcchainvm::common::vm::Connector for ChainVm {
    async fn connected(&self, _id: &ids::node::Id) -> Result<()> {
        // no-op
        Ok(())
    }

    async fn disconnected(&self, _id: &ids::node::Id) -> Result<()> {
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
        _to_engine: Sender<rpcchainvm::common::message::Message>,
        _fxs: &[rpcchainvm::common::vm::Fx],
        app_sender: Box<dyn rpcchainvm::common::appsender::AppSender + Send + Sync>,
    ) -> Result<()> {
        let mut vm = self.inner.write().await;
        vm.ctx = ctx;

        let version =
            Version::parse(VERSION).map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
        vm.version = version;

        let current = db_manager.current().await?;
        self.db = current.db.clone();

        self.app_sender = Some(app_sender);
        self.network = Some(Arc::new(RwLock::new(network::Push::new(self.clone()))));

        let verified_blocks = self.verified_blocks.clone();

        vm.state = State::new(self.db.clone(), verified_blocks);

        // Try to load last accepted
        let resp = vm.state.has_last_accepted().await;
        if resp.is_err() {
            log::error!("could not determine if have last accepted");
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
                .set_last_accepted(genesis_block)
                .await
                .map_err(|e| {
                    Error::new(ErrorKind::Other, format!("failed to accept block: {:?}", e))
                })?;
            vm.preferred = genesis_block_id;

            log::info!("initialized from genesis block: {:?}", genesis_block_id)
        }
        Ok(())
    }

    /// Called when the node is shutting down.
    async fn shutdown(&self) -> Result<()> {
        // grpc servers are shutdown via broadcast channel
        // if additional shutdown is required we can extend.
        Ok(())
    }

    /// Communicates to Vm the next state phase.
    async fn set_state(&self, snow_state: rpcchainvm::snow::State) -> Result<()> {
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
        Ok(String::from(VERSION))
    }

    /// Creates the HTTP handlers for custom Vm network calls
    /// for "ext/vm/[vmId]"
    async fn create_static_handlers(
        &self,
    ) -> std::io::Result<
        std::collections::HashMap<String, rpcchainvm::common::http_handler::HttpHandler>,
    > {
        log::debug!("create_static_handlers called");

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
        Ok(HashMap::new())
    }
}

#[tonic::async_trait]
impl rpcchainvm::snowman::block::Getter for ChainVm {
    /// Attempt to load a block.
    async fn get_block(
        &self,
        // TOOD: make this reference
        id: ids::Id,
    ) -> Result<Box<dyn rpcchainvm::concensus::snowman::Block + Send + Sync>> {
        // TODO: why do we need mutable reference?
        let mut vm = self.inner.write().await;

        let block =
            vm.state.get_block(id).await.map_err(|e| {
                Error::new(ErrorKind::Other, format!("failed to get block: {:?}", e))
            })?;

        Ok(Box::new(block))
    }
}

#[tonic::async_trait]
impl rpcchainvm::snowman::block::Parser for ChainVm {
    /// Attempt to create a block from a stream of bytes.
    async fn parse_block(
        &self,
        // TOOD: make this generic
        bytes: &[u8],
    ) -> Result<Box<dyn rpcchainvm::concensus::snowman::Block + Send + Sync>> {
        let mut vm = self.inner.write().await;

        let new_block = vm
            .state
            .parse_block(bytes.to_vec(), Status::Processing)
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

        let mut txs = Vec::new();

        loop {
            match mempool.pop_back() {
                Some(tx) => {
                    log::debug!("writing tx{:?}\n", tx);
                    // verify
                    tx.execute(self.db.clone(), block.clone())
                        .await
                        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
                    txs.push(tx)
                }
                _ => break,
            }
        }

        block.txs = txs;

        // initialize block
        let bytes = block.to_bytes().await;
        block
            .init(&bytes.unwrap(), status::Status::Processing)
            .await
            .unwrap();

        // verify
        block
            .verify()
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

        self.notify_block_ready().await;

        Ok(Box::new(block))
    }

    /// Notify the Vm of the currently preferred block.
    async fn set_preference(&self, id: ids::Id) -> Result<()> {
        let mut vm = self.inner.write().await;
        vm.preferred_block_id = Some(id);

        Ok(())
    }

    // Returns the Id of the last accepted block.
    async fn last_accepted(&self) -> Result<ids::Id> {
        let vm = self.inner.write().await;
        let state = vm.state.clone();
        let last_accepted_id = state.get_last_accepted().await?;

        Ok(last_accepted_id)
    }

    /// Attempts to issue a transaction into consensus.
    async fn issue_tx(
        &self,
    ) -> Result<Box<dyn rpcchainvm::concensus::snowman::Block + Send + Sync>> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "issue tx not implemented",
        ))
    }
}
