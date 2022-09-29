use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
};

use avalanche_types::ids;
use eip_712::parser::Type as ParserType;
use ethereum_types::H256;
use keccak_hash::keccak;
use radix_fmt::radix;
use serde::{Deserialize, Serialize};
use serde_json::to_value;

use super::{base, bucket, delete, set, tx::TransactionType, unsigned};

pub const TD_STRING: &str = "string";
pub const TD_U64: &str = "u64";
pub const TD_BYTES: &str = "bytes";
pub const TD_BLOCK_ID: &str = "blockId";
pub const TD_BUCKET: &str = "bucket";
pub const TD_KEY: &str = "key";
pub const TD_VALUE: &str = "value";

pub type Type = eip_712::eip712::FieldType;

pub type Types = HashMap<String, Vec<Type>>;

pub type TypedDataMessage = HashMap<String, MessageValue>;

// TypedDataDomain represents the domain part of an EIP-712 message.
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct TypedDataDomain {
    pub name: String,
    pub magic: String,
}

pub fn mini_kvvm_domain(m: u64) -> TypedDataDomain {
    return TypedDataDomain {
        name: "MiniKvvm".to_string(),
        magic: radix(m, 10).to_string(),
    };
}

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
    pub domain: TypedDataDomain,
    pub message: TypedDataMessage,
}

pub fn create_typed_data(
    tx_type: TransactionType,
    tx_fields: Vec<Type>,
    message: TypedDataMessage,
) -> TypedData {
    let mut types = Types::new();
    types.insert("txType".to_owned(), tx_fields);
    types.insert(
        "EIP712Domain".to_owned(),
        vec![
            Type {
                name: "name".to_owned(),
                type_: "string".to_owned(),
            },
            Type {
                name: "magic".to_owned(),
                type_: "uint64".to_owned(),
            },
        ],
    );
    return TypedData {
        types,
        message,
        domain: mini_kvvm_domain(0), // TODO: pass magic
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

pub fn hash_structured_data(typed_data: &TypedData) -> eip_712::error::Result<H256> {
    // EIP-191 compliant
    let prefix = (b"\x19\x01").to_vec();
    let domain = to_value(&typed_data.domain).unwrap();
    let message = to_value(&typed_data.message).unwrap();
    let (domain_hash, data_hash) = (
        eip_712::encode_data(
            &ParserType::Custom("EIP712Domain".into()),
            &typed_data.types,
            &domain,
            None,
        )?,
        eip_712::encode_data(
            &ParserType::Custom(typed_data.primary_type.to_string()),
            &typed_data.types,
            &message,
            None,
        )?,
    );
    let concat = [&prefix[..], &domain_hash[..], &data_hash[..]].concat();
    Ok(keccak(concat))
}
