#![allow(dead_code)]
#![allow(unused_imports)]

use std::collections::HashMap;
use std::future::Future;
use std::io::{self, Error, ErrorKind};
use std::sync::Arc;
use std::time;

use async_trait::async_trait;
use avalanche_types::ids::{short::Id as ShortId, Id};
use jsonrpc_derive::rpc;
use jsonrpc_http_server::jsonrpc_core::{BoxFuture, IoHandler, Params, Result as RpcResult, Value};
use jsonrpc_http_server::ServerBuilder;
use prost::bytes::Bytes;
use semver::Version;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};
use tonic::transport::Channel;

use avalanche_proto::{
    appsender::app_sender_client::AppSenderClient, messenger::messenger_client::MessengerClient,
    rpcdb::database_client::DatabaseClient, vm::vm_server::Vm,
};

use crate::block::{Block, Status};
use crate::engine::*;
use crate::genesis::Genesis;
use crate::state::{Database, State, BLOCK_DATA_LEN};

#[derive(Debug)]
pub struct ChainVMInterior {
    pub ctx: Option<Context>,
    pub bootstrapped: bool,
    pub version: Version,
    pub genesis: Genesis,
    pub db: Option<Database>,
    pub state: State,
    pub preferred: Id,
    pub mempool: Vec<[u8; BLOCK_DATA_LEN]>,
    pub verified_blocks: HashMap<Id, Block>,
    pub last_accepted: Block,
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
        // let mut db = crate::state::Interior::new(current_db);

        vm.db = Some(current_db.clone());

        log::info!("kvvm: pre init_genesis");

        let state = State::new(Some(current_db.clone()));
        vm.state = state;

        if genesis_bytes.len() > BLOCK_DATA_LEN {
            return Err(Error::new(
                ErrorKind::Other,
                format!(
                    "genesis data must not exceed 32 bytes: len {}",
                    genesis_bytes.len(),
                ),
            ));
        }

        let genesis = Genesis::from_json(genesis_bytes).unwrap();
        vm.genesis = genesis;
        match vm.genesis.verify() {
            Ok(g) => g,
            Err(e) => eprintln!("failed to verify genesis: {:?}", e),
        }

        if vm.state.has_last_accepted_block().await? {
            let last_block_id = vm
                .state
                .get_last_accepted_block_id()
                .await
                .unwrap()
                .unwrap();
            let last_block = vm.state.get_block(last_block_id).await.unwrap().unwrap();

            vm.preferred = last_block_id;
            vm.last_accepted = last_block;
            log::info!("initialized from last accepted block {:?}", last_block_id)
        } else {
            let genesis_block_vec = Id::from_slice(genesis_bytes).to_vec();
            let genesis_block_bytes = genesis_block_vec.try_into().unwrap();

            let mut genesis_block = Block::new(
                Id::empty(),
                0,
                genesis_block_bytes,
                chrono::offset::Utc::now(),
                Status::Processing,
            )?;

            let genesis_block_id = genesis_block.init()?.clone();

            match vm.state.put_block(genesis_block.clone()).await {
                Ok(g) => g,
                Err(e) => eprintln!("failed to put genesis block: {:?}", e),
            }

            log::info!("initialized from genesis block {:?}", genesis_block_id)
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

impl Getter for ChainVMInterior {
    fn get_block(id: Id) -> Result<Block, Error> {
        Ok(Block::default())
    }
}

impl Parser for ChainVMInterior {
    fn parse_block(_bytes: &[u8]) -> Result<Block, Error> {
        Ok(Block::default())
    }
}
#[async_trait]
impl ChainVM for ChainVMInterior {
    async fn build_block(inner: &Arc<RwLock<ChainVMInterior>>) -> Result<Block, Error> {
        let mut vm = inner.write().await;

        // Pop next block from mempool error if empty
        let block_value = vm
            .mempool
            .pop()
            .ok_or_else(|| Error::new(ErrorKind::Other, "there is no block to propose"))?;

        // TODO; manage error vs unwrap
        // Get Preferred Block
        let preferred_block = Self::get_block(vm.preferred).unwrap();

        let preferred_height = preferred_block.height();

        let new_block = Block::new(
            vm.preferred,
            preferred_height + 1,
            block_value,
            chrono::offset::Utc::now(),
            Status::Processing,
        )
        .unwrap();

        // TODO: verify block

        Ok(new_block)
    }
    fn issue_tx() -> Result<Block, Error> {
        Ok(Block::default())
    }
    fn set_preference(_id: Id) -> Result<(), Error> {
        Ok(())
    }

    async fn last_accepted(inner: &Arc<RwLock<ChainVMInterior>>) -> Result<Id, Error> {
        log::info!("last_accepted");
        let interior = inner.read().await;

        log::info!("last_accepted interior {:?}", interior);
        let mut state = crate::state::State::new(Some(interior.db.clone().unwrap()));

        log::info!("last_accepted state ok");
        let last_accepted_id = state.get_last_accepted_block_id().await;

        log::info!("last_accepted_id: {:?}", last_accepted_id);
        let next = last_accepted_id.unwrap();

        log::info!("next value: {:?}", next);

        if next.is_none() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("last_accepted not found"),
            ));
        }

        Ok(next.unwrap())
    }

    //TODO remove
    async fn initialize_genesis(
        inner: &Arc<RwLock<ChainVMInterior>>,
        genesis_bytes: &[u8],
    ) -> Result<(), Error> {
        log::info!("initialize genesis called");
        Ok(())
    }
}
