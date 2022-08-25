use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
    sync::Arc,
};

use avalanche_types::{choices::status::Status, ids, rpcchainvm};
use lru::LruCache;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use tokio::sync::RwLock;

use super::Block;

const BLOCKS_LRU_SIZE: usize = 8192;
const LAST_ACCEPTED_BLOCK_KEY: &[u8] = b"last_accepted";
pub const BYTE_DELIMITER: &[u8] = b"/";
pub const HASH_LEN: usize = ids::ID_LEN + 2;

#[derive(Serialize, Deserialize, Default)]
pub struct ValueMeta {
    size: u64,
    tx_id: ids::Id,
}

#[derive(Serialize, Deserialize, Default)]
pub struct BlockWrapper {
    block: Vec<u8>,
    status: Status,
}

#[derive(Default, Clone)]
pub struct State {
    pub verified_blocks: Arc<RwLock<HashMap<ids::Id, Block>>>,
    pub lru: Lru,
    pub inner: InnerState,
    pub last_accepted: Arc<RwLock<ids::Id>>,
}

#[derive(Clone, Debug)]
pub struct Lru {
    cache: Arc<RwLock<LruCache<ids::Id, Block>>>,
}

impl Default for Lru {
    fn default() -> Lru {
        Lru {
            cache: Arc::new(RwLock::new(LruCache::unbounded())),
        }
    }
}

#[derive(Clone)]
pub struct InnerState {
    db: Arc<RwLock<Box<dyn rpcchainvm::database::Database + Send + Sync>>>,
}

impl Default for InnerState {
    // Memdb by default
    fn default() -> InnerState {
        InnerState {
            db: Arc::new(RwLock::new(rpcchainvm::database::memdb::Database::new())),
        }
    }
}

impl State {
    pub fn new(
        db: Box<dyn rpcchainvm::database::Database + Send + Sync>,
        verified_blocks: Arc<RwLock<HashMap<ids::Id, Block>>>,
    ) -> Self {
        let cache: LruCache<ids::Id, Block> = LruCache::new(BLOCKS_LRU_SIZE);
        return Self {
            inner: InnerState {
                db: Arc::new(RwLock::new(db)),
            },
            lru: Lru {
                cache: Arc::new(RwLock::new(cache)),
            },
            verified_blocks,
            last_accepted: Arc::new(RwLock::new(ids::Id::empty())),
        };
    }

    /// Persists last accepted block Id into both cache and database.
    pub async fn set_last_accepted(&self, block_id: ids::Id) -> Result<()> {
        let mut last_accepted = self.last_accepted.write().await;
        // check memory for match
        if *last_accepted == block_id {
            return Ok(());
        }

        // put last_accepted Id to memory
        *last_accepted = block_id;

        // persist last_accepted Id to database with fixed key
        let mut db = self.inner.db.write().await;
        db.put(LAST_ACCEPTED_BLOCK_KEY, &last_accepted.to_vec())
            .await
            .map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("failed to put last accepted block: {:?}", e),
                )
            })?;

        Ok(())
    }

    /// Attempts to retrieve the last accepted block and return the corresponding
    /// block Id. If not the key is found returns Id::empty().
    pub async fn get_last_accepted(&self) -> Result<ids::Id> {
        let mut last_accepted = self.last_accepted.write().await;
        if last_accepted.is_empty() {
            return Ok(*last_accepted);
        }

        let db = self.inner.db.read().await;
        match db.get(LAST_ACCEPTED_BLOCK_KEY).await {
            Ok(value) => {
                let block_id = ids::Id::from_slice(&value);
                *last_accepted = block_id;
                Ok(block_id)
            }
            Err(e) => {
                if e.kind() == ErrorKind::NotFound && e.to_string().contains("not found") {
                    return Ok(ids::Id::empty());
                }
                return Err(e);
            }
        }
    }

    /// Attempts to return block from cache given a valid block id.
    /// If the cache is not hit check the database.
    pub async fn get_block(&mut self, block_id: ids::Id) -> Result<Block> {
        let db = self.inner.db.read().await;

        let mut cache = self.lru.cache.write().await;

        // check cache for block
        let cached = cache.get(&block_id);
        if cached.is_some() {
            return Ok(cached.unwrap().to_owned());
        }

        let wrapped_block_bytes = db.get(&block_id.to_vec()).await?;

        // first decode/unmarshal the block wrapper so we can have status and block bytes
        let wrapped_block: BlockWrapper = serde_json::from_slice(&wrapped_block_bytes)?;

        // now decode/marshal the actual block bytes to block
        let mut block: Block = serde_json::from_slice(&wrapped_block.block)?;
        block.bytes = wrapped_block.block.to_vec();
        block.id = ids::Id::from_slice_with_sha256(&Sha3_256::digest(wrapped_block.block.to_vec()));
        block.st = wrapped_block.status;

        cache.put(block.id, block.to_owned());

        Ok(block.to_owned())
    }

    /// Attempts to return block from state given a valid block id.
    pub async fn put_block(&mut self, block: &Block) -> Result<()> {
        let mut db = self.inner.db.write().await;
        let mut cache = self.lru.cache.write().await;

        let wrapped_block = BlockWrapper {
            block: block.to_owned().bytes,
            status: block.to_owned().st,
        };
        // encode block wrapper to its byte representation
        let wrapped_bytes = serde_json::to_vec(&wrapped_block)?;

        let block_copy = block.clone();

        // put actual block to cache, so we can directly fetch it from cache
        cache.put(block_copy.id, block.to_owned());

        // put wrapped block into database
        db.put(&block_copy.id.to_vec(), &wrapped_bytes)
            .await
            .map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("failed to put last accepted block: {:?}", e),
                )
            })?;

        Ok(())
    }

    /// Checks if the last accepted block key exists and returns true if has a value.
    pub async fn has_last_accepted(&self) -> Result<bool> {
        let db = self.inner.db.read().await;

        match db.has(LAST_ACCEPTED_BLOCK_KEY).await {
            Ok(found) => Ok(found),
            Err(e) => Err(Error::new(ErrorKind::Other, e.to_string())),
        }
    }
}

#[tokio::test]
async fn block_test() {
    use avalanche_types::rpcchainvm::{concensus::snowman::*, database::memdb::Database};

    use crate::block::state::State;

    // initialize state
    let verified_blocks = Arc::new(RwLock::new(HashMap::new()));
    let db = Database::new();
    let state = State::new(db, verified_blocks);
    let genesis_bytes =
        "{\"author\":\"subnet creator\",\"welcome_message\":\"Hello from Rust VM!\"}".as_bytes();

    // create genesis block
    let mut block = crate::block::Block::new(ids::Id::empty(), 0, genesis_bytes, 0, state).await;

    // initialize block
    let bytes = block.to_bytes().await;
    block
        .init(&bytes.unwrap(), Status::Processing)
        .await
        .unwrap();

    // write block
    let mut state = block.state.clone();
    let resp = state.put_block(&block).await;
    assert!(!resp.is_err());

    // verify cache was populated then release read lock
    {
        let lru = state.lru.cache.read().await;
        assert_eq!(lru.len(), 1);
    }

    // get block by id from cache
    let mut state = block.state.clone();
    let resp = state.get_block(block.id().await).await;
    assert!(!resp.is_err());
}

#[tokio::test]
async fn last_accepted_test() {
    use avalanche_types::rpcchainvm::database::memdb::Database;

    // initialize state
    let verified_blocks = Arc::new(RwLock::new(HashMap::new()));
    let db = Database::new();
    let state = State::new(db, verified_blocks);

    // set
    let resp = state.set_last_accepted(ids::Id::empty()).await;
    assert!(!resp.is_err());

    // get
    let resp = state.get_last_accepted().await;
    assert!(!resp.is_err());
    assert_eq!(resp.unwrap(), ids::Id::empty())
}
