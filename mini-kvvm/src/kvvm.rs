use std::{
    collections::HashMap,
    convert::TryInto,
    io::{Error, ErrorKind, Result},
    sync::Arc,
};

use avalanche_types::{
    choices::{self, status::Status},
    ids,
    rpcchainvm::{
        common,
        concensus::snowman,
        context::Context,
        database,
        database::manager::{versioned_database::VersionedDatabase, Manager},
        state::State,
    },
};
use chrono::{DateTime, NaiveDateTime, Utc};
use lru::LruCache;
use semver::Version;
use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;

use crate::chain::{
    block::StatelessBlock,
    genesis::Genesis,
    network::Network,
    // vm::Vm,
    storage::{get_last_accepted, has_last_accepted},
    txn::TransactionInterior,
    vm::Vm,
};

use crate::chain;

const BLOCKS_LRU_SIZE: usize = 128;

pub struct VmInterior {
    pub ctx: Option<Context>,
    pub bootstrapped: bool,
    pub version: Version,
    pub genesis: Genesis,
    pub db: Option<Box<dyn database::Database + Send + Sync>>,
    pub block: LruCache<ids::Id, StatelessBlock>,
    pub preferred: ids::Id,
    pub mempool: Vec<Vec<u8>>,
    pub verified_blocks: HashMap<ids::Id, StatelessBlock>,
    pub last_accepted: StatelessBlock,
    pub preferred_block_id: Option<ids::Id>,
    pub network: Network,
    pub app_sender: Option<Box<dyn common::appsender::AppSender + Send + Sync>>,
}

pub struct Vm {
    inner: Arc<RwLock<VmInterior>>,
}

impl Vm {
    pub fn new() -> Box<dyn avalanche_types::rpcchainvm::vm::Vm + Send + Sync> {
        let mut cache: LruCache<ids::Id, StatelessBlock> = LruCache::new(BLOCKS_LRU_SIZE);
        let inner = VmInterior {
            ctx: None,
            bootstrapped: false,
            version: Version::new(0, 0, 1), //TODO: lazy static
            genesis: Genesis::default(),
            db: None,
            block: cache,
            preferred: ids::Id::empty(),
            mempool: Vec::new(),
            verified_blocks: HashMap::new(),
            last_accepted: StatelessBlock::default(),
            preferred_block_id: None,
            network: Network::new(),
            app_sender: None,
        };
        Box::new(Vm {
            inner: Arc::new(RwLock::new(inner)),
        })
    }
}

#[tonic::async_trait]
impl avalanche_types::rpcchainvm::vm::Vm for Vm {}

#[tonic::async_trait]
impl avalanche_types::rpcchainvm::common::vm::Vm for Vm {
    async fn initialize(
        &self,
        ctx: Option<Context>,
        db_manager: Box<dyn Manager + Send + Sync>,
        genesis_bytes: &[u8],
        upgrade_bytes: &[u8],
        config_bytes: &[u8],
        to_engine: Sender<common::message::Message>,
        _fxs: &[common::vm::Fx],
        app_sender: Box<dyn common::appsender::AppSender + Send + Sync>,
    ) -> Result<()> {
        let mut vm = self.inner.write().await;
        vm.ctx = ctx;

        let versioned_db = db_manager.current().await?;
        vm.db = Some(versioned_db.inner.clone().into_inner());
        vm.app_sender = Some(app_sender);

        // Try to load last accepted
        let resp = has_last_accepted(vm.db.clone()).await;
        if resp.is_err() {
            log::error!("could not determine if have last accepted");
            return Err(Error::new(ErrorKind::Other, resp.unwrap_err()));
        }
        let has = resp.unwrap();

        // Parse genesis data
        let genesis = Genesis::from_json(genesis_bytes)?;
        vm.genesis = genesis;
        vm.genesis.verify().map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to verify genesis: {:?}", e),
            )
        })?;

        // Check if last accepted block exists
        if has {
            let resp = get_last_accepted(vm.db).await;
            if resp.is_err() {
                log::error!("could not get last accepted");
                return Err(Error::new(ErrorKind::Other, resp.unwrap_err()));
            }
            let block_id = resp.unwrap();

            let resp = self.get_stateless_block(block_id).await;
            if resp.is_err() {
                log::error!("could not get stateless block");
                return Err(Error::new(ErrorKind::Other, resp.unwrap_err()));
            }
            let block = resp.unwrap();

            vm.preferred = block_id;
            vm.last_accepted = block;
            log::info!("initialized vm from last accepted block id: {:?}", block_id)
        } else {
            let genesis_block_vec = genesis_bytes.to_vec();
            let genesis_block_bytes = genesis_block_vec.try_into().unwrap();

            // TODO we need a real genesis is is not going to cut it.
            let mut genesis_block = StatelessBlock::new(
                genesis_block_bytes,
                0, // TODO: use timestamp
                Status::Processing,
            )?;

            let genesis_block_id = genesis_block.initialize()?;

            let accepted_block_id = vm.state.accept_block(genesis_block).await.map_err(|e| {
                Error::new(ErrorKind::Other, format!("failed to accept block: {:?}", e))
            })?;
            // Remove accepted block now that it is accepted
            vm.verified_blocks.remove(&accepted_block_id);

            log::info!("initialized from genesis block: {:?}", genesis_block_id)
        }

        Ok(())
    }

    async fn set_state(&self, state: State) -> Result<()> {
        todo!();
    }

    async fn shutdown(&self) -> Result<()> {
        todo!();
    }

    async fn version(&self) -> Result<String> {
        todo!();
    }

    async fn create_static_handlers(
        &self,
    ) -> Result<HashMap<String, common::http_handler::HttpHandler>> {
        todo!();
    }

    async fn create_handlers(&self) -> Result<HashMap<String, common::http_handler::HttpHandler>> {
        todo!();
    }
}

#[tonic::async_trait]
impl avalanche_types::rpcchainvm::common::apphandler::AppHandler for Vm {}

#[tonic::async_trait]
impl avalanche_types::rpcchainvm::common::vm::Connector for Vm {}

#[tonic::async_trait]
impl avalanche_types::rpcchainvm::snowman::block::ChainVm for Vm {
    async fn build_block(&self) -> Result<Box<dyn snowman::Block>> {
        todo!();
    }

    /// Issues a transaction to the chain
    async fn issue_tx(&self) -> Result<Box<dyn snowman::Block>> {
        todo!();
    }

    /// Notify the Vm of the currently preferred block.
    async fn set_preference(&self, id: ids::Id) -> Result<()> {
        todo!();
    }

    /// Returns the ID of the last accepted block.
    /// If no blocks have been accepted, this should return the genesis block
    async fn last_accepted(&self) -> Result<ids::Id> {
        todo!();
    }
}

#[tonic::async_trait]
impl crate::chain::network::PushNetwork for Vm {
    async fn send_txs(&self, txs: Vec<TransactionInterior>) -> Result<()> {
        if txs.len() == 0 {
            return Ok(());
        }

        let vm = self.inner.read().await;
        let b = serde_json::from_slice(txs)
            .map_err(|e| Error::Serde(format!("failed to marshal txs: {:?}", e.to_string())))?;

        let client = vm.app_sender.expect("appsender should never be none");
        client
            .send_app_gossip(b)
            .await
            .map_err(|e| log::warn!("GossipTxs failed: {:?}", e.to_string()));
        Ok(())
    }

    async fn gossip_new_tx(&self, new_tx: Vec<TransactionInterior>) -> Result<()> {
        Ok(())
    }

    async fn regossip_tx(&self) -> Result<()> {
        todo!();
    }
}

#[tonic::async_trait]
impl crate::chain::vm::Vm for Vm {
    async fn genesis(&self) -> Genesis {
        let vm = self.inner.read().await;
        return vm.genesis;
    }

    async fn is_bootstrapped(&self) -> bool {
        let vm = self.inner.read().await;
        return vm.bootstrapped;
    }

    async fn state(
        &self,
    ) -> Box<dyn avalanche_types::rpcchainvm::database::Database + Send + Sync> {
        let vm = self.inner.read().await;
        return vm.db.unwrap();
    }

    async fn get_stateless_block(&self, block_id: ids::Id) -> Result<&StatelessBlock> {
        let vm = self.inner.read().await;

        // has the block been cached from previous "Accepted" call
        let resp = vm.block.get(&block_id);
        if resp.is_some() {
            return Ok(resp.unwrap());
        }

        // has the block been verified, not yet accepted
        let block = vm.verified_blocks.get(&block_id);
        if block.is_some() {
            return Ok(block.unwrap());
        }

        let stateful_block = chain::storage::get_block(vm.db.unwrap(), block_id)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

        return chain::block::parse_stateful_block(
            stateful_block,
            (),
            choices::status::Status::Accepted,
            &vm.genesis,
        );
    }

    async fn execution_context(
        &self,
        current_time: u64,
        parent: &StatelessBlock,
    ) -> Result<chain::vm::Context> {
        todo!()
    }
}

#[tonic::async_trait]
impl avalanche_types::rpcchainvm::health::Checkable for Vm {
    /// Checks if the database has been closed.
    async fn health_check(&self) -> Result<Vec<u8>> {
        Ok(vec![])
    }
}

#[tonic::async_trait]
impl avalanche_types::rpcchainvm::snowman::block::Getter for Vm {
    // Attempts to return a block based on Id.
    async fn get_block(&self, id: ids::Id) -> Result<StatelessBlock> {
        log::debug!("kvvm get_block called");

        let vm = self.inner.read().await;
        let db = vm.db.clone();
        match self.get_stateless_block(id).await? {
            Some(mut block) => {
                let block_id = block.initialize()?;
                log::debug!("found old block id: {}", block_id.to_string());
                Ok(block)
            }
            None => Err(Error::new(
                ErrorKind::NotFound,
                format!("failed to get block id: {}", id),
            )),
        }
    }
}

#[tonic::async_trait]
impl avalanche_types::rpcchainvm::snowman::block::Parser for Vm {}
