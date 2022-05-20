#![allow(dead_code)]

use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::time;

use async_trait::async_trait;
use avalanche_proto::{
    appsender::app_sender_client::AppSenderClient, messenger::messenger_client::MessengerClient,
};
use avalanche_types::ids::{short::Id as ShortId, Id};
use semver::Version;
use tokio::sync::RwLock;
use tonic::transport::Channel;

use crate::block::{Block, Status};
use crate::engine::*;
use crate::genesis::Genesis;
use crate::state::{Database, State};

#[derive(Debug)]
pub struct ChainVMInterior {
    pub ctx: Option<Context>,
    pub bootstrapped: bool,
    pub version: Version,
    pub genesis: Genesis,
    pub db: Option<Database>,
    pub state: State,
    pub preferred: Id,
    pub mempool: Vec<Vec<u8>>,
    pub verified_blocks: HashMap<Id, Block>,
    pub last_accepted: Block,
    preferred_block_id: Option<Id>,
}

impl ChainVMInterior {
    pub fn new() -> Self {
        Self {
            ctx: None,
            bootstrapped: false,
            version: Version::new(0, 0, 1),
            genesis: Genesis::default(),
            db: None,
            state: State::default(),
            preferred: Id::empty(),
            mempool: Vec::new(),
            verified_blocks: HashMap::new(),
            last_accepted: Block::default(),
            preferred_block_id: None,
        }
    }
}

// This VM doesn't (currently) have any app-specific messages
impl AppHandler for ChainVMInterior {
    fn app_request(
        _node_id: &ShortId,
        _request_id: u32,
        _deadline: time::Instant,
        _request: &[u8],
    ) -> Result<(), Error> {
        Ok(())
    }

    fn app_request_failed(_node_id: &ShortId, _request_id: u32) -> Result<(), Error> {
        Ok(())
    }

    fn app_response(_node_id: &ShortId, _request_id: u32, _response: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn app_gossip(_node_id: &ShortId, _msg: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}

// This VM doesn't implement Connector these methods are noop.
impl Connector for ChainVMInterior {
    fn connected(_id: &ShortId) -> Result<(), Error> {
        Ok(())
    }
    fn disconnected(_id: &ShortId) -> Result<(), Error> {
        Ok(())
    }
}

impl Checkable for ChainVMInterior {
    fn health_check() -> Result<Health, Error> {
        Ok(())
    }
}

#[tonic::async_trait]
impl VM for ChainVMInterior {
    async fn initialize(
        vm_inner: &Arc<RwLock<ChainVMInterior>>,
        ctx: Option<Context>,
        db_manager: &DbManager,
        genesis_bytes: &[u8],
        _upgrade_bytes: &[u8],
        _config_bytes: &[u8],
        _to_engine: &MessengerClient<Channel>,
        _fxs: &[Fx],
        _app_sender: &AppSenderClient<Channel>,
    ) -> Result<(), Error> {
        let mut vm = vm_inner.write().await;
        vm.ctx = ctx;

        let current_db = &db_manager[0].database;
        vm.db = Some(current_db.clone());

        let state = State::new(Some(current_db.clone()));
        vm.state = state;

        let genesis = Genesis::from_json(genesis_bytes).map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to deserialize genesis: {:?}", e),
            )
        })?;
        vm.genesis = genesis;

        vm.genesis.verify().map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to verify genesis: {:?}", e),
            )
        })?;

        // Check if last accepted block exists
        if vm.state.has_last_accepted_block().await? {
            let last_accepted_block_id = vm
                .state
                .get_last_accepted_block_id()
                .await
                .map_err(|e| {
                    Error::new(
                        ErrorKind::Other,
                        format!("failed to get last accepted block id: {:?}", e),
                    )
                })?
                .unwrap();

            let last_accepted_block = vm
                .state
                .get_block(last_accepted_block_id)
                .await
                .map_err(|e| {
                    Error::new(
                        ErrorKind::Other,
                        format!("failed to verify genesis: {:?}", e),
                    )
                })?
                .unwrap();

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
                Id::default(),
                0,
                genesis_block_bytes,
                chrono::offset::Utc::now(),
                Status::Processing,
            )?;

            let genesis_block_id = genesis_block.init()?;

            vm.state.put_block(genesis_block).await.map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("failed to verify genesis: {:?}", e),
                )
            });

            let accepted_block_id = vm.state.accept_block(genesis_block).await?;
            // Remove accepted block now that it is accepted
            vm.verified_blocks.remove(&accepted_block_id);

            log::info!("initialized from genesis block: {}", genesis_block_id)
        }

        Ok(())
    }
    fn bootstrapping() -> Result<(), Error> {
        Ok(())
    }
    fn bootstrapped() -> Result<(), Error> {
        Ok(())
    }
    fn shutdown() -> Result<(), Error> {
        Ok(())
    }

    async fn set_state(inner: &Arc<RwLock<ChainVMInterior>>) -> Result<(), Error> {
        let mut interior = inner.write().await;
        // TODO: correctly implement
        interior.bootstrapped = true;
        Ok(())
    }

    fn issue_tx(_key: String, _value: String) -> Result<(), Error> {
        Ok(())
    }

    /// Returns this VM's version
    fn version() -> Result<String, Error> {
        Ok("".to_string())
    }
    fn create_static_handlers() -> Result<HashMap<String, HTTPHandler>, Error> {
        Ok(HashMap::new())
    }
    fn create_handlers() -> Result<HashMap<String, HTTPHandler>, Error> {
        Ok(HashMap::new())
    }
}

#[async_trait]
impl Getter for ChainVMInterior {
    async fn get_block(
        inner: &Arc<RwLock<ChainVMInterior>>,
        id: Id,
    ) -> Result<Option<Block>, Error> {
        log::debug!("kvvm get_block called");
        let vm = inner.write().await;
        let mut state = crate::state::State::new(vm.db.clone());
        Ok(state.get_block(id).await?)
    }
}

#[async_trait]
impl Parser for ChainVMInterior {
    async fn parse_block(
        inner: &Arc<RwLock<ChainVMInterior>>,
        bytes: &[u8],
    ) -> Result<Block, Error> {
        log::debug!(
            "kvvm parse_block called: {}",
            String::from_utf8_lossy(&bytes)
        );
        let mut new_block: Block = serde_json::from_slice(bytes.as_ref()).map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to deserialize block: {:?}", e),
            )
        })?;

        new_block.status = Status::Processing;

        let interior = inner.read().await;
        let mut state = crate::state::State::new(interior.db);

        let new_block_id = new_block
            .init()
            .map_err(|e| Error::new(ErrorKind::Other, format!("failed to init block: {:?}", e)))?;

        match state.get_block(new_block_id).await? {
            Some(mut old_block) => {
                let old_block_id = old_block.init().map_err(|e| {
                    Error::new(ErrorKind::Other, format!("failed to init block: {:?}", e))
                })?;
                log::debug!("found old block id: {}", old_block_id);
                return Ok(old_block);
            }
            None => {
                log::debug!("found new block id: {}", new_block_id);
                Ok(new_block)
            }
        }
    }
}
#[async_trait]
impl ChainVM for ChainVMInterior {
    async fn build_block(inner: &Arc<RwLock<ChainVMInterior>>) -> Result<Block, Error> {
        log::info!("build_block called");
        let mut vm = inner.write().await;

        // Pop next block from mempool error if empty
        let block_value = vm
            .mempool
            .pop()
            .ok_or_else(|| Error::new(ErrorKind::Other, "there is no block to propose"))?;

        // Get Preferred Block
        let preferred_block = Self::get_block(inner, vm.preferred)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, format!("failed to get block: {:?}", e)))?
            .unwrap();

        let new_block = Block::new(
            vm.preferred,
            preferred_block.height() + 1,
            block_value,
            chrono::offset::Utc::now(),
            Status::Processing,
        )
        .map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to create new block: {:?}", e),
            )
        })?;

        new_block.verify(inner).await.map_err(|e| {
            Error::new(ErrorKind::Other, format!("failed to verify block: {:?}", e))
        })?;
        log::debug!("block verified {:?}", new_block.id());

        Ok(new_block)
    }

    async fn issue_tx() -> Result<Block, Error> {
        log::info!("kvvm issue_tx called");
        Ok(Block::default())
    }

    async fn set_preference(inner: &Arc<RwLock<ChainVMInterior>>, id: Id) -> Result<(), Error> {
        log::info!("setting preferred block id...");
        let mut vm = inner.write().await;
        vm.preferred_block_id = Some(id);
        Ok(())
    }

    async fn last_accepted(inner: &Arc<RwLock<ChainVMInterior>>) -> Result<Id, Error> {
        let vm = inner.read().await;
        let mut state = crate::state::State::new(vm.db);
        let last_accepted_block_id = state
            .get_last_accepted_block_id()
            .await
            .map_err(|e| Error::new(ErrorKind::Other, format!("failed to get block: {:?}", e)))?;
        if last_accepted_block_id.is_none() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("last accepted block not found"),
            ));
        }

        Ok(last_accepted_block_id.unwrap())
    }
}
