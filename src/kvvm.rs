#![allow(dead_code)]
#![allow(unused_imports)]

use std::io::{self, Error, ErrorKind};
use std::sync::{Arc, Mutex};
use avalanche_types::ids;
use std::time;

use crate::{engine::*};

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

struct MiniKVVM;

impl AppHandler for MiniKVVM {
    fn app_request(
        node_id: &ids::ShortId,
        request_id: u32,
        deadline: time::Instant,
        request: &[u8],
    ) -> Result<(), Error> {
        Ok(())
    }
    fn app_request_failed(
        node_id: &ids::ShortId,
        request_id: u32,
    ) -> Result<(), Error> {
        Ok(())
    }
    fn app_response(
        node_id: &ids::ShortId,
        request_id: u32,
        response: &[u8],
    ) -> Result<(), Error> {
        Ok(())
    }
    fn app_gossip(node_id: &ids::ShortId, msg: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}

// impl Vm for Handler {
// TODO
// }
