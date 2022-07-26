use std::io::Result;

use avalanche_types::ids;

use crate::chain::block::StatelessBlock;
use crate::chain::genesis::Genesis;

pub struct Context {
    pub recent_block_ids: ids::Set,
    pub recent_tx_ids: ids::Set,
    pub recent_load_units: u64,
}

#[tonic::async_trait]
pub trait Vm: avalanche_types::rpcchainvm::vm::Vm + crate::chain::network::PushNetwork {
    async fn genesis(&self) -> Genesis;
    async fn is_bootstrapped(&self) -> bool;
    async fn state(&self)
        -> Box<dyn avalanche_types::rpcchainvm::database::Database + Send + Sync>;
    async fn get_stateless_block(&self, block_id: ids::Id) -> Result<&StatelessBlock>;
    async fn execution_context(
        &self,
        current_time: u64,
        parent: &StatelessBlock,
    ) -> Result<Context>;
}
