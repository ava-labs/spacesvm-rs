use std::{any::Any, collections::HashMap, io::Result};

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

    pub owner: ethereum_types::Address,
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

    async fn execute(&self, txn_ctx: unsigned::TransactionContext) -> Result<()> {
        let mut db = txn_ctx.db;
        // TODO: ensure expected format of bucket

        // ensure bucket does not exist for now update requires an explicit delete tx
        if has_bucket(&db, self.bucket.as_bytes()).await? {
            log::info!("bucket exists: {}", self.bucket);
            return Ok(());
        }
        let new_info = Info {
            created: txn_ctx.block_time,
            updated: txn_ctx.block_time,
            owner: txn_ctx.sender,
            raw_bucket: ids::short::Id::empty(), // is that right?
        };

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
        message.insert(
            TD_BLOCK_ID.to_owned(),
            MessageValue::Vec(self.base_tx.block_id.to_vec()),
        );

        return create_typed_data(super::tx::TransactionType::Bucket, tx_fields, message);
    }
}
