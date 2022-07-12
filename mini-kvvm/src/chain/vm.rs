use std::io::Result;

use avalanche_types::ids::Id;

use crate::genesis::Genesis;
use crate::chain::block::StatelessBlock;

#[tonic::async_trait]
pub trait Vm {
    async fn genesis(&self) -> Genesis;
    async fn is_bootstrapped(&self);
    async fn state(&self)
        -> Box<dyn avalanche_types::rpcchainvm::database::Database + Send + Sync>;
    async fn get_stateless_block(&self, block_id: Id) -> Result<StatelessBlock>;
}
