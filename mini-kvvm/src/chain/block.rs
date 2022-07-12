use std::io::Result;

use avalanche_types::{
    choices::status::Status,
    ids::{must_deserialize_id, Id},
    rpcchainvm::database::VersionedDatabase,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const DATA_LEN: usize = 32;

#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct StatefulBlock {
    #[serde(deserialize_with = "must_deserialize_id")]
    pub parent: Id,
    pub status: Status,
    height: u64,
    timestamp: u64,
    data: Vec<u8>,
}

#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct StatelessBlock {
    block: StatefulBlock,

    #[serde(skip)]
    id: Id,

    #[serde(skip)]
    st: Status,

    #[serde(skip)]
    t: DateTime<Utc>,

    #[serde(skip)]
    bytes: Vec<u8>,

    #[serde(skip)]
    children: Vec<StatelessBlock>,

    #[serde(skip)]
    on_accept_db: Box<dyn VersionedDatabase>,
}

#[tonic::async_trait]
impl avalanche_types::rpcchainvm::block::Block for StatelessBlock {
    /// implements "snowman.Block"
    async fn bytes(&self) -> &[u8] {
        return self.block.bytes.as_ref();
    }

    /// implements "snowman.Block"
    async fn height(&self) -> u64 {
        return self.block.height;
    }

    /// implements "snowman.Block"
    async fn timestamp(&self) -> u64 {
        return self.block.timestamp;
    }

    /// implements "snowman.Block"
    async fn parent(&self) -> Id {
        return self.block.id;
    }

    /// implements "snowman.Block"
    async fn verify(&self) -> Result<()> {
        Ok(())
    }
}

#[tonic::async_trait]
impl avalanche_types::rpcchainvm::block::Decidable for StatelessBlock {
    /// implements "snowman.Block.choices.Decidable"
    async fn status(&self) -> Status {
        return self.block.st;
    }

    /// implements "snowman.Block.choices.Decidable"
    async fn status(&self) -> Id {
        return self.block.id;
    }

    /// implements "snowman.Block.choices.Decidable"
    async fn accept(&mut self) -> Result<()> {
        Ok(())
    }

    /// implements "snowman.Block.choices.Decidable"
    async fn reject(&mut self) -> Result<()> {
        Ok(())
    }
}
