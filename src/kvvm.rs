#![allow(dead_code)]
#![allow(unused_imports)]

use avalanche_types::ids;
use jsonrpc_http_server::jsonrpc_core::{IoHandler, Params, Value};
use jsonrpc_http_server::ServerBuilder;
use std::collections::HashMap;
use std::io::{self, Error, ErrorKind};
use std::sync::{Arc, Mutex};
use std::time;

use crate::engine::*;

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
        db_manager: &DBManager,
        genesis_bytes: &[u8],
        upgrade_bytes: &[u8],
        config_bytes: &[u8],
        to_engine: MessageChannel,
        fxs: &[Fx],
        app_sender: &AppSender,
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
    fn version() -> Result<String, Error> {
        Ok("".to_string())
    }
    fn create_static_handlers() -> Result<HashMap<String, HTTPHandler>, Error> {
        Ok(HashMap::new())
    }
    fn create_handlers() -> Result<HashMap<String, HTTPHandler>, Error> {
        let mut handler: HashMap<String, HTTPHandler> = HashMap::new();
        let s = HTTPHandler {
            // server_addr: String::from("127.0.0.1:2379"),
            lock_options: 0,
        };
        handler.insert(String::from("/cool"), s);
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

// TODO: remove this is just a test for rpc server
pub fn new_test_handler() -> IoHandler {
    let mut handler = IoHandler::default();
    handler.add_sync_method("hello", |params: Params| {
        match params.parse::<(String,)>() {
            Ok((msg,)) => Ok(Value::String(format!("hello {}", msg))),
            _ => Ok(Value::String("world".into())),
        }
    });
    handler
}
