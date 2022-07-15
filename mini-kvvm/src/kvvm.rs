use std::{
    collections::HashMap,
    convert::TryInto,
    io::{Error, ErrorKind, Result},
    sync::Arc,
    time,
};

use avalanche_proto::{
    appsender::app_sender_client::AppSenderClient, messenger::messenger_client::MessengerClient,
};
use avalanche_types::{
    choices::status::Status,
    ids::{node::Id as NodeId, Id},
    rpcchainvm::{
        block::Parser,
        database::manager::{versioned_database::VersionedDatabase, Manager},
    },
    vm::context::Context,
    vm::state::State as VmState,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use lru::LruCache;
use semver::Version;
use tokio::sync::RwLock;
use tonic::transport::Channel;

use crate::chain::{
    block::StatelessBlock,
    genesis::Genesis,
    network::Network,
    // vm::Vm,
    storage::{get_last_accepted, has_last_accepted},
    txn::TransactionInterior,
};

const BLOCKS_LRU_SIZE: usize = 128;

pub struct VmInterior {
    pub ctx: Option<Context>,
    pub bootstrapped: bool,
    pub version: Version,
    pub genesis: Genesis,
    pub db: Option<VersionedDatabase>,
    pub block: LruCache<Id, StatelessBlock>,
    pub preferred: Id,
    pub mempool: Vec<Vec<u8>>,
    pub verified_blocks: HashMap<Id, StatelessBlock>,
    pub last_accepted: StatelessBlock,
    pub preferred_block_id: Option<Id>,
    pub network: Network,
    pub app_sender: Option<AppSenderClient<Channel>>,
}

pub struct Vm {
    inner: Arc<RwLock<VmInterior>>,
}

// Database is local scope which allows the following usage.
// database::memdb::Database::new()
impl Vm {
    pub fn new() -> Box<dyn crate::block::ChainVm + Send + Sync> {
        let mut cache: LruCache<Id, StatelessBlock> = LruCache::new(BLOCKS_LRU_SIZE);
        let inner = VmInterior {
            ctx: None,
            bootstrapped: false,
            version: Version::new(0, 0, 1), //TODO: lazy static
            genesis: Genesis::default(),
            db: None,
            block: cache,
            preferred: Id::empty(),
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
impl crate::chain::vm::Vm for Vm {
    async fn initialize(
        &self,
        ctx: Option<Context>,
        db_manager: Box<dyn Manager>,
        genesis_bytes: &[u8],
        upgrade_bytes: &[u8],
        config_bytes: &[u8],
        to_engine: &MessengerClient<Channel>,
        _fxs: (),
        app_sender: &AppSenderClient<Channel>,
    ) -> Result<()> {
        let mut vm = self.inner.write().await;
        vm.ctx = ctx;

        let versioned_db = db_manager.current().await?;
        vm.db = versioned_db.inner.clone().into_inner();
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

            let mut genesis_block = StatelessBlock::new(
                Id::empty(),
                0,
                genesis_block_bytes,
                DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc),
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

    // Attempts to return a block based on Id.
    async fn get_block(&self, id: Id) -> Result<StatelessBlock> {
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
        let db = self.inner.read().await;
        return db.clond().into();
    }

    async fn get_stateless_block(&self, block_id: Id) -> Result<StatelessBlock> {
        let vm = self.inner.read().await;

        // has the block been cached from previous "Accepted" call
        let resp = vm.block.get(&block_id);
        if resp.is_some {
            Ok(resp.unwrap());
        }

        // has the block been verified, not yet accepted
        if vm.verified_blocks.contains_key(&block_id) {
            Ok(vm.verified_blocks.get_key_value(&block_id))
        }

        let db = vm.db.cloned();
        return crate::chain::storage::get_block(db, block_id);
    }

    // Push Network
    async fn send_txs(&self, txs: Vec<TransactionInterior>) -> Result<()> {
        if txs.len() == 0 {
            Ok(())
        }

        let vm = self.inner.read().await;

         if vm.app_sender.is_some() {
            let client = vm.app_sender.unwrap();
            let resp = client.send_app_gossip(request);
         } 

        Ok(())
    }

    async fn gossip_new_tx(&self, new_tx: Vec<TransactionInterior>) -> Result<()> {
        Ok(())
    }

    async fn regossip_tx(&self) -> Result<()> {
        Ok(())
    }
}

//setstate
//shutdown
//version
// create static handlers
// create handlers
// }

/// ...
/// AppHandler (conditional) not used for this impl
/// Connector (conditional) not used for this impl
///
#[tonic::async_trait]
impl avalanche_types::rpcchainvm::health::Checkable for Vm {
    /// Checks if the database has been closed.
    async fn health_check(&self) -> Result<Vec<u8>> {
        Ok(vec![])
    }
}

#[tonic::async_trait]
impl avalanche_types::rpcchainvm::block::Getter for Vm {}
