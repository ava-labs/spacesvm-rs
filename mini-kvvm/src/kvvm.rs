#![allow(dead_code)]
#![allow(unused_imports)]

use std::collections::HashMap;
use std::future::Future;
use std::io::{self, Error, ErrorKind};
use std::sync::Arc;
use std::time;

use async_trait::async_trait;
use avalanche_types::ids;
use jsonrpc_derive::rpc;
use jsonrpc_http_server::jsonrpc_core::{BoxFuture, IoHandler, Params, Result as RpcResult, Value};
use jsonrpc_http_server::ServerBuilder;
use semver::Version;
use serde::{Deserialize, Serialize};
use tonic::transport::Channel;
use tokio::sync::{Mutex, RwLock};


use avalanche_proto::{
    appsender::app_sender_client::AppSenderClient, messenger::messenger_client::MessengerClient,
    rpcdb::database_client::DatabaseClient, vm::vm_server::Vm,
};

use crate::block::Block;
use crate::engine::*;
use crate::genesis::Genesis;
use crate::state::{Database, State, BLOCK_DATA_LEN};

pub struct ChainVMInterior {
    pub ctx: Option<Context>,
    pub bootstrapped: bool,
    pub version: Version,
    pub genesis: Genesis,
    pub db: Option<Database>,
    pub state: Option<State>,
}

impl ChainVMInterior {
    pub fn new() -> Self {
        Self {
            ctx: None,
            bootstrapped: false,
            version: Version::new(0, 0, 1),
            genesis: Genesis::default(),
            db: None,
            state: None,
        }
    }
}

// This VM doesn't (currently) have any app-specific messages
impl AppHandler for ChainVMInterior {
    fn app_request(
        _node_id: &ids::ShortId,
        _request_id: u32,
        _deadline: time::Instant,
        _request: &[u8],
    ) -> Result<(), Error> {
        Ok(())
    }

    fn app_request_failed(_node_id: &ids::ShortId, _request_id: u32) -> Result<(), Error> {
        Ok(())
    }

    fn app_response(
        _node_id: &ids::ShortId,
        _request_id: u32,
        _response: &[u8],
    ) -> Result<(), Error> {
        Ok(())
    }

    fn app_gossip(_node_id: &ids::ShortId, _msg: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}

// This VM doesn't implement Connector these methods are noop.
impl Connector for ChainVMInterior {
    fn connected(_id: &ids::ShortId) -> Result<(), Error> {
        Ok(())
    }
    fn disconnected(_id: &ids::ShortId) -> Result<(), Error> {
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
        let mut interior = vm_inner.write().await;
        interior.ctx = ctx;

        let current_db = &db_manager[0].database;
        // let mut db = crate::state::Interior::new(current_db);

        interior.db = Some(current_db.clone());

        let state = State::new(current_db.clone());
        interior.state = Some(state);

        log::info!("testChainVMInterior");

        // store genesis to struct
        let genesis = Genesis::from_json(genesis_bytes).unwrap();
        interior.genesis = genesis;
        match interior.genesis.verify() {
            Ok(g) => g,
            Err(e) => eprintln!("failed to verify genesis: {:?}", e),
        }

        // // TODO: just testing
        // let bytes = vec![0x41, 0x42, 0x43];
        // let out = crate::state::State::put(&mut db, ids::Id::default(), bytes);
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
    fn get_block(_id: String) -> Result<Block, Error> {
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
    fn build_block() -> Result<Block, Error> {
        Ok(Block::default())
    }
    fn issue_tx() -> Result<Block, Error> {
        Ok(Block::default())
    }
    fn set_preference(_id: ids::Id) -> Result<(), Error> {
        Ok(())
    }
    async fn last_accepted(inner: &Arc<RwLock<ChainVMInterior>>) -> Result<ids::Id, Error> {
        let interior = inner.read().await;
        let mut state = crate::state::State::new(interior.db.clone().unwrap());
        // db::get_last_accepted_block_id();
        let out = state.get_last_accepted_block_id();
        let something = out.await?.unwrap();
        Ok(something)
    }
}
