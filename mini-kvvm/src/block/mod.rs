pub mod state;

use std::io::{Error, ErrorKind, Result};

use avalanche_types::rpcchainvm::concensus::snowman::StatusWriter;
use avalanche_types::{
    choices::{self, status::Status},
    ids,
};
use derivative::{self, Derivative};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

pub const DATA_LEN: usize = 32;

#[derive(Serialize, Deserialize, Clone, Derivative)]
#[derivative(Debug)]
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

        let parent_block = self.state.get_block(parent_id).await.map_err(|e| {
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
        let block = self.clone();
        self.state.put_block(&block).await.map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("accept block failed: {}", e.to_string()),
            )
        })?;

        self.state
            .set_last_accepted(block_id)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

        let mut verified_blocks = self.state.verified_blocks.write().await;
        // remove this block from verified blocks as it's accepted.
        verified_blocks.remove(&block_id);

        // TODO: add support for versiondb
        // self.state.commit()

        Ok(())
    }

    /// Implements "snowman.Block.choices.Decidable"
    async fn reject(&mut self) -> Result<()> {
        self.set_status(Status::Rejected).await;

        let block = self.clone();
        self.state.put_block(&block).await.map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("accept block failed: {}", e.to_string()),
            )
        })?;

        let mut verified_blocks = self.state.verified_blocks.write().await;
        // remove this block from verified blocks as it's accepted.
        verified_blocks.remove(&block.id);

        Ok(())
    }
}

#[tonic::async_trait]
impl avalanche_types::rpcchainvm::concensus::snowman::Initializer for Block {
    /// Initializes a block.
    async fn init(&mut self, bytes: &[u8], status: Status) -> Result<()> {
        self.bytes = bytes.to_vec();
        self.id = ids::Id::from_slice_with_sha256(&Sha3_256::digest(&self.bytes));
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
async fn genesis_test() {
    use crate::block::state::State;
    use avalanche_types::rpcchainvm::concensus::snowman::*;
    use avalanche_types::rpcchainvm::database::memdb::Database;
    use chrono::{DateTime, NaiveDateTime, Utc};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    let verified_blocks = Arc::new(RwLock::new(HashMap::new()));

    let db = Database::new();
    let state = State::new(db, verified_blocks);

    let genesis_bytes =
        "{\"author\":\"subnet creator\",\"welcome_message\":\"Hello from Rust VM!\"}".as_bytes();

    let timestamp =
        DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc).timestamp() as u64;

    // create genesis block
    let mut block = crate::block::Block::new(ids::Id::empty(), 0, genesis_bytes, timestamp, state);

    let bytes = block.to_bytes().await;
    block
        .init(&bytes.unwrap(), Status::Processing)
        .await
        .unwrap();

    // clone state
    let mut state = block.state.clone();

    // put block
    let resp = state.put_block(&block).await;
    assert!(!resp.is_err());

    // accept block
    let resp = block.accept().await;
    assert!(!resp.is_err());

    // ensure last accepted is genesis
    let resp = state.get_last_accepted().await;
    assert!(!resp.is_err());
    let genesis_id = resp.unwrap();
    assert_eq!(genesis_id, block.id);

    let block_bytes = "{\"author\":\"subnet user\",\"welcome_message\":\"Sup!?\"}".as_bytes();
    let timestamp = Utc::now().timestamp() as u64;
    // create 2nd block
    let mut block = crate::block::Block::new(genesis_id, 1, block_bytes, timestamp, state);

    // initialize 2nd block
    let bytes = block.to_bytes().await;
    block
        .init(&bytes.unwrap(), Status::Processing)
        .await
        .unwrap();

    // put block
    let mut state = block.state.clone();
    let resp = state.put_block(&block).await;
    assert!(!resp.is_err());

    // accept block
    let resp = block.accept().await;
    assert!(!resp.is_err());

    // verify
    assert_eq!(block.verify().await.is_err(), false);
}
