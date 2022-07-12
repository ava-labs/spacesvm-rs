use std::{
    io::{Error, ErrorKind, Result},
    sync::Arc,
};

use hmac_sha256::Hash;

use avalanche_proto::rpcdb::database_client::DatabaseClient;
use tokio::sync::RwLock;
use tonic::transport::Channel;

use crate::block::Block;

use avalanche_types::ids::{Id, ID_LEN};

const LAST_ACCEPTED_BLOCK_KEY: &[u8] = b"last_accepted";
const BLOCK_PREFIX: u8 = 0x0;
const TX_PREFIX: u8 = 0x1;
const TX_VALUE_PREFIX: u8 = 0x2;
const KEY_PREFIX: u8 = 0x3;
const BALANCE_PREFIX: u8 = 0x4;
pub const BYTE_DELIMITER: u8 = b"/";

pub const DATA_LEN: usize = ID_LEN + 2;

pub struct StateInterior {
    db: DatabaseClient<Channel>,
}

pub struct State {
    inner: Arc<RwLock<StateInterior>>,
}

impl State {
    fn new(db: DatabaseClient<Channel>) -> Self {
        Self {
            inner: StateInterior { db },
        }
    }
}

/// Attempts to persist the last accepted block id value to LAST_ACCEPTED_BLOCK_KEY
/// then also persists the prefix_block_key + block_id = serialized block as the value.
pub async fn set_last_accepted(
    db: Box<dyn avalanche_types::rpcchainvm::database::Database + Send + Sync>,
    block: &Block,
) -> Result<()> {
    let block_id = block
        .initalize()
        .map_err(|e| Error::new(ErrorKind::Other, format!("failed to init block: {:?}", e)))?;
    let resp = db.put(LAST_ACCEPTED_BLOCK_KEY, block_id).await;
    if resp.is_err() {
        return Err(Error::new(ErrorKind::Other, resp.unwrap_err()));
    }

    let value = serde_json::to_vec(&block)?;
    let resp = db.put(prefix_block_key(block_id), value).await;
    if resp.is_err() {
        return Err(Error::new(ErrorKind::Other, resp.unwrap_err()));
    }
    Ok(())
}

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
    let k: [u8; DATA_LEN] = Default::default();
    k[0] = BLOCK_PREFIX;
    k[1] = BYTE_DELIMITER;
    k[2..DATA_LEN].clone_from_slice(block_id);
    &k
}

/// 'TX_PREFIX' + 'BYTE_DELIMITER' + 'tx_id'
fn prefix_tx_key(tx_id: &Id) -> &[u8] {
    let k: [u8; DATA_LEN] = Default::default();
    k[0] = TX_PREFIX;
    k[1] = BYTE_DELIMITER;
    k[2..DATA_LEN].clone_from_slice(tx_id);
    &k
}

/// 'TX_VALUE_PREFIX' + 'BYTE_DELIMITER' + 'tx_id'
fn prefix_tx_value_key(tx_id: &Id) -> &[u8] {
    let k: [u8; DATA_LEN] = Default::default();
    k[0] = TX_VALUE_PREFIX;
    k[1] = BYTE_DELIMITER;
    k[2..DATA_LEN].clone_from_slice(tx_id);
    &k
}

/// 'KEY_PREFIX' + 'BYTE_DELIMITER' + 'key'
fn value_key(key: Hash) -> [u8] {
    let k: [u8; DATA_LEN] = Default::default();
    k[0] = KEY_PREFIX;
    k[1] = BYTE_DELIMITER;
    k[2..DATA_LEN].clone_from_slice(key.k);
    k
}
