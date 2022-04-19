#![allow(dead_code)]
#![allow(unused_imports)]

use avalanche_types::ids;
use jsonrpc_derive::rpc;
use jsonrpc_http_server::jsonrpc_core::{BoxFuture, IoHandler, Params, Result as RpcResult, Value};
use jsonrpc_http_server::ServerBuilder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, Error, ErrorKind};
use std::sync::{Arc, Mutex};
use std::time;

use crate::engine::*;

const DATA_LEN: u32 = 32;
const CODEC_VERSION: u32 = 0;

// http call: name.endpoint
const NAME: &str = "kvvm";

// control how many tx before we build a block
// with current value 2, every time we have >=2 pending tx in mempool, we start building a new block
const TX_BATCH_SIZE: u32 = 2;

// time for periodic flush of pending transaction in the mempool
const TX_BATCH_TIME: time::Duration = time::Duration::from_secs(5);

#[derive(Debug)]
pub struct MiniKVVM {
    bootstrapped: bool,
}

impl MiniKVVM {
    pub fn new() -> Self {
        MiniKVVM {
            bootstrapped: false,
        }
    }
}

// This VM doesn't (currently) have any app-specific messages
impl AppHandler for MiniKVVM {
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
impl Connector for MiniKVVM {
    fn connected(_id: &ids::ShortId) -> Result<(), Error> {
        Ok(())
    }
    fn disconnected(_id: &ids::ShortId) -> Result<(), Error> {
        Ok(())
    }
}

impl Checkable for MiniKVVM {
    fn health_check() -> Result<Health, Error> {
        Ok(())
    }
}

impl VM for MiniKVVM {
    fn initialize(
        ctx: &Context,
        db_manager: &DbManager,
        genesis_bytes: &[u8],
        upgrade_bytes: &[u8],
        config_bytes: &[u8],
        to_engine: MessageChannel,
        _fxs: &[Fx],
        _app_sender: &AppSender,
    ) -> Result<(), Error> {
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

    /// Returns this VM's version
    fn version() -> Result<String, Error> {
        Ok("".to_string())
    }
    fn create_static_handlers() -> Result<HashMap<String, HTTPHandler>, Error> {
        Ok(HashMap::new())
    }
    fn create_handlers() -> Result<HashMap<String, HTTPHandler>, Error> {
        let mut handler: HashMap<String, HTTPHandler> = HashMap::new();
        let mut io_handler = IoHandler::default();

        let ping = PingApiImp;
        io_handler.extend_with(ping.to_delegate());

        let s = HTTPHandler {
            lock_options: 0,
            handler: io_handler,
        };
        handler.insert(String::from("/rpc"), s);
        Ok(handler)
    }
}

impl Getter for MiniKVVM {
    fn get_block(id: ids::Id) -> Result<Block, Error> {
        Ok(())
    }
}

impl Parser for MiniKVVM {
    fn parse_block(bytes: &[u8]) -> Result<Block, Error> {
        Ok(())
    }
}

impl ChainVM for MiniKVVM {
    fn build_block() -> Result<Block, Error> {
        Ok(())
    }
    fn set_preference(id: ids::Id) -> Result<(), Error> {
        Ok(())
    }
    fn last_accepted() -> Result<ids::Id, Error> {
        Ok(ids::Id::default())
    }
}

#[derive(Serialize, Deserialize)]
pub struct PingReply {
    success: bool,
}

#[derive(Serialize, Deserialize)]
pub struct PingSuccess {
    success: bool,
}

#[rpc(server)]
pub trait PingApi {
    #[rpc(name = "ping", alias("subnet.ping"))]
    fn ping(&self) -> BoxFuture<RpcResult<PingReply>>;
}

pub struct PingApiImp;

impl PingApi for PingApiImp {
    fn ping(&self) -> BoxFuture<RpcResult<PingReply>> {
        Box::pin(async move {
            log::info!("Ping");
            Ok(PingReply { success: true })
        })
    }
}
