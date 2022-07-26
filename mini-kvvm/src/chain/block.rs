use std::io::{Error, ErrorKind, Result};

use avalanche_types::{
    choices::{status::Status, self},
    ids,
    ids::must_deserialize_id,
    rpcchainvm::{concensus::snowman, database::VersionedDatabase},
};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sha3::Keccak256;

use super::{genesis::Genesis, txn::Transaction, vm};

pub const DATA_LEN: usize = 32;

#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct StatefulBlock {
    #[serde(deserialize_with = "must_deserialize_id")]
    pub parent: ids::Id,
    height: u64,
    pub timestamp: u64,
    data: Vec<u8>,
    // access_proof: TODO
    txs: Vec<Box<dyn Transaction + Send + Sync>>,
}

#[derive(Serialize, Deserialize)]
pub struct StatelessBlock {
    pub stateful_block: StatefulBlock,

    #[serde(skip)]
    id: ids::Id,

    #[serde(skip)]
    st: Status,

    #[serde(skip)]
    t: DateTime<Utc>,

    #[serde(skip)]
    bytes: Vec<u8>,

    #[serde(skip)]
    children: Vec<StatelessBlock>,

    #[serde(skip)]
    on_accept_db: Option<Box<dyn VersionedDatabase + Send + Sync>>,

    #[serde(skip)]
    genesis: Genesis,
}

impl StatelessBlock {
    async fn new(
        genesis: Genesis,
        parent: Box<dyn snowman::Block>,
        timestamp: u64,
        ctx: vm::Context,
    ) -> Self {
        Self {
            stateful_block: StatefulBlock {
                parent: parent.id().await,
                height: parent.height().await,
                timestamp,
                data: vec![],
                txs: vec![],
            },
            id: ids::Id::empty(),
            st: choices::status::Status::Processing,
            t: DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc),
            bytes: vec![],
            children: vec![],
            on_accept_db: None,
            genesis,
        }
    }
}

#[tonic::async_trait]
impl avalanche_types::rpcchainvm::concensus::snowman::Block for StatelessBlock {
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
        return self.t;
    }

    /// implements "snowman.Block"
    async fn parent(&self) -> ids::Id {
        return self.stateful_block.parent;
    }

    /// implements "snowman.Block"
    async fn verify(&self) -> Result<()> {
        Ok(())
    }
}

#[tonic::async_trait]
impl avalanche_types::rpcchainvm::concensus::snowman::Decidable for StatelessBlock {
    /// implements "snowman.Block.choices.Decidable"
    async fn status(&self) -> Status {
        return self.st;
    }

    /// implements "snowman.Block.choices.Decidable"
    async fn id(&self) -> ids::Id {
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

#[tonic::async_trait]
impl Initializer for StatelessBlock {
    /// Initializes a stateless block.
    async fn init(&self) -> Result<()> {
        let bytes = serde_json::to_string(&self);
        if bytes.is_err() {
            return Err(Error::new(ErrorKind::Other, bytes.unwrap_err()));
        }
        self.bytes = bytes.unwrap();
        self.id = ids::Id::from_slice_sha256(&Keccak256::digest(&self.bytes));

        self.t = Utc.timestamp(self.stateful_block.timestamp, 0);
        for tx in self.stateful_block.txs.iter() {
            let resp = tx.init(&self.genesis);
            if resp.is_err() {
                Err(Error::new(ErrorKind::Other, resp.unwrap_err()))
            }
        }
        Ok(())
    }
}

#[tonic::async_trait]
pub trait Initializer {
    async fn init(&self) -> Result<()>;
}

pub async fn parse_block(
    source: &[u8],
    status: Status,
    genesis: &Genesis,
) -> Result<StatelessBlock> {
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

    let b = StatelessBlock::new(source, block, status, genesis);
    b.id = ids::Id::from_slice_sha256(&Keccak256::digest(&b));

    for tx in block.txs.iter() {
        let resp = tx.init(genesis);
        if resp.is_err() {
            Err(Error::new(ErrorKind::Other, resp.unwrap_err()))
        }
    }
    Ok(block)
}
