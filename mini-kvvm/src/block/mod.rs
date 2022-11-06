pub mod builder;
pub mod state;

use std::io::{Error, ErrorKind, Result};
use std::vec;

use avalanche_types::choices::status;
use avalanche_types::hash;
use avalanche_types::rpcchainvm::concensus::snowman::StatusWriter;
use avalanche_types::rpcchainvm::database;
use avalanche_types::{
    choices::{self, status::Status},
    ids,
};
use derivative::{self, Derivative};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256, Sha3_256};

use crate::chain::tx::tx::Transaction;
use crate::genesis::Genesis;

pub const DATA_LEN: usize = 32;
pub const BLOCKS_LRU_SIZE: usize = 8192;

#[derive(Serialize, Deserialize, Clone, Derivative)]
#[derivative(Debug, Default)]
pub struct Block {
    #[serde(deserialize_with = "ids::must_deserialize_id")]
    pub parent: ids::Id,
    pub height: u64,
    pub timestamp: u64,
    pub data: Vec<u8>,

    #[serde(skip)]
    // current block status.
    pub st: Status,

    #[serde(skip)]
    // encoded block bytes.
    pub bytes: Vec<u8>,

    #[serde(skip)]
    // generated block Id.
    pub id: ids::Id,

    #[derivative(Debug = "ignore")]
    #[serde(skip)]
    pub state: state::State,

    #[serde(skip)]
    pub txs: Vec<Transaction>,

    #[serde(skip)]
    pub children: Vec<Block>,
}

impl Block {
    pub fn new(
        parent: ids::Id,
        height: u64,
        data: &[u8],
        timestamp: u64,
        state: state::State,
    ) -> Self {
        Self {
            parent,
            height,
            data: data.to_vec(),
            timestamp,
            state,

            // set defaults
            id: ids::Id::empty(),
            st: choices::status::Status::Unknown("initialized".to_string()),
            bytes: vec![],
            txs: vec![],
            children: vec![],
        }
    }

    /// Used for validating new txs and some tests
    pub fn new_dummy(timestamp: u64, tx: Transaction, state: state::State) -> Self {
        let mut txs: Vec<Transaction> = Vec::with_capacity(0);
        txs.push(tx);
        Self {
            parent: ids::Id::empty(),
            height: 0,
            data: vec![],
            timestamp,
            state,
            id: ids::Id::empty(),
            st: choices::status::Status::Unknown("dummy".to_string()),
            bytes: vec![],
            txs,
            children: vec![],
        }
    }
}

#[tonic::async_trait]
impl avalanche_types::rpcchainvm::concensus::snowman::Block for Block {
    /// Implements "snowman.Block"
    async fn bytes(&self) -> &[u8] {
        return self.bytes.as_ref();
    }

    /// Helper method which serializes the block to bytes.
    async fn to_bytes(&self) -> Result<Vec<u8>> {
        let block = self.clone();
        let bytes = serde_json::to_vec(&block).map_err(|e| {
            Error::new(
                ErrorKind::InvalidData,
                format!("failed to serialize block to bytes: {:?}", e),
            )
        })?;
        return Ok(bytes);
    }

    /// Implements "snowman.Block"
    async fn height(&self) -> u64 {
        return self.height;
    }

    /// Implements "snowman.Block"
    async fn timestamp(&self) -> u64 {
        return self.timestamp;
    }

    /// Implements "snowman.Block"
    async fn parent(&self) -> ids::Id {
        return self.parent;
    }

    /// Implements "snowman.Block"
    async fn verify(&mut self) -> Result<()> {
        let parent_id = self.parent().await;

        let mut parent_block = self.state.get_block(parent_id).await.map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to verify parent block not found: {}", e.to_string()),
            )
        })?;

        // ensure block height comes right after its parent's height
        let height = self.height().await;
        let parent_height = parent_block.height().await;
        if height > 0 && parent_height + 1 != height {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "failed to verify block invalid height",
            ));
        }
        // ensure block timestamp is after its parent's timestamp
        let timestamp = self.timestamp().await;
        let parent_timestamp = parent_block.timestamp().await;
        if timestamp > 0 && timestamp < parent_timestamp {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "block timestamp: {} is after parents: {}",
                    timestamp, parent_timestamp
                ),
            ));
        }

        let state = self.state.clone();
        state.set_last_accepted(&mut self).await.map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("set last accepted failed: {}", e.to_string()),
            )
        })?;

        parent_block.children.push(self.to_owned());
        state
            .set_verified_block(self.to_owned())
            .await
            .map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("set verified block failed: {}", e.to_string()),
                )
            })?;
        return Ok(());
    }
}

#[tonic::async_trait]
impl avalanche_types::rpcchainvm::concensus::snowman::Decidable for Block {
    /// Implements "snowman.Block.choices.Decidable"
    async fn status(&self) -> Status {
        return self.st.clone();
    }

    /// Implements "snowman.Block.choices.Decidable"
    async fn id(&self) -> ids::Id {
        return self.id;
    }

    /// Implements "snowman.Block.choices.Decidable"
    async fn accept(&mut self) -> Result<()> {
        self.set_status(Status::Accepted).await;

        let block_id = self.id().await;
        let mut block = self.clone();
        // add block to cache
        self.state
            .set_accepted_block(block_id, &block)
            .await
            .map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("failed to add block to cache: {}", e.to_string()),
                )
            })?;

        self.state
            .set_last_accepted(&mut block)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

        // remove this block from verified blocks as it's accepted.
        let _ = self.state.remove_verified_block(block_id).await;

        Ok(())
    }

    /// Implements "snowman.Block.choices.Decidable"
    async fn reject(&mut self) -> Result<()> {
        self.set_status(Status::Rejected).await;

        // remove this block from verified blocks as it's rejected.
        let _ = self.state.remove_verified_block(self.id).await;

        Ok(())
    }
}

#[tonic::async_trait]
impl avalanche_types::rpcchainvm::concensus::snowman::Initializer for Block {
    /// Initializes a block.
    async fn init(&mut self, bytes: &[u8], status: Status) -> Result<()> {
        self.bytes = bytes.to_vec();
        // this is equal to ids.ToID(crypto.Keccak256(b.bytes))
        self.id = ids::Id::from_slice(hash::keccak256(&self.bytes).as_bytes());
        self.st = status;
        Ok(())
    }
}

#[tonic::async_trait]
impl avalanche_types::rpcchainvm::concensus::snowman::StatusWriter for Block {
    /// Sets the blocks status.
    async fn set_status(&mut self, status: Status) {
        self.st = status;
    }
}

#[tokio::test]
async fn test_init() {
    use crate::block::state::State;
    use avalanche_types::rpcchainvm::concensus::snowman::Initializer;

    let state = State::new(database::memdb::Database::new());
    let mut block = Block::new(ids::Id::empty(), 0, &[], 0, state);

    let bytes = serde_json::to_vec(&Genesis::default());
    assert!(bytes.is_ok());

    // let resp = block.init(&bytes.unwrap(),status::Status::Accepted).await;

    // let id = ids::Id::from_slice_with_sha256(&Sha3_256::digest (&bytes.unwrap()));
    // println!("id: {}", id);

    let bytes = "test".as_bytes();

    // let id = ids::Id::from_slice_with_sha256(hash::keccak256(&bytes.unwrap()).as_bytes());
    // println!("id: {}", id);

    // let bytes = &bytes.unwrap();

    println!(
        "id: {}",
        ids::Id::from_slice(hash::keccak256(&bytes).as_bytes())
    );

    // rust "test"
    // [156, 34, 255, 95, 33, 240, 184, 27, 17, 62, 99, 247, 219, 109, 169, 79, 237, 239, 17, 178, 17, 155, 64, 136, 184, 150, 100, 251, 154, 60, 182, 88]

    // golang
    // [156 34 255 95 33 240 184 27 17 62 99 247 219 109 169 79 237 239 17 178 17 155 64 136 184 150 100 251 154 60 182 88]

    // golang test string 2BmJZD6JA79SzY6JqCe93atfmDcv9ECaZ6wxBiQXHBXPxtQsCK

    // qauoHQ9pvsV5485SNKiaxCV52JjAus97p7B5TBrQTJdujAhTw
    // qauoHQ9pvsV5485SNKiaxCV52JjAus97p7B5TBrQTJdujAhTw
    // println!("id: {}", block.id);
    // assert!(resp.is_ok());
}
