pub mod base;
pub mod bucket;
pub mod decoder;
pub mod delete;
pub mod heap;
pub mod set;
pub mod tx;
pub mod unsigned;

use std::io::Result;

use avalanche_types::{ids, rpcchainvm};

use crate::block::Block;

#[tonic::async_trait]
#[typetag::serde]
pub trait Transaction: Send + Sync {
    async fn init(&mut self) -> Result<()>;
    async fn bytes(&self) -> &Vec<u8>;
    async fn size(&self) -> u64;
    async fn id(&self) -> ids::Id;
    async fn execute(
        &self,
        db: Box<dyn rpcchainvm::database::Database + Send + Sync>,
        block: Block,
    ) -> Result<()>;
}
