use std::{
    any::Any,
    collections::HashMap,
    io::{Error, ErrorKind, Result},
};

use serde::{Deserialize, Serialize};

use crate::chain::{
    storage,
    tx::decoder::{create_typed_data, MessageValue, Type, TypedData},
};

use super::{
    base,
    decoder::{TD_BLOCK_ID, TD_BUCKET, TD_KEY, TD_STRING},
    tx::TransactionType,
    unsigned,
};

/// Removes a key and value from the underlying bucket. No error will return
/// if the key is not found.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Tx {
    pub base_tx: base::Tx,
    pub bucket: String,
    pub key: String,
}

#[typetag::serde]
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
        TransactionType::Delete
    }

    async fn execute(&self, mut txn_ctx: unsigned::TransactionContext) -> Result<()> {
        let db = txn_ctx.db.clone();

        // while we do not use value meta currently we verify it exists.
        let v = storage::get_value_meta(&db, self.bucket.as_bytes(), self.key.as_bytes()).await?;
        if v.is_none() {
            log::info!("value meta key not found: {}", self.key);
            return Ok(());
        }

        storage::delete_bucket_key(&mut txn_ctx.db, self.bucket.as_bytes(), self.key.as_bytes())
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

        Ok(())
    }

    async fn typed_data(&self) -> TypedData {
        let mut tx_fields: Vec<Type> = Vec::new();
        tx_fields.push(Type {
            name: TD_BUCKET.to_owned(),
            typ: TD_STRING.to_owned(),
        });
        tx_fields.push(Type {
            name: TD_BLOCK_ID.to_owned(),
            typ: TD_STRING.to_owned(),
        });
        tx_fields.push(Type {
            name: TD_KEY.to_owned(),
            typ: TD_STRING.to_owned(),
        });

        let mut message: HashMap<String, MessageValue> = HashMap::with_capacity(1);
        message.insert(
            TD_BUCKET.to_owned(),
            MessageValue::Vec(self.bucket.as_bytes().to_vec()),
        );
        message.insert(
            TD_KEY.to_owned(),
            MessageValue::Vec(self.key.as_bytes().to_vec()),
        );
        message.insert(
            TD_BLOCK_ID.to_owned(),
            MessageValue::Vec(self.base_tx.block_id.to_vec()),
        );

        return create_typed_data(super::tx::TransactionType::Delete, tx_fields, message);
    }
}