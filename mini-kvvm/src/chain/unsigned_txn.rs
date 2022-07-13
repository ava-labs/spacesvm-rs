use std::io::Result;

use avalanche_types::{ids::Id, rpcchainvm::database::Database};
use ethereum_types::Address;

use crate::chain::{activity::Activity, genesis::Genesis};

pub trait UnsignedTransaction {
    fn copy() -> Box<dyn UnsignedTransaction>;
    fn get_block_id(&self) -> Id;
    fn get_magic(&self) -> u64;
    fn get_price(&self) -> u64;
    fn set_block_id(&self, id: Id);
    fn set_magic(&self, magic: u64);
    fn set_price(&self, magic: u64);
    fn feed_units(&self, genesis: &Genesis) -> u64; // number of units to mine tx
    fn load_units(&self, genesis: &Genesis) -> u64; // units that should impact fee rate
    fn execute_base(&self, genesis: &Genesis) -> Result<()>;
    fn execute(&self, genesis: &TransactionContext) -> Result<()>;
    // TypedData() *tdata.TypedData // TODO
    fn activity(&self) -> Activity;
}

pub struct TransactionContext {
    genesis: Genesis,
    database: Box<dyn Database + Send + Sync>,
    block_time: u64,
    tx_id: Id,
    sender: Address,
}
