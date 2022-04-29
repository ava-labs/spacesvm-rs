#![allow(dead_code)]
#![allow(unused_imports)]

use std::sync::{Arc, Mutex};

use avalanche_proto::vm::vm_server::Vm;

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

// impl Vm for Handler {
// TODO
// }
