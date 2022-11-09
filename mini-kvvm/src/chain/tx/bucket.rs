use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
};

use avalanche_types::ids;
use serde::{Deserialize, Serialize};

use crate::chain::{
    storage::{has_bucket, put_bucket_info},
    tx::decoder::{create_typed_data, MessageValue, Type, TypedData},
};

use super::{
    base,
    decoder::{TD_BLOCK_ID, TD_BUCKET, TD_STRING},
    tx::TransactionType,
    unsigned,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Info {
    pub created: u64,
    pub updated: u64,

    #[serde(deserialize_with = "ids::short::must_deserialize_id")]
    pub raw_bucket: ids::short::Id,

    pub owner: ethereum_types::H160,
}

/// Creates a bucket, which acts as a logical keyspace root.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Tx {
    pub base_tx: base::Tx,
    pub bucket: String,
}

// important to define an unique name of the trait implementation
#[typetag::serde(name = "bucket")]
#[tonic::async_trait]
impl unsigned::Transaction for Tx {
    async fn get_block_id(&self) -> avalanche_types::ids::Id {
        self.base_tx.block_id
    }

    async fn set_block_id(&mut self, id: avalanche_types::ids::Id) {
        self.base_tx.block_id = id;
    }

    async fn get_value(&self) -> Option<Vec<u8>> {
        None
    }

    async fn set_value(&mut self, _value: Vec<u8>) -> std::io::Result<()> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "value is not supported for delete tx",
        ))
    }

    async fn typ(&self) -> TransactionType {
        TransactionType::Set
    }

    async fn execute(&self, txn_ctx: unsigned::TransactionContext) -> Result<()> {
        let mut db = txn_ctx.db;
        // TODO: ensure expected format of bucket

        // ensure bucket does not exist for now update requires an explicit delete tx
        if has_bucket(&db, self.bucket.as_bytes()).await? {
            log::info!("bucket exists: {}", self.bucket);
            return Err(Error::new(
                ErrorKind::AlreadyExists,
                format!("bucket exists: {}", self.bucket),
            ));
        }
        log::info!("bucket exec sender: {}", &txn_ctx.sender);
        let new_info = Info {
            created: txn_ctx.block_time,
            updated: txn_ctx.block_time,
            owner: txn_ctx.sender,
            raw_bucket: ids::short::Id::empty(), // is that right?
        };

        log::info!("bucket info: {}", &txn_ctx.sender);

        return put_bucket_info(&mut db, self.bucket.as_bytes(), new_info, 0).await;
    }

    async fn typed_data(&self) -> TypedData {
        let mut tx_fields: Vec<Type> = Vec::new();
        tx_fields.push(Type {
            name: TD_BUCKET.to_owned(),
            type_: TD_STRING.to_owned(),
        });
        tx_fields.push(Type {
            name: TD_BLOCK_ID.to_owned(),
            type_: TD_STRING.to_owned(),
        });

        let mut message: HashMap<String, MessageValue> = HashMap::with_capacity(1);
        message.insert(
            TD_BUCKET.to_owned(),
            MessageValue::Vec(self.bucket.as_bytes().to_vec()),
        );
        let value = MessageValue::Vec(self.base_tx.block_id.to_vec());
        log::info!("typed_data message value: {:?}", value);
        log::info!("typed_data id vec: {:?}", self.base_tx.block_id.to_vec());
        log::info!("typed_data id: {}", self.base_tx.block_id);
        message.insert(
            TD_BLOCK_ID.to_owned(),
            MessageValue::Vec(self.base_tx.block_id.to_vec()),
        );

        return create_typed_data(super::tx::TransactionType::Bucket, tx_fields, message);
    }
}
