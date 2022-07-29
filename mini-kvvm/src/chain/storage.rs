use std::{
    io::{Error, ErrorKind, Result},
    sync::Arc,
};

use avalanche_proto::rpcdb::database_client::DatabaseClient;
use avalanche_types::ids::{Id, ID_LEN};
// use hmac_sha256::Hash;
use tokio::sync::RwLock;
use tonic::transport::Channel;

use crate::chain::{
    block::{StatefulBlock, StatelessBlock},
    common::Hash,
    txn::Transaction,
};

const LAST_ACCEPTED_BLOCK_KEY: &[u8] = b"last_accepted";
const BLOCK_PREFIX: u8 = 0x0;
const TX_PREFIX: u8 = 0x1;
const TX_VALUE_PREFIX: u8 = 0x2;
const KEY_PREFIX: u8 = 0x3;
const BALANCE_PREFIX: u8 = 0x4;
pub const BYTE_DELIMITER: &[u8] = b"/";

pub const HASH_LEN: usize = ID_LEN + 2;

pub struct StateInterior {
    db: DatabaseClient<Channel>,
}

pub struct State {
    inner: Arc<RwLock<StateInterior>>,
}

impl State {
    fn new(db: DatabaseClient<Channel>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(StateInterior { db })),
        }
    }
}

/// Attempts to persist the last accepted block id value to LAST_ACCEPTED_BLOCK_KEY
/// then also persists the prefix_block_key + block_id = serialized block as the value.
pub async fn set_last_accepted(
    mut db: Box<dyn avalanche_types::rpcchainvm::database::Database + Send + Sync>,
    block: &StatelessBlock,
) -> Result<()> {
    let block_id = block.id;
    db.put(LAST_ACCEPTED_BLOCK_KEY, &block_id.to_vec())
        .await
        .map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to put last accepted block: {:?}", e),
            )
        })?;

        let og_txs = 

    let stateful_bytes = serde_json::to_vec(&block).map_err(|e| {
        Error::new(
            ErrorKind::Other,
            format!("failed deserialize block: {:?}", e),
        )
    })?;

    db.put(prefix_block_key(&block_id), &stateful_bytes)
        .await
        .map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to put last accepted block: {:?}", e),
            )
        })?;

        block.stateful_block.txs = 

    Ok(())
}


pub fn link_values(mut db: Box<dyn avalanche_types::rpcchainvm::database::Database + Send + Sync>, block: &StatelessBlock) -> Result<Vec<Box<dyn Transaction>>> {

}

/// Attempts to retrieve the last accepted block and return the corresponding
/// block Id. If not the key is found returns Id::empty().
pub async fn get_last_accepted(
    db: Box<dyn avalanche_types::rpcchainvm::database::Database + Send + Sync>,
) -> Result<Id> {
    match db.get(LAST_ACCEPTED_BLOCK_KEY).await {
        Ok(value) => Ok(Id::from_slice(&value)),
        Err(e) => {
            if e.kind() == ErrorKind::Other && e.to_string().contains("not found") {
                return Ok(Id::empty());
            }
            return Err(e);
        }
    }
}

/// Attempts to return block from state given a valid block id.
pub async fn get_block(
    db: Box<dyn avalanche_types::rpcchainvm::database::Database + Send + Sync>,
    block_id: Id,
) -> Result<Option<StatefulBlock>> {
    match db.get(prefix_block_key(&block_id)).await {
        Ok(value) => Ok(Some(serde_json::from_slice(&value)?)),
        Err(e) => {
            if e.kind() == ErrorKind::Other && e.to_string().contains("not found") {
                return Ok(None);
            }
            return Err(e);
        }
    }
}

/// Checks if the last accepted block key exists and returns true if has a value.
pub async fn has_last_accepted(
    db: Box<dyn avalanche_types::rpcchainvm::database::Database + Send + Sync>,
) -> Result<bool> {
    match db.has(LAST_ACCEPTED_BLOCK_KEY).await {
        Ok(found) => Ok(found),
        Err(e) => Err(Error::new(ErrorKind::Other, e.to_string())),
    }
}

/// 'BLOCK_PREFIX' + 'BYTE_DELIMITER' + 'block_id'
fn prefix_block_key(block_id: &Id) -> &[u8] {
    let mut k: Vec<u8> = Vec::with_capacity(HASH_LEN);
    k.push(BLOCK_PREFIX);
    k.extend_from_slice(BYTE_DELIMITER);
    k.extend_from_slice(&block_id.to_vec());
    &k
}

/// 'TX_PREFIX' + 'BYTE_DELIMITER' + 'tx_id'
fn prefix_tx_key(tx_id: &Id) -> &[u8] {
    let mut k: Vec<u8> = Vec::with_capacity(HASH_LEN);
    k.push(TX_PREFIX);
    k.extend_from_slice(BYTE_DELIMITER);
    k.extend_from_slice(&tx_id.to_vec());
    &k
}

/// 'TX_VALUE_PREFIX' + 'BYTE_DELIMITER' + 'tx_id'
fn prefix_tx_value_key(tx_id: &Id) -> &[u8] {
    let mut k: Vec<u8> = Vec::with_capacity(HASH_LEN);
    k.push(TX_VALUE_PREFIX);
    k.extend_from_slice(BYTE_DELIMITER);
    k.extend_from_slice(&tx_id.to_vec());
    &k
}

/// 'KEY_PREFIX' + 'BYTE_DELIMITER' + 'key'
fn value_key(key: Hash) -> Vec<u8> {
    let mut k: Vec<u8> = Vec::with_capacity(HASH_LEN);
    k.push(KEY_PREFIX);
    k.extend_from_slice(BYTE_DELIMITER);
    k.extend_from_slice(key.as_bytes());
    k
}

pub async fn set_transaction(
    mut db: Box<dyn avalanche_types::rpcchainvm::database::Database + Send + Sync>,
    txn: Box<dyn Transaction>,
) -> Result<()> {
    let id = txn.id().await;
    let k = prefix_tx_key(&id);
    return db.put(k, k).await;
}

// #[test]
// fn test_value_key() {
//     let key = Hash::hash("hello".as_bytes());

//     value_key(key)
// }
