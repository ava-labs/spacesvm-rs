#![allow(dead_code)]
#![allow(unused_imports)]

use avalanche_types::ids;
use std::collections::HashMap;
use std::io::{self, Error, ErrorKind};
use std::sync::{Arc, Mutex};
use std::time;

use crate::engine::*;

#[derive(Debug)]
pub struct Handler {
    db: Db,
}

#[derive(Debug, Clone)]
struct Db {
    shared: Arc<Shared>,
}

#[derive(Debug)]
struct Shared {
    state: Mutex<State>,
}

#[derive(Debug)]
struct State {
    bootstrapped: bool,
}

pub struct MiniKVVM;

impl AppHandler for MiniKVVM {
    fn app_request(
        node_id: &ids::ShortId,
        request_id: u32,
        deadline: time::Instant,
        request: &[u8],
    ) -> Result<(), Error> {
        Ok(())
    }
    fn app_request_failed(node_id: &ids::ShortId, request_id: u32) -> Result<(), Error> {
        Ok(())
    }
    fn app_response(node_id: &ids::ShortId, request_id: u32, response: &[u8]) -> Result<(), Error> {
        Ok(())
    }
    fn app_gossip(node_id: &ids::ShortId, msg: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}

impl Connector for MiniKVVM {
    fn connected(id: &ids::ShortId) -> Result<(), Error> {
        Ok(())
    }
    fn disconnected(id: &ids::ShortId) -> Result<(), Error> {
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
            server_addr: String::from("127.0.0.1:2379"),
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
