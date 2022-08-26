use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
    sync::Arc,
    time,
};

use avalanche_types::{
    choices::status,
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
    block::Block as StatefulBlock,
    block::{self, state::State},
    chain::{self, tx::Transaction},
    genesis::Genesis,
};

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
    pub mempool: Arc<RwLock<Vec<chain::tx::tx::Transaction>>>,
    pub inner: Arc<RwLock<ChainVmInterior>>,
    pub verified_blocks: Arc<RwLock<HashMap<ids::Id, crate::block::Block>>>,
}

impl ChainVm {
    /// Returns initialized ChainVm Boxed as rpcchainvm::vm::Vm trait.
    pub fn new() -> Box<dyn rpcchainvm::vm::Vm + Send + Sync> {
        let inner = Arc::new(RwLock::new(ChainVmInterior::default()));
        let db = rpcchainvm::database::memdb::Database::new();
        let mempool = Arc::new(RwLock::new(Vec::new()));
        let verified_blocks = Arc::new(RwLock::new(HashMap::new()));

        Box::new(ChainVm {
            db,
            inner,
            mempool,
            verified_blocks,
        })
    }

    pub fn new_with_state(db: &Box<dyn rpcchainvm::database::Database + Sync + Send>) -> Self {
        let mempool = Arc::new(RwLock::new(Vec::new()));
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
            mempool,
            verified_blocks: Arc::clone(verified_blocks),
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

    async fn submit(&self, mut txs: Vec<crate::chain::tx::tx::Transaction>) -> Result<()> {
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
            mempool.push(tx.to_owned());
        }
        Ok(())
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
        Err(Error::new(
            ErrorKind::Unsupported,
            "connected not implemented",
        ))
    }

    async fn disconnected(&self, _id: &ids::node::Id) -> Result<()> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "disconnected not implemented",
        ))
    }
}

#[tonic::async_trait]
impl rpcchainvm::health::Checkable for ChainVm {
    async fn health_check(&self) -> Result<Vec<u8>> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "health check not implemented",
        ))
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
        _app_sender: Box<dyn rpcchainvm::common::appsender::AppSender + Send + Sync>,
    ) -> Result<()> {
        let mut vm = self.inner.write().await;
        vm.ctx = ctx;

        let current = db_manager.current().await?;
        self.db = current.db.clone();

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
        Err(Error::new(
            ErrorKind::Unsupported,
            "shutdown not implemented",
        ))
    }

    /// Communicates to Vm the next state it starts.
    async fn set_state(&self, _snow_state: rpcchainvm::snow::State) -> Result<()> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "set_state not implemented",
        ))
    }

    /// Returns the version of the VM this node is running.
    async fn version(&self) -> Result<String> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "version not implemented",
        ))
    }

    /// Creates the HTTP handlers for custom Vm network calls.
    async fn create_static_handlers(
        &self,
    ) -> std::io::Result<
        std::collections::HashMap<
            String,
            avalanche_types::rpcchainvm::common::http_handler::HttpHandler,
        >,
    > {
        Err(Error::new(
            ErrorKind::Unsupported,
            "create_static_handlers not implemented",
        ))
    }

    /// Creates the HTTP handlers for custom chain network calls.
    async fn create_handlers(
        &self,
    ) -> std::io::Result<
        std::collections::HashMap<
            String,
            avalanche_types::rpcchainvm::common::http_handler::HttpHandler,
        >,
    > {
        Err(Error::new(
            ErrorKind::Unsupported,
            "create_handlers not implemented",
        ))
    }
}

#[tonic::async_trait]
impl rpcchainvm::snowman::block::Getter for ChainVm {
    /// Attempt to load a block.
    async fn get_block(
        &self,
        _id: ids::Id,
    ) -> Result<Box<dyn rpcchainvm::concensus::snowman::Block + Send + Sync>> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "get_block not implemented",
        ))
    }
}

#[tonic::async_trait]
impl rpcchainvm::snowman::block::Parser for ChainVm {
    /// Attempt to create a block from a stream of bytes.
    async fn parse_block(
        &self,
        _bytes: &[u8],
    ) -> Result<Box<dyn rpcchainvm::concensus::snowman::Block + Send + Sync>> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "parse_block not implemented",
        ))
    }
}

#[tonic::async_trait]
impl rpcchainvm::snowman::block::ChainVm for ChainVm {
    /// Attempt to create a new block.
    async fn build_block(
        &self,
    ) -> Result<Box<dyn rpcchainvm::concensus::snowman::Block + Send + Sync>> {
        let mut mempool = self.mempool.write().await;
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
            match mempool.pop() {
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

        Ok(Box::new(block))
    }

    /// Notify the Vm of the currently preferred block.
    async fn set_preference(&self, _id: ids::Id) -> Result<()> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "set_preference not implemented",
        ))
    }

    // Returns the Id of the last accepted block.
    async fn last_accepted(&self) -> Result<ids::Id> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "last_accepted not implemented",
        ))
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
