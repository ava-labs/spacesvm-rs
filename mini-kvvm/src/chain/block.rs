use std::{
    io::{Error, ErrorKind, Result},
    collections::BTreeMap as Map,
};

use avalanche_types::{
    choices::{self, status::Status},
    ids,
    ids::must_deserialize_id,
    rpcchainvm::{concensus::snowman, database::VersionedDatabase},
};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde::{Serialize, Deserialize};
use sha3::{Digest, Keccak256};
use erased_serde::Deserializer;

use super::{genesis::Genesis, txn::{Transaction, TransactionInterior}};

pub const DATA_LEN: usize = 32;

#[derive(Serialize, Deserialize)]
pub struct StatefulBlock {
    #[serde(deserialize_with = "must_deserialize_id")]
    pub parent: ids::Id,
    height: u64,
    pub timestamp: i64,
    data: Vec<u8>,
    // access_proof: TODO
    txs: Vec<TransactionInterior>,
}

#[derive(Serialize)]
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
    async fn new(genesis: Genesis, parent: Box<dyn snowman::Block>, timestamp: i64) -> Self {
        Self {
            stateful_block: StatefulBlock {
                parent: parent.id().await,
                height: parent.height().await + 1,
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
        return self.bytes.as_ref();
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
        self.bytes = serde_json::to_vec(&self)
            .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;

        self.id = ids::Id::from_slice_with_sha256(&Keccak256::digest(&self.bytes));

        self.t = Utc.timestamp(self.stateful_block.timestamp as i64, 0);

        for tx in self.stateful_block.txs.iter() {
            tx.init(&self.genesis)
                .await
                .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
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
    genesis: Genesis,
) -> Result<StatelessBlock> {
    // Deserialize json bytes to a StatelessBlock.
    // let block: StatefulBlock =  &mut serde_json::from_slice(source.as_ref()).map_err(|e| {
    //     Error::new(
    //         ErrorKind::Other,
    //         format!("failed to deserialize block: {:?}", e),
    //     )
    // })?;

     let block_de =  &mut serde_json::Deserializer::from_slice(source.as_ref());
    let mut formats: Map<&str, Box<dyn Deserializer>> = Map::new();
    formats.insert("json", Box::new(<dyn Deserializer>::erase(block_de)));

    let format = formats.get_mut("json").unwrap();
     erased_serde::deserialize(format).unwrap();

     let block: StatefulBlock = erased_serde::deserialize(format).unwrap();

    return parse_stateful_block(block, source.to_vec(), status, genesis).await;
}

pub async fn parse_stateful_block(
    block: StatefulBlock,
    mut source: Vec<u8>,
    status: Status,
    genesis: Genesis,
) -> Result<StatelessBlock> {
    // If src is empty populate bytes with marshalled block.
    if source.len() == 0 {
        let b = serde_json::to_vec(&block).map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to deserialize block: {:?}", e),
            )
        })?;
        source = b;
    }

    let mut b = StatelessBlock {
        stateful_block: block,
        t: DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc),
        bytes: source,
        st: status,
        genesis: genesis.clone(),
        on_accept_db: None,
        id: ids::Id::empty(),
        children: vec![],
    };

    b.id = ids::Id::from_slice_with_sha256(&Keccak256::digest(&b.bytes));

    for tx in block.txs.iter() {
        tx.init(&genesis.clone())
            .await
            .map_err(|e| Error::new(ErrorKind::Other, format!("failed to init tx: {:?}", e)))?
    }
    Ok(b)
}
