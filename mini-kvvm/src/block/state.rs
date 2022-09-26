use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
    num::NonZeroUsize,
    sync::Arc,
};

use avalanche_types::{
    choices::status::{self, Status},
    ids, rpcchainvm,
};
use lru::LruCache;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use tokio::sync::RwLock;

use crate::chain::{
    self,
    storage::{prefix_block_key, prefix_tx_value_key},
    tx::{set, Transaction},
};

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
    pub db: Arc<RwLock<Box<dyn rpcchainvm::database::Database + Send + Sync>>>,
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
        let cache: LruCache<ids::Id, Block> =
            LruCache::new(NonZeroUsize::new(BLOCKS_LRU_SIZE).unwrap());
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
    pub async fn set_last_accepted(&self, mut block: Block) -> Result<()> {
        let block_id = block.id;

        // persist last_accepted Id to database with fixed key
        let mut db = self.inner.db.write().await;

        log::info!("set_last_accepted key value: {}\n", block_id);
        db.put(LAST_ACCEPTED_BLOCK_KEY, &block_id.to_vec())
            .await
            .map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("failed to put last accepted block: {:?}", e),
                )
            })?;

        for tx in block.txs.iter_mut() {
            if is_set_tx(&tx).await {
                let maybe_set_tx = tx
                    .unsigned_transaction
                    .as_any()
                    .await
                    .downcast_ref::<set::Tx>();

                if maybe_set_tx.is_none() {
                    continue;
                }
                let set_tx = maybe_set_tx.unwrap();
                log::info!(
                    "set_last_accepted put prefix_tx_value_key: {:?}\n",
                    prefix_tx_value_key(&tx.id)
                );
                log::info!(
                    "set_last_accepted put prefix_tx_value_key value: {:?}\n",
                    &set_tx.value
                );
                db.put(&prefix_tx_value_key(&tx.id), &set_tx.value)
                    .await
                    .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
            }
        }

        let bytes = &serde_json::to_vec(&block)?;

        db.put(&prefix_block_key(&block_id), &bytes)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

        Ok(())
    }

    /// Attempts to retrieve the last accepted block and return the corresponding
    /// block Id. If not the key is found returns Id::empty().
    pub async fn get_last_accepted(&self) -> Result<ids::Id> {
        let last_accepted = self.last_accepted.read().await;
        if last_accepted.is_empty() {
            return Ok(*last_accepted);
        }

        let db = self.inner.db.read().await;
        match db.get(LAST_ACCEPTED_BLOCK_KEY).await {
            Ok(value) => {
                let block_id = ids::Id::from_slice(&value);
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
        log::debug!("get block called\n");

        let mut cache = self.lru.cache.write().await;

        // check cache for block
        let cached = cache.get(&block_id);
        if cached.is_some() {
            return Ok(cached.unwrap().to_owned());
        }

        let db = self.inner.db.read().await;

        let block_bytes = db.get(&prefix_block_key(&block_id)).await?;
        let mut block: Block = serde_json::from_slice(&block_bytes)?;

        //  restore the unlinked values associated with all set_tx.value
        for tx in block.txs.iter_mut() {
            if is_set_tx(&tx).await {
                let set_tx = tx
                    .unsigned_transaction
                    .as_any_mut()
                    .await
                    .downcast_mut::<set::Tx>()
                    .unwrap();
                if set_tx.value.is_empty() {
                    continue;
                }

                let tx_id = &ids::Id::from_slice(&set_tx.value);

                let value = db
                    .get(&prefix_tx_value_key(tx_id))
                    .await
                    .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
                set_tx.value = value;
            }
        }

        // parse block inline
        let bytes = &serde_json::to_vec(&block)?;
        block.bytes = bytes.to_vec();
        block.id = ids::Id::from_slice_with_sha256(&Sha3_256::digest(bytes));
        block.st = status::Status::Accepted;
        block.state = self.clone();

        for tx in block.txs.iter_mut() {
            tx.init().await?;
        }

        log::info!("get block found: {:?}", block);
        Ok(block)
    }

    /// Attempts to parse a byte array into a block. If the source is empty
    /// bytes will be marshalled from a default block.
    pub async fn parse_block(&self, mut source: Vec<u8>, status: Status) -> Result<Block> {
        let mut block = Block::default();

        if source.is_empty() {
            source = serde_json::to_vec(&block)?;
        }
        block.bytes = source.to_vec();
        block.id = ids::Id::from_slice_with_sha256(&Sha3_256::digest(source));
        block.st = status;
        block.state = self.clone();

        for tx in block.txs.iter_mut() {
            tx.init().await?;
        }

        Ok(block.to_owned())
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

async fn is_set_tx(tx: &chain::tx::tx::Transaction) -> bool {
    match tx.unsigned_transaction.typ().await {
        chain::tx::tx::TransactionType::Bucket => false,
        chain::tx::tx::TransactionType::Set => true,
        chain::tx::tx::TransactionType::Delete => false,
        chain::tx::tx::TransactionType::Unknown => false,
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
    let mut block = crate::block::Block::new(ids::Id::empty(), 0, genesis_bytes, 0, state);

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

    use crate::block;

    // initialize state
    let verified_blocks = Arc::new(RwLock::new(HashMap::new()));
    let db = Database::new();
    let state = State::new(db, verified_blocks);
    let block = block::Block::new(ids::Id::empty(), 0, &[], 0, state.clone());

    // set
    let resp = state.set_last_accepted(block).await;
    assert!(!resp.is_err());

    // get
    let resp = state.get_last_accepted().await;
    assert!(!resp.is_err());
    assert_eq!(resp.unwrap(), ids::Id::empty())
}
