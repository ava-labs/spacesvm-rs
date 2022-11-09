use std::{
    io::{Error, ErrorKind, Result},
    str,
};

use avalanche_types::{ids, subnet};
use byteorder::{BigEndian, ByteOrder};
use chrono::Utc;

use serde::{Deserialize, Serialize};

use crate::{
    block::{
        state::{self, HASH_LEN},
        Block,
    },
    chain::crypto,
};

use super::tx::{self, bucket, Transaction};

const SHORT_ID_LEN: usize = 20;
const BLOCK_PREFIX: u8 = 0x0;
const TX_PREFIX: u8 = 0x1;
const TX_VALUE_PREFIX: u8 = 0x2;
const INFO_PREFIX: u8 = 0x3;
const KEY_PREFIX: u8 = 0x4;

pub const BYTE_DELIMITER: u8 = b'/';

pub async fn set_transaction(
    mut db: Box<dyn avalanche_types::subnet::rpc::database::Database + Send + Sync>,
    tx: tx::tx::Transaction,
) -> Result<()> {
    let k = prefix_tx_key(&tx.id);
    return db.put(&k, &vec![]).await;
}

pub async fn delete_bucket_key(
    db: &mut Box<dyn avalanche_types::subnet::rpc::database::Database + Send + Sync>,
    bucket: &[u8],
    key: &[u8],
) -> Result<()> {
    match get_bucket_info(db, bucket).await? {
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

pub async fn submit(state: &state::State, txs: &mut Vec<tx::tx::Transaction>) -> Result<()> {
    let now = Utc::now().timestamp() as u64;
    let db = &state.get_db().await;

    for tx in txs.iter_mut() {
        tx.init()
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
        if tx.id().await == ids::Id::empty() {
            return Err(Error::new(ErrorKind::Other, "invalid block id"));
        }
        let dummy_block = Block::new_dummy(now, tx.to_owned(), state.clone());

        tx.execute(&db, &dummy_block)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
    }

    Ok(())
}

pub async fn get_value(
    db: &Box<dyn avalanche_types::subnet::rpc::database::Database + Send + Sync>,
    bucket: &[u8],
    key: &[u8],
) -> Result<Option<Vec<u8>>> {
    let info: Option<tx::bucket::Info> = match get_bucket_info(db, bucket).await {
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

    log::error!("get_value tx_id: {:?}", tx_id);

    let value_key = prefix_tx_value_key(&tx_id);

    let value = db
        .get(&value_key)
        .await
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

    Ok(Some(value))
}

pub async fn get_value_meta(
    db: &Box<dyn avalanche_types::subnet::rpc::database::Database + Send + Sync>,
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

// Attempts to write the value
pub async fn put_bucket_key(
    db: &mut Box<dyn subnet::rpc::database::Database + Send + Sync>,
    bucket: &[u8],
    key: &[u8],
    vmeta: ValueMeta,
) -> Result<()> {
    let resp = get_bucket_info(db, bucket)
        .await
        .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
    if resp.is_none() {
        return Err(Error::new(ErrorKind::NotFound, format!("bucket not found")));
    }

    let k = bucket_value_key(resp.unwrap().raw_bucket, key);
    log::info!("put_value key: {:?}", k);
    let rv_meta = serde_json::to_vec(&vmeta)
        .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;

    return db.put(&k, &rv_meta).await;
}

/// Attempts to store the bucket info by using a key 'bucket_info_key' with the value
/// being serialized bucket info.
pub async fn put_bucket_info(
    db: &mut Box<dyn subnet::rpc::database::Database + Send + Sync>,
    bucket: &[u8],
    mut info: bucket::Info,
    _last_expiry: u64,
) -> Result<()> {
    // If [raw_bucket] is empty, this is a new bucket.
    if info.raw_bucket.is_empty() {
        log::info!("put_bucket_info: new bucket found");
        let r_bucket = raw_bucket(bucket, info.created)
            .await
            .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
        log::info!("put_bucket_info: raw_becket: {:?}", r_bucket);
        info.raw_bucket = r_bucket;
    }
    let value =
        serde_json::to_vec(&info).map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;

    let key = &bucket_info_key(bucket);
    log::info!("put_bucket_info key: {:?}", key);
    log::info!("put_bucket_info value: {:?}", value);

    db.put(key, &value).await
}

// Attempts to get info from a bucket.
pub async fn get_bucket_info(
    db: &Box<dyn subnet::rpc::database::Database + Send + Sync>,
    bucket: &[u8],
) -> Result<Option<bucket::Info>> {
    match db.get(&bucket_info_key(bucket)).await {
        Err(e) => {
            if is_not_found(&e) {
                return Ok(None);
            }
            Err(e)
        }
        Ok(value) => {
            let info: bucket::Info = serde_json::from_slice(&value)
                .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;

            log::info!("get_bucket_info info: {:?}", info);
            Ok(Some(info))
        }
    }
}


pub async fn raw_bucket(bucket: &[u8], block_time: u64) -> Result<ids::short::Id> {
    let mut r: Vec<u8> = Vec::new();
    r.extend_from_slice(bucket);
    r.push(BYTE_DELIMITER);
    r.resize(bucket.len() + 1 + 8, 20);
    BigEndian::write_u64(&mut r[bucket.len() + 1..].to_vec(), block_time);
    let hash = crypto::compute_hash_160(&r);

    Ok(ids::short::Id::from_slice(&hash))
}

/// Returns true if a bucket with the same name already exists.
pub async fn has_bucket(
    db: &Box<dyn subnet::rpc::database::Database + Send + Sync>,
    bucket: &[u8],
) -> Result<bool> {
    db.has(&bucket_info_key(bucket)).await
}

/// 'KEY_PREFIX' + 'BYTE_DELIMITER' + [r_bucket] + 'BYTE_DELIMITER' + [key]
pub fn bucket_value_key(r_bucket: ids::short::Id, key: &[u8]) -> Vec<u8> {
    let mut k: Vec<u8> = Vec::with_capacity(2 + SHORT_ID_LEN + 1 + key.len());
    k.push(KEY_PREFIX);
    k.push(BYTE_DELIMITER);
    k.extend_from_slice(r_bucket.as_ref());
    k.push(BYTE_DELIMITER);
    k.extend_from_slice(key);
    k
}

/// 'INFO_PREFIX' + 'BYTE_DELIMITER' + [bucket]
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
    false
}

#[test]
fn test_prefix() {
    // 'KEY_PREFIX' [4] + 'BYTE_DELIMITER' [47] + [raw_bucket] 0 x 20 + 'BYTE_DELIMITER' [4] + [key] [102, 111, 111]
    assert_eq!(
        bucket_value_key(ids::short::Id::empty(), "foo".as_bytes().to_vec().as_ref()),
        [4, 47, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 47, 102, 111, 111]
    );
    // 'INFO_PREFIX' [3] + 'BYTE_DELIMITER' [47] + 'bucket' [102, 111, 111]
    assert_eq!(
        bucket_info_key("foo".as_bytes().to_vec().as_ref()),
        [3, 47, 102, 111, 111]
    );
    // 'BLOCK_PREFIX' [0] + 'BYTE_DELIMITER' [47] + 'block_id' 0 x 32
    assert_eq!(
        prefix_block_key(&ids::Id::empty()),
        [
            0, 47, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0
        ]
    );
    // 'TX_PREFIX' [1] + 'BYTE_DELIMITER' [47] + 'tx_id' 0 x 32
    assert_eq!(
        prefix_tx_key(&ids::Id::empty()),
        [
            1, 47, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0
        ]
    );
    // 'TX_VALUE_PREFIX' [2] + 'BYTE_DELIMITER' [47] + 'tx_id' 0 x 32
    assert_eq!(
        prefix_tx_value_key(&ids::Id::empty()),
        [
            2, 47, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0
        ]
    )
}

#[tokio::test]
async fn test_raw_bucket() {
    let resp = raw_bucket("kvs".as_bytes(), 0).await;
    assert!(resp.is_ok());
    assert_eq!(
        resp.unwrap(),
        ids::short::Id::from_slice(&[
            28, 196, 105, 174, 208, 254, 253, 229, 213, 10, 32, 26, 54, 105, 74, 64, 119, 12, 91,
            61
        ])
    )
}

#[tokio::test]
async fn test_bucket_info_rt() {
    use super::tx::bucket::Info;
    use ethereum_types::H160;

    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug"),
    );

    let bucket = "kvs".as_bytes();
    let new_info = Info {
        created: 0,
        updated: 1,
        owner: H160::default(),
        raw_bucket: ids::short::Id::empty(),
    };
    let mut db = subnet::rpc::database::memdb::Database::new();
    // put
    let resp = put_bucket_info(&mut db, &bucket, new_info, 2).await;
    assert!(resp.is_ok());

    // get
    let resp = get_bucket_info(&mut db, &bucket).await;
    assert!(resp.as_ref().is_ok());
    assert!(resp.as_ref().unwrap().is_some());
    let info = resp.unwrap().unwrap();
    assert_eq!(
        info.raw_bucket,
        ids::short::Id::from_slice(&[
            230, 185, 125, 2, 27, 125, 127, 228, 212, 79, 188, 214, 107, 248, 146, 237, 254, 112,
            153, 17
        ])
    );
    assert_eq!(info.updated, 1);
}
