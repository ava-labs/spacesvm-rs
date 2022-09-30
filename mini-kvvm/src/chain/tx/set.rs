use std::{
    any::Any,
    collections::HashMap,
    io::{Error, ErrorKind},
};

use hex::ToHex;
use serde::{Deserialize, Serialize};
use sha3::Digest;

use crate::chain::{
    storage::{self, get_bucket_info, put_bucket_info, put_bucket_key, ValueMeta},
    tx::decoder::{create_typed_data, MessageValue, Type, TypedData},
};

use super::{
    base,
    decoder::{TD_BLOCK_ID, TD_BUCKET, TD_BYTES, TD_KEY, TD_STRING, TD_VALUE},
    tx::TransactionType,
    unsigned::{self},
};

/// 0x + hex-encoded hash
const HASH_LEN: usize = 66;

/// Performs a write against the logical keyspace. If the key exists
/// the value will be overwritten. The root bucket must be created
/// in advance.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Tx {
    pub base_tx: base::Tx,

    /// Base namespace for the key value pair.
    pub bucket: String,

    /// Parsed from the given input, with its bucket removed.
    pub key: String,

    /// Written as the key-value pair to the storage. If a previous value
    /// exists, it is overwritten.
    pub value: Vec<u8>,
}

#[tonic::async_trait]
#[typetag::serde]
impl unsigned::Transaction for Tx {
    async fn get_block_id(&self) -> avalanche_types::ids::Id {
        self.base_tx.block_id
    }

    async fn set_block_id(&mut self, id: avalanche_types::ids::Id) {
        self.base_tx.block_id = id;
    }

    /// Provides downcast support for the trait object.
    /// ref. https://doc.rust-lang.org/std/any/index.html
    async fn as_any(&self) -> &(dyn Any + Send + Sync) {
        self
    }

    /// Provides downcast support for the trait object.
    /// ref. https://doc.rust-lang.org/std/any/index.html
    async fn as_any_mut(&mut self) -> &mut (dyn Any + Send + Sync) {
        self
    }

    async fn typ(&self) -> TransactionType {
        TransactionType::Set
    }

    async fn execute(&self, txn_ctx: unsigned::TransactionContext) -> std::io::Result<()> {
        let mut db = txn_ctx.db;
        // TODO: ensure expected format of bucket, key and value

        if self.key.len() == HASH_LEN {
            let hash = value_hash(&self.value);
            if self.key != hash {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("invalid key: {} expected: {}", self.key, hash),
                ));
            }
        }

        let value_size = self.value.len() as u64;

        let mut new_vmeta = ValueMeta {
            size: value_size,
            tx_id: txn_ctx.tx_id,
            created: txn_ctx.block_time,
            updated: txn_ctx.block_time,
        };

        let v = storage::get_value_meta(&db, self.bucket.as_bytes(), self.key.as_bytes()).await?;
        if v.is_none() {
            new_vmeta.created = txn_ctx.block_time;
        } else {
            new_vmeta.created = v.unwrap().created;
        }

        log::debug!(
            "set_tx execute put_bucket_key: bucket: {} key: {} value_meta: {:?}\n",
            self.bucket,
            self.key,
            new_vmeta
        );

        put_bucket_key(
            &mut db,
            self.bucket.as_bytes(),
            self.key.as_bytes(),
            new_vmeta,
        )
        .await
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

        let info = get_bucket_info(&db, self.bucket.as_bytes())
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
        if info.is_none() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("bucket not found: {}", self.bucket),
            ));
        }

        put_bucket_info(&mut db, self.bucket.as_bytes(), info.unwrap(), 0)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

        Ok(())
    }

    async fn typed_data(&self) -> TypedData {
        let mut tx_fields: Vec<Type> = Vec::with_capacity(2);
        tx_fields.push(Type {
            name: TD_BUCKET.to_owned(),
            type_: TD_STRING.to_owned(),
        });
        tx_fields.push(Type {
            name: TD_BLOCK_ID.to_owned(),
            type_: TD_STRING.to_owned(),
        });
        tx_fields.push(Type {
            name: TD_VALUE.to_owned(),
            type_: TD_BYTES.to_owned(),
        });

        let mut message = HashMap::with_capacity(3);
        message.insert(
            TD_BUCKET.to_owned(),
            MessageValue::Vec(self.bucket.as_bytes().to_vec()),
        );
        message.insert(
            TD_KEY.to_owned(),
            MessageValue::Vec(self.key.as_bytes().to_vec()),
        );
        message.insert(
            TD_VALUE.to_owned(),
            MessageValue::Bytes(self.value.encode_hex::<String>().as_bytes().to_vec()),
        );
        message.insert(
            TD_BLOCK_ID.to_owned(),
            MessageValue::Vec(self.base_tx.block_id.to_vec()),
        );

        return create_typed_data(super::tx::TransactionType::Set, tx_fields, message);
    }
}

fn value_hash(value: &[u8]) -> String {
    let mut hasher = sha3::Sha3_256::new();
    hasher.update(value);
    let result = hasher.finalize();
    hex::encode(&result[..])
}

#[tokio::test]
async fn service_test() {
    use super::unsigned::Transaction;

    // set tx bucket not found
    let db = avalanche_types::rpcchainvm::database::memdb::Database::new();
    let ut_ctx = unsigned::TransactionContext {
        db,
        block_time: 0,
        tx_id: avalanche_types::ids::Id::empty(),
    };
    let tx = Tx {
        base_tx: base::Tx::default(),
        bucket: "kvs".to_string(),
        key: "foo".to_string(),
        value: "bar".as_bytes().to_vec(),
    };
    let resp = tx.execute(ut_ctx).await;
    assert!(resp.unwrap_err().to_string().contains("bucket not found"));

    // update key value
    let db = avalanche_types::rpcchainvm::database::memdb::Database::new();
    let ut_ctx = unsigned::TransactionContext {
        db: db.clone(),
        block_time: 0,
        tx_id: avalanche_types::ids::Id::empty(),
    };
    let tx = crate::chain::tx::bucket::Tx {
        base_tx: base::Tx::default(),
        bucket: "kvs".to_string(),
    };
    let resp = tx.execute(ut_ctx).await;
    assert!(resp.is_ok());

    let ut_ctx = unsigned::TransactionContext {
        db: db.clone(),
        block_time: 0,
        tx_id: avalanche_types::ids::Id::empty(),
    };
    let tx = Tx {
        base_tx: base::Tx::default(),
        bucket: "kvs".to_string(),
        key: "foo".to_string(),
        value: "bar".as_bytes().to_vec(),
    };
    let resp = tx.execute(ut_ctx).await;
    assert!(resp.is_ok());

    let ut_ctx = unsigned::TransactionContext {
        db: db.clone(),
        block_time: 0,
        tx_id: avalanche_types::ids::Id::empty(),
    };
    let tx = Tx {
        base_tx: base::Tx::default(),
        bucket: "kvs".to_string(),
        key: "bar".to_string(),
        value: "bar".as_bytes().to_vec(),
    };
    let resp = tx.execute(ut_ctx).await;
    assert!(resp.is_ok());
}
