#![allow(dead_code)]
#![allow(unused_imports)]

use avalanche_types::ids;
use jsonrpc_derive::rpc;
use jsonrpc_http_server::jsonrpc_core::{BoxFuture, IoHandler, Params, Result as RpcResult, Value};
use jsonrpc_http_server::ServerBuilder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, Error, ErrorKind};
use std::sync::{Arc, Mutex, RwLock};
use std::time;
use tonic::transport::Channel;
// use tokio::sync::RwLock;

use avalanche_proto::{
    vm::vm_server::Vm,
    appsender::app_sender_client::AppSenderClient,
    messenger::messenger_client::MessengerClient,
};

use crate::block::Block;
use crate::engine::*;

// #[derive(Debug)]
pub struct ChainVMInterior {
    pub ctx: Option<Context>,
    pub bootstrapped: bool,
}

impl ChainVMInterior {
    pub fn new() -> Self {
        Self {
            ctx: None,
            bootstrapped: false,
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

impl VM for ChainVMInterior {
    fn initialize(
        vm_inner: &Arc<RwLock<ChainVMInterior>>,
        ctx: Option<Context>,
        _db_manager: &DbManager,
        _genesis_bytes: &[u8],
        _upgrade_bytes: &[u8],
        _config_bytes: &[u8],
        _to_engine: &MessengerClient<Channel>,
        _fxs: &[Fx],
        _app_sender: &AppSenderClient<Channel>,
    ) -> Result<(), Error> {
        let mut writable_interior = vm_inner.write().unwrap();
        writable_interior.ctx = ctx;
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
    fn last_accepted() -> Result<ids::Id, Error> {
        Ok(ids::Id::default())
    }
}
