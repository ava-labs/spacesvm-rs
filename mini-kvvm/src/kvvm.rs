use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
    sync::Arc,
    time,
};

use avalanche_types::{ids, rpcchainvm};
use semver::Version;
use tokio::sync::{mpsc::Sender, RwLock};

use crate::genesis::Genesis;

#[derive(Debug, Default)]
pub struct State;

#[derive(Debug, Default)]
pub struct Block;

pub struct ChainVmInterior {
    pub ctx: Option<rpcchainvm::context::Context>,
    pub bootstrapped: bool,
    pub version: Version,
    pub genesis: Genesis,
    pub db_manager: Box<dyn rpcchainvm::database::manager::Manager + Send + Sync>,
    pub state: State,
    pub preferred: ids::Id,
    pub mempool: Vec<Vec<u8>>,
    pub verified_blocks:
        Arc<RwLock<HashMap<ids::Id, Box<dyn rpcchainvm::concensus::snowman::Block + Sync + Send>>>>,
    pub last_accepted: Block,
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
            db_manager: rpcchainvm::database::manager::DatabaseManager::new_from_databases(
                Vec::new(),
            ),
            state: State::default(),
            preferred: ids::Id::empty(),
            mempool: Vec::new(),
            verified_blocks: Arc::new(RwLock::new(HashMap::new())),
            last_accepted: Block::default(),
            to_engine: None,
            preferred_block_id: None,
        }
    }
}

#[derive(Clone)]
pub struct ChainVm {
    pub inner: Arc<RwLock<ChainVmInterior>>,
}

impl ChainVm {
    pub fn new() -> Box<dyn rpcchainvm::vm::Vm + Send + Sync> {
        let inner = Arc::new(RwLock::new(ChainVmInterior::default()));
        Box::new(ChainVm { inner })
    }
}

impl rpcchainvm::vm::Vm for ChainVm {}

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
        &self,
        _ctx: Option<rpcchainvm::context::Context>,
        _db_manager: Box<dyn rpcchainvm::database::manager::Manager + Send + Sync>,
        _genesis_bytes: &[u8],
        _upgrade_bytes: &[u8],
        _config_bytes: &[u8],
        _to_engine: Sender<rpcchainvm::common::message::Message>,
        _fxs: &[rpcchainvm::common::vm::Fx],
        _app_sender: Box<dyn rpcchainvm::common::appsender::AppSender + Send + Sync>,
    ) -> Result<()> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "initialize not implemented",
        ))
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
        Err(Error::new(
            ErrorKind::Unsupported,
            "build_block not implemented",
        ))
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
