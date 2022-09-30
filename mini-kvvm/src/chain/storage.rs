use std::{
    io::{Error, ErrorKind, Result},
    str,
};

use avalanche_types::{ids, rpcchainvm};
use byteorder::{BigEndian, ByteOrder};
use chrono::Utc;
use ethereum_types::H160;
use serde::{Deserialize, Serialize};

use crate::block::{state::HASH_LEN, Block};

use super::tx::{self, bucket, Transaction};

const SHORT_ID_LEN: usize = 20;
const BLOCK_PREFIX: u8 = 0x0;
const TX_PREFIX: u8 = 0x1;
const TX_VALUE_PREFIX: u8 = 0x2;
const INFO_PREFIX: u8 = 0x3;
const KEY_PREFIX: u8 = 0x4;
const _BALANCE_PREFIX: u8 = 0x5;

pub const BYTE_DELIMITER: u8 = b'/';

pub async fn set_transaction(
    mut db: Box<dyn avalanche_types::rpcchainvm::database::Database + Send + Sync>,
    tx: tx::tx::Transaction,
) -> Result<()> {
    let k = prefix_tx_key(&tx.id);
    return db.put(&k, &vec![]).await;
}

pub async fn delete_bucket_key(
    db: &mut Box<dyn avalanche_types::rpcchainvm::database::Database + Send + Sync>,
    bucket: &[u8],
    key: &[u8],
) -> Result<()> {
    match get_bucket_info(&db, bucket).await? {
        None => Err(Error::new(
            ErrorKind::InvalidData,
            format!("bucket not found"),
        )),
        Some(info) => {
            db.delete(&bucket_value_key(info.raw_bucket, key))
                .await
                .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
            Ok(())
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct ValueMeta {
    pub size: u64,
    #[serde(deserialize_with = "ids::must_deserialize_id")]
    pub tx_id: ids::Id,

    pub created: u64,
    pub updated: u64,
}

pub async fn submit(
    db: &Box<dyn avalanche_types::rpcchainvm::database::Database + Send + Sync>,
    txs: &mut Vec<tx::tx::Transaction>,
) -> Result<()> {
    let now = Utc::now().timestamp() as u64;
    for tx in txs.iter_mut() {
        tx.init()
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
        if tx.id().await == ids::Id::empty() {
            return Err(Error::new(ErrorKind::Other, "invalid block id"));
        }
        let dummy_block = Block::new_dummy(now, tx.to_owned());

        tx.execute(db.to_owned(), dummy_block)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
    }
    Ok(())
}

pub async fn get_value(
    db: &Box<dyn avalanche_types::rpcchainvm::database::Database + Send + Sync>,
    bucket: &[u8],
    key: &[u8],
) -> Result<Option<Vec<u8>>> {
    let info: Option<tx::bucket::Info> = match get_bucket_info(&db, bucket).await {
        Ok(info) => info,
        Err(e) => {
            if is_not_found(&e) {
                return Ok(None);
            }
            return Err(e);
        }
    };
    if info.is_none() {
        return Ok(None);
    }

    let value = db
        .get(&bucket_value_key(info.unwrap().raw_bucket, key))
        .await
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

    let vmeta: ValueMeta = serde_json::from_slice(&value)
        .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;

    let tx_id = vmeta.tx_id;

    log::debug!("get_value tx_id: {}", tx_id.to_string());

    let value_key = prefix_tx_value_key(&tx_id);

    log::debug!("get_value prefix_tx_value_key: {:?}", value_key);

    let value = db
        .get(&value_key)
        .await
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

    log::debug!("get_value prefix_tx_value_key: found");
    Ok(Some(value))
}

pub async fn get_value_meta(
    db: &Box<dyn avalanche_types::rpcchainvm::database::Database + Send + Sync>,
    bucket: &[u8],
    key: &[u8],
) -> Result<Option<ValueMeta>> {
    match get_bucket_info(&db, bucket).await? {
        None => Ok(None),
        Some(info) => match db.get(&bucket_value_key(info.raw_bucket, key)).await {
            Err(e) => {
                if is_not_found(&e) {
                    return Ok(None);
                }
                Err(e)
            }
            Ok(value) => {
                let vmeta: ValueMeta = serde_json::from_slice(&value)
                    .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
                Ok(Some(vmeta))
            }
        },
    }
}

pub async fn put_bucket_key(
    db: &mut Box<dyn rpcchainvm::database::Database + Send + Sync>,
    bucket: &[u8],
    key: &[u8],
    vmeta: ValueMeta,
) -> Result<()> {
    let resp = get_bucket_info(&db, bucket)
        .await
        .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
    if resp.is_none() {
        return Err(Error::new(ErrorKind::NotFound, format!("bucket not found")));
    }

    let k = bucket_value_key(resp.unwrap().raw_bucket, key);
    let rv_meta = serde_json::to_vec(&vmeta)
        .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;

    return db.put(&k, &rv_meta).await;
}

pub async fn put_bucket_info(
    db: &mut Box<dyn rpcchainvm::database::Database + Send + Sync>,
    bucket: &[u8],
    mut info: bucket::Info,
    _last_expiry: u64,
) -> Result<()> {
    log::debug!("put_bucket_info called: {:?}\n", bucket);

    // If [raw_bucket] is empty, this is a new space.
    if info.raw_bucket.is_empty() {
        let r_bucket = raw_bucket(bucket, info.created)
            .await
            .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;

        info.raw_bucket = r_bucket;
    }
    let value =
        serde_json::to_vec(&info).map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;

    db.put(&bucket_info_key(bucket), &value).await
}

pub async fn get_bucket_info(
    db: &Box<dyn rpcchainvm::database::Database + Send + Sync>,
    bucket: &[u8],
) -> Result<Option<bucket::Info>> {
    log::debug!("get_bucket_info called: {:?}\n", bucket);

    match db.get(&bucket_info_key(bucket)).await {
        Err(e) => {
            if is_not_found(&e) {
                return Ok(None);
            }
            Err(e)
        }
        Ok(value) => {
            log::debug!("get_bucket_info value: {:?}\n", value);
            let info: bucket::Info = serde_json::from_slice(&value)
                .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
            log::debug!("get_bucket_info info: {:?}\n", info);
            Ok(Some(info))
        }
    }
}

pub async fn raw_bucket(bucket: &[u8], block_time: u64) -> Result<ids::short::Id> {
    let mut r: Vec<u8> = Vec::from(bucket);
    let bucket_len = bucket.len();
    r.resize(20, 0);
    r[bucket_len] = BYTE_DELIMITER;
    BigEndian::write_u64(&mut r[bucket_len + 1..], block_time);
    let hash = H160::from_slice(&r);

    Ok(ids::short::Id::from_slice(hash.as_bytes()))
}

/// Returns true if a bucket with the same name already exists.
pub async fn has_bucket(
    db: &Box<dyn rpcchainvm::database::Database + Send + Sync>,
    bucket: &[u8],
) -> Result<bool> {
    return db.has(&bucket_info_key(bucket)).await;
}

/// [keyPrefix] + [delimiter] + [raw_bucket] + [delimiter] + [key]
pub fn bucket_value_key(r_bucket: ids::short::Id, key: &[u8]) -> Vec<u8> {
    let mut k: Vec<u8> = Vec::with_capacity(2 + SHORT_ID_LEN + 1 + key.len());
    k.push(KEY_PREFIX);
    k.push(BYTE_DELIMITER);
    k.extend_from_slice(r_bucket.as_ref());
    k.push(BYTE_DELIMITER);
    k.extend_from_slice(key);
    k
}

/// 'INFO_PREFIX' + 'BYTE_DELIMITER' + 'bucket'
pub fn bucket_info_key(bucket: &[u8]) -> Vec<u8> {
    let mut k: Vec<u8> = Vec::with_capacity(bucket.len() + 2);
    k.push(INFO_PREFIX);
    k.push(BYTE_DELIMITER);
    k.extend_from_slice(bucket);
    k
}

/// 'BLOCK_PREFIX' + 'BYTE_DELIMITER' + 'block_id'
pub fn prefix_block_key(block_id: &ids::Id) -> Vec<u8> {
    let mut k: Vec<u8> = Vec::with_capacity(HASH_LEN);
    k.push(BLOCK_PREFIX);
    k.push(BYTE_DELIMITER);
    k.extend_from_slice(&block_id.to_vec());
    k
}

/// 'TX_PREFIX' + 'BYTE_DELIMITER' + 'tx_id'
pub fn prefix_tx_key(tx_id: &ids::Id) -> Vec<u8> {
    let mut k: Vec<u8> = Vec::with_capacity(HASH_LEN);
    k.push(TX_PREFIX);
    k.push(BYTE_DELIMITER);
    k.extend_from_slice(&tx_id.to_vec());
    k
}

/// 'TX_VALUE_PREFIX' + 'BYTE_DELIMITER' + 'tx_id'
pub fn prefix_tx_value_key(tx_id: &ids::Id) -> Vec<u8> {
    let mut k: Vec<u8> = Vec::with_capacity(HASH_LEN);
    k.push(TX_VALUE_PREFIX);
    k.push(BYTE_DELIMITER);
    k.extend_from_slice(&tx_id.to_vec());
    k
}

/// Returns false if the io::Error is ErrorKind::NotFound and contains a string "not found".
pub fn is_not_found(error: &Error) -> bool {
    if error.kind() == ErrorKind::NotFound && error.to_string().contains("not found") {
        return true;
    }
    return false;
}
