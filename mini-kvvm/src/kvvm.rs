#![allow(dead_code)]

use std::{
    collections::HashMap,
    convert::TryInto,
    io::{Error, ErrorKind, Result},
    sync::Arc,
    time,
};

use avalanche_types::{
    choices::status::Status,
    ids,
    rpcchainvm::{
        self,
        concensus::snowman::Block as BlockTrait,
        database::manager::{DatabaseManager, Manager},
    },
};
use chrono::{DateTime, NaiveDateTime, Utc};
use semver::Version;
use tokio::sync::{mpsc::Sender, RwLock};

use crate::block::{Block, MiniKvvmBlock};
use crate::genesis::Genesis;
use crate::state::State;

pub struct ChainVmInterior {
    pub ctx: Option<rpcchainvm::context::Context>,
    pub bootstrapped: bool,
    pub version: Version,
    pub genesis: Genesis,
    pub db_manager: Box<dyn Manager + Send + Sync>,
    pub state: State,
    pub preferred: ids::Id,
    pub mempool: Vec<Vec<u8>>,
    pub verified_blocks: HashMap<ids::Id, Box<dyn BlockTrait>>,
    pub last_accepted: Block,
    pub to_engine: Option<Sender<avalanche_types::rpcchainvm::common::message::Message>>,
    preferred_block_id: Option<ids::Id>,
}

impl ChainVmInterior {
    pub fn new(
        ctx: Option<rpcchainvm::context::Context>,
        bootstrapped: bool,
        version: Version,
        genesis: Genesis,
        db_manager: Box<dyn Manager + Send + Sync>,
        state: State,
        preferred: ids::Id,
        mempool: Vec<Vec<u8>>,
        verified_blocks: HashMap<ids::Id, Box<dyn BlockTrait>>,
        last_accepted: Block,
        to_engine: Option<Sender<avalanche_types::rpcchainvm::common::message::Message>>,
        preferred_block_id: Option<ids::Id>,
    ) -> Self {
        Self {
            ctx,
            bootstrapped,
            version,
            genesis,
            db_manager,
            state,
            preferred,
            mempool,
            verified_blocks,
            last_accepted,
            to_engine,
            preferred_block_id,
        }
    }
}

impl Default for ChainVmInterior {
    fn default() -> Self {
        Self {
            ctx: None,
            bootstrapped: false,
            version: Version::new(0, 0, 1),
            genesis: Genesis::default(),
            db_manager: DatabaseManager::new_from_databases(Vec::new()),
            state: State::new(None),
            preferred: ids::Id::empty(),
            mempool: Vec::new(),
            verified_blocks: HashMap::new(),
            last_accepted: Block::default(),
            to_engine: None,
            preferred_block_id: None,
        }
    }
}

// Wrapper around ChainVmInterior, allowing for easier access with [Arc<RwLock<>>] access
#[derive(Clone)]
pub struct ChainVm {
    pub inner: Arc<RwLock<ChainVmInterior>>,
}

impl ChainVm {
    pub fn new(inner: Arc<RwLock<ChainVmInterior>>) -> Box<dyn rpcchainvm::vm::Vm + Send + Sync> {
        Box::new(ChainVm { inner })
    }
}

impl Default for ChainVm {
    fn default() -> Self {
        ChainVm {
            inner: Arc::new(RwLock::new(ChainVmInterior::default())),
        }
    }
}

// This VM doesn't (currently) have any app-specific messages
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

// This VM doesn't implement Connector these methods are noop.
#[tonic::async_trait]
impl rpcchainvm::common::vm::Connector for ChainVm {
    async fn connected(&self, _id: &ids::node::Id) -> Result<()> {
        log::info!("connected called");
        Ok(())
    }
    async fn disconnected(&self, _id: &ids::node::Id) -> Result<()> {
        log::info!("disconnected called");
        Ok(())
    }
}

#[tonic::async_trait]
impl rpcchainvm::health::Checkable for ChainVm {
    async fn health_check(&self) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }
}

#[tonic::async_trait]
impl rpcchainvm::common::vm::Vm for ChainVm {
    async fn initialize(
        &self,
        ctx: Option<rpcchainvm::context::Context>,
        db_manager: Box<dyn rpcchainvm::database::manager::Manager + Send + Sync>,
        genesis_bytes: &[u8],
        _upgrade_bytes: &[u8],
        _config_bytes: &[u8],
        to_engine: Sender<rpcchainvm::common::message::Message>,
        _fxs: &[rpcchainvm::common::vm::Fx],
        _app_sender: Box<dyn rpcchainvm::common::appsender::AppSender + Send + Sync>,
    ) -> Result<()> {
        let mut vm = self.inner.write().await;

        vm.ctx = ctx;
        vm.db_manager = db_manager;
        vm.to_engine = Some(to_engine);

        let current_db = vm.db_manager.current().await.map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to verify genesis: {:?}", e),
            )
        })?;

        let state = State::new(Some(current_db.clone()));
        vm.state = state;

        let genesis = Genesis::default(); //NOTE changed to have default genesis for testing
        vm.genesis = genesis;
        vm.genesis.verify().map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to verify genesis: {:?}", e),
            )
        })?;

        // Check if last accepted block exists
        if vm.state.has_last_accepted_block().await? {
            let maybe_last_accepted_block_id = vm.state.get_last_accepted_block_id().await?;
            if maybe_last_accepted_block_id.is_none() {
                return Err(Error::new(ErrorKind::Other, "invalid block no id found"));
            }
            let last_accepted_block_id = maybe_last_accepted_block_id.unwrap();

            let maybe_last_accepted_block = vm.state.get_block(last_accepted_block_id).await?;
            if maybe_last_accepted_block.is_none() {
                return Err(Error::new(ErrorKind::Other, "invalid block no id found"));
            }
            let last_accepted_block = maybe_last_accepted_block.unwrap();

            vm.preferred = last_accepted_block_id;
            vm.last_accepted = last_accepted_block;

            log::info!(
                "initialized from last accepted block {}",
                last_accepted_block_id
            );
        } else {
            let genesis_block_vec = genesis_bytes.to_vec();
            let genesis_block_bytes = genesis_block_vec.try_into().unwrap();

            let mut genesis_block = Block::new(
                ids::Id::empty(),
                0,
                genesis_block_bytes,
                DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc),
                Status::Processing,
            );

            let genesis_block_id = genesis_block.initialize(self.clone())?;

            let accepted_block_id = vm
                .state
                .accept_block(genesis_block, self.clone())
                .await
                .map_err(|e| {
                    Error::new(ErrorKind::Other, format!("failed to accept block: {:?}", e))
                })?;
            // Remove accepted block now that it is accepted
            vm.verified_blocks.remove(&accepted_block_id);

            log::info!("initialized from genesis block: {:?}", genesis_block_id)
        }

        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn set_state(&self, snow_state: rpcchainvm::state::State) -> Result<()> {
        let mut vm = self.inner.write().await;
        match snow_state.try_into() {
            // Initializing is called by chains manager when it is creating the chain.
            Ok(rpcchainvm::state::State::Initializing) => {
                log::debug!("set_state: initializing");
                vm.bootstrapped = false;
                Ok(())
            }
            Ok(rpcchainvm::state::State::StateSyncing) => {
                log::debug!("set_state: state syncing");
                Err(Error::new(ErrorKind::Other, "state sync is not supported"))
            }
            // Bootstrapping is called by the bootstrapper to signal bootstrapping has started.
            Ok(rpcchainvm::state::State::Bootstrapping) => {
                log::debug!("set_state: bootstrapping");
                vm.bootstrapped = false;
                Ok(())
            }
            // NormalOp os called when consensus has started signalling bootstrap phase is complete
            Ok(rpcchainvm::state::State::NormalOp) => {
                log::debug!("set_state: normal op");
                vm.bootstrapped = true;
                Ok(())
            }
            Err(_) => Err(Error::new(ErrorKind::Other, "failed to accept block")),
        }
    }

    /// Returns this VM's version
    async fn version(&self) -> Result<String> {
        let vm = self.inner.read().await;
        Ok(vm.version.to_string())
    }

    async fn create_static_handlers(
        &self,
    ) -> std::io::Result<
        std::collections::HashMap<
            String,
            avalanche_types::rpcchainvm::common::http_handler::HttpHandler,
        >,
    > {
        use super::static_service::{
            StaticService, StaticServiceImpl, STATICSERVICE_PUBLICENDPOINT,
        };
        log::debug!("create_static_handlers called");

        // make a new jsonrpc service with this vm as a reference
        let mut io = jsonrpc_core::IoHandler::new();
        let service = StaticServiceImpl { vm: self.clone() };

        // Allow [io] to handle methods defined in service.rs
        io.extend_with(service.to_delegate());
        let http_handler = rpcchainvm::common::http_handler::HttpHandler::new_from_u8(0, io)
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?;

        let mut handlers = std::collections::HashMap::new();
        handlers.insert(String::from(STATICSERVICE_PUBLICENDPOINT), http_handler);
        Ok(handlers)
    }

    async fn create_handlers(
        &self,
    ) -> std::io::Result<
        std::collections::HashMap<
            String,
            avalanche_types::rpcchainvm::common::http_handler::HttpHandler,
        >,
    > {
        use super::service::{Service, ServiceImpl, SERVICE_PUBLICENDPOINT};
        log::debug!("create_handlers called");

        // make a new jsonrpc service with this vm as a reference
        let mut io = jsonrpc_core::IoHandler::new();
        let service = ServiceImpl { vm: self.clone() };

        // Allow [io] to handle methods defined in service.rs
        io.extend_with(service.to_delegate());
        let http_handler = rpcchainvm::common::http_handler::HttpHandler::new_from_u8(0, io)
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?;

        let mut handlers = std::collections::HashMap::new();
        handlers.insert(String::from(SERVICE_PUBLICENDPOINT), http_handler);
        Ok(handlers)
    }
}

#[tonic::async_trait]
impl rpcchainvm::snowman::block::Getter for ChainVm {
    async fn get_block(
        &self,
        id: ids::Id,
    ) -> Result<Box<dyn rpcchainvm::concensus::snowman::Block>> {
        let vm = self.inner.write().await;
        log::debug!("kvvm get_block called");

        let current_db = vm.db_manager.current().await.map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to get current db: {:?}", e),
            )
        })?;

        let state = crate::state::State::new(Some(current_db.clone()));

        match state.get_block(id).await? {
            Some(mut block) => {
                let block_id = block.initialize(self.clone())?;

                log::debug!("found old block id: {}", block_id.to_string());

                Ok(Box::new(block))
            }
            None => Err(Error::new(
                ErrorKind::NotFound,
                format!("failed to get block id: {}", id),
            )),
        }
    }
}

#[tonic::async_trait]
impl rpcchainvm::snowman::block::Parser for ChainVm {
    async fn parse_block(
        &self,
        bytes: &[u8],
    ) -> Result<Box<dyn rpcchainvm::concensus::snowman::Block>> {
        let vm = self.inner.write().await;
        log::debug!(
            "kvvm parse_block called: {}",
            String::from_utf8_lossy(&bytes)
        );

        let mut new_block: Block = serde_json::from_slice(bytes.as_ref())?;
        new_block.status = Status::Processing;

        let current_db = vm.db_manager.current().await.map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to get current db: {:?}", e),
            )
        })?;

        let state = crate::state::State::new(Some(current_db.clone()));

        let new_block_id = new_block
            .initialize(self.clone())
            .map_err(|e| Error::new(ErrorKind::Other, format!("failed to init block: {:?}", e)))?;

        match state.get_block(new_block_id).await? {
            Some(mut old_block) => {
                let old_block_id = old_block.initialize(self.clone())?;
                log::debug!("parsed old block id: {}", old_block_id.to_string());
                Ok(Box::new(old_block))
            }
            None => {
                log::debug!("parsed new block id: {}", new_block_id);
                Ok(Box::new(new_block))
            }
        }
    }
}

#[tonic::async_trait]
impl rpcchainvm::snowman::block::ChainVm for ChainVm {
    async fn build_block(&self) -> Result<Box<dyn rpcchainvm::concensus::snowman::Block>> {
        log::debug!("build_block called");
        use avalanche_types::rpcchainvm::snowman::block::Getter;
        let mut vm = self.inner.write().await;

        // Pop next block from mempool error if empty
        let block_value = vm
            .mempool
            .pop()
            .ok_or_else(|| Error::new(ErrorKind::Other, "there is no block to propose"))?;

        // Get Preferred Block
        let preferred_block = self.get_block(vm.preferred).await?;

        let mut new_block = Block::new(
            vm.preferred,
            preferred_block.height().await + 1,
            block_value,
            chrono::offset::Utc::now(),
            Status::Processing,
        );

        let new_block_id = new_block.initialize(self.clone())?;

        new_block.verify().await.map_err(|e| {
            Error::new(ErrorKind::Other, format!("failed to verify block: {:?}", e))
        })?;

        // Add block as verified
        vm.verified_blocks.insert(new_block_id, preferred_block);
        log::debug!("block verified {:?}", new_block_id);

        Ok(Box::new(new_block))
    }

    async fn set_preference(&self, id: ids::Id) -> Result<()> {
        log::info!("setting preferred block id...");
        let mut vm = self.inner.write().await;
        vm.preferred_block_id = Some(id);
        Ok(())
    }

    async fn last_accepted(&self) -> Result<ids::Id> {
        let vm = self.inner.write().await;
        let current_db = vm.db_manager.current().await.map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to get current db: {:?}", e),
            )
        })?;

        let state = crate::state::State::new(Some(current_db.clone()));

        match state.get_last_accepted_block_id().await? {
            Some(last_accepted_block_id) => Ok(last_accepted_block_id),
            None => Err(Error::new(
                ErrorKind::NotFound,
                "failed to get last accepted block",
            )),
        }
    }

    async fn issue_tx(&self) -> Result<Box<dyn rpcchainvm::concensus::snowman::Block>> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "issue tx not implemented",
        ))
    }
}

impl rpcchainvm::vm::Vm for ChainVm {}
