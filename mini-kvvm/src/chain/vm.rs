use avalanche_types::ids;

use std::io::Result;

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
}
