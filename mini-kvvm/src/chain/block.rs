use std::io::{Error, ErrorKind, Result};

use avalanche_types::{
    choices::status::Status,
    ids::{must_deserialize_id, Id},
    rpcchainvm::{block::Block as SnowmanBlock, block::Decidable, database::VersionedDatabase},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha3::Keccak256;

use super::{genesis::Genesis, txn::Transaction};

pub const DATA_LEN: usize = 32;

#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct StatefulBlock {
    #[serde(deserialize_with = "must_deserialize_id")]
    pub parent: Id,
    height: u64,
    timestamp: u64,
    data: Vec<u8>,
    txs: Vec<Box<dyn Transaction>>,
}

#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct StatelessBlock {
    stateful_block: StatefulBlock,

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
    on_accept_db: Option<Box<dyn VersionedDatabase>>,
}

#[tonic::async_trait]
impl SnowmanBlock for StatelessBlock {
    /// implements "snowman.Block"
    async fn bytes(&self) -> &[u8] {
        return self.stateful_block.bytes.as_ref();
    }

    /// implements "snowman.Block"
    async fn height(&self) -> u64 {
        return self.stateful_block.height;
    }

    /// implements "snowman.Block"
    async fn timestamp(&self) -> u64 {
        return self.timestamp;
    }

    /// implements "snowman.Block"
    async fn parent(&self) -> Id {
        return self.stateful_block.parent;
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
        return self.st;
    }

    /// implements "snowman.Block.choices.Decidable"
    async fn id(&self) -> Id {
        return self.id;
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

pub async fn parse_block(source: &[u8], status: Status, genesis: &Genesis) -> Result<StatelessBlock> {
    // Deserialize json bytes to a StatelessBlock.
    let mut block: StatelessBlock = serde_json::from_slice(source.as_ref()).map_err(|e| {
        Error::new(
            ErrorKind::Other,
            format!("failed to deserialize block: {:?}", e),
        )
    })?;

    return parse_stateful_block(block, source, status, genesis);
}

pub async fn parse_stateful_block(
    block: StatefulBlock,
    source: &[u8],
    status: Status,
    genesis: &Genesis,
) -> Result<StatelessBlock> {
    // If src is empty populate bytes with marshalled block.
    if source.len() == 0 {
        let b = serde_json::to_string(&block);
        if b.is_err() {
            log::error!("failed to marshal block: {:?}", block);
            return Err(Error::new(ErrorKind::Other, b.unwrap_err()));
            source = b
        }
    }

    let b = StatelessBlock::new(source, block, status);
    b.id = Id::from_slice_sha256(&Keccak256::digest(&b));

    for tx in block.txs.iter() {
        let resp = tx.init(genesis);
        if resp.is_err() {
            Err(Error::new(ErrorKind::Other, resp.unwrap_err()))
        }
    }
    Ok(block)
}
