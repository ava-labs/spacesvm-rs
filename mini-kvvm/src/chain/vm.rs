use avalanche_types::ids;

use std::io::Result;

use crate::block;

use super::tx::tx::Transaction;

pub struct Context {
    pub recent_block_ids: ids::Set,
    pub recent_tx_ids: ids::Set,
    pub recent_load_units: u64,
}

#[tonic::async_trait]
pub trait Vm: avalanche_types::rpcchainvm::vm::Vm {
    async fn is_bootstrapped(&self) -> bool;
    async fn submit(&self, txs: Vec<Transaction>) -> Result<()>;
    async fn notify_block_ready(&self);
    async fn new_timed_builder(self) -> Box<dyn block::builder::Builder + Send + Sync>;
    async fn new_push_network(&self) -> block::builder::Timed;
}
