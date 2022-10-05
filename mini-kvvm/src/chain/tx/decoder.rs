use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
};

use avalanche_types::ids;
use serde::{Deserialize, Serialize};

use super::{base, bucket, delete, set, tx::TransactionType, unsigned};

pub const TD_STRING: &str = "string";
pub const TD_U64: &str = "u64";
pub const TD_BYTES: &str = "bytes";
pub const TD_BLOCK_ID: &str = "blockId";
pub const TD_BUCKET: &str = "bucket";
pub const TD_KEY: &str = "key";
pub const TD_VALUE: &str = "value";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Type {
    pub name: String,
    pub typ: String,
}

pub type Types = HashMap<String, Vec<Type>>;

pub type TypedDataMessage = HashMap<String, MessageValue>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageValue {
    Vec(Vec<u8>),
    Bytes(Vec<u8>),
}

impl MessageValue {
    pub fn to_string(self) -> String {
        match self {
            MessageValue::Vec(v) => String::from_utf8_lossy(&v).to_string(),
            MessageValue::Bytes(v) => String::from_utf8_lossy(&v).to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TypedData {
    pub types: Types,
    pub primary_type: TransactionType,
    pub message: TypedDataMessage,
}

pub fn create_typed_data(
    tx_type: TransactionType,
    tx_fields: Vec<Type>,
    message: TypedDataMessage,
) -> TypedData {
    let mut types = Types::new();
    types.insert("txType".to_owned(), tx_fields);
    return TypedData {
        types,
        message,
        primary_type: tx_type,
    };
}

impl TypedData {
    // Attempts to return the base tx from typed data.
    pub fn parse_base_tx(&self) -> Result<base::Tx> {
        let r_block_id = self
            .get_typed_message(TD_BLOCK_ID.to_owned())
            .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
        let block_id = ids::Id::from_slice(r_block_id.as_bytes());

        Ok(base::Tx { block_id })
    }

    // Attempts to return and unsigned transaction from typed data.
    pub fn parse_typed_data(&self) -> Result<Box<dyn unsigned::Transaction + Send + Sync>> {
        let base_tx = self.parse_base_tx().map_err(|e| {
            Error::new(
                ErrorKind::InvalidData,
                format!("failed to parse base tx: {:?}", e),
            )
        })?;

        match self.primary_type {
            TransactionType::Bucket => {
                let bucket = self
                    .get_typed_message(TD_BUCKET.to_owned())
                    .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
                return Ok(Box::new(bucket::Tx {
                    base_tx,
                    bucket: bucket.to_owned(),
                }));
            }

            TransactionType::Set => {
                let bucket = self
                    .get_typed_message(TD_BUCKET.to_owned())
                    .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
                let key = self
                    .get_typed_message(TD_KEY.to_owned())
                    .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
                let value = self
                    .get_typed_message(TD_VALUE.to_owned())
                    .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
                return Ok(Box::new(set::Tx {
                    base_tx,
                    bucket: bucket.to_owned(),
                    key: key.to_owned(),
                    value: value.as_bytes().to_vec(),
                }));
            }

            TransactionType::Delete => {
                let bucket = self
                    .get_typed_message(TD_BUCKET.to_owned())
                    .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
                let key = self
                    .get_typed_message(TD_KEY.to_owned())
                    .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
                return Ok(Box::new(delete::Tx {
                    base_tx,
                    bucket: bucket.to_owned(),
                    key: key.to_owned(),
                }));
            }
            TransactionType::Unknown => Err(Error::new(
                ErrorKind::Other,
                "transaction type Unknown is not valid",
            )),
        }
    }

    pub fn get_typed_message(&self, key: String) -> Result<String> {
        match self.message.get(&key) {
            Some(value) => Ok(value.to_owned().to_string()),
            None => Err(Error::new(
                ErrorKind::NotFound,
                format!("typed data key missing: {:?}", key),
            )),
        }
    }
}
