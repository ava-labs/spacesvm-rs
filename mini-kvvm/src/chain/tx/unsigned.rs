use std::{
    any::Any,
    fmt::Debug,
    io::{Error, ErrorKind, Result},
};

use avalanche_types::{ids::Id, rpcchainvm};
use dyn_clone::DynClone;
use serde::{Deserialize, Serialize};

use crate::chain::tx::decoder::TypedData;

use super::{base, bucket, delete, set, tx::TransactionType};

#[typetag::serde]
#[tonic::async_trait]
pub trait Transaction: Debug + DynClone + Send + Sync {
    async fn get_block_id(&self) -> Id;
    async fn set_block_id(&mut self, id: Id);
    async fn execute(&self, txn_ctx: TransactionContext) -> Result<()>;
    async fn typed_data(&self) -> TypedData;
    async fn typ(&self) -> TransactionType;
    async fn as_any(&self) -> &(dyn Any + Send + Sync);
    async fn as_any_mut(&mut self) -> &mut (dyn Any + Send + Sync);
}

// ref. https://docs.rs/dyn-clone/latest/dyn_clone/macro.clone_trait_object.html
dyn_clone::clone_trait_object!(Transaction);

pub struct TransactionContext {
    pub db: Box<dyn rpcchainvm::database::Database + Send + Sync>,
    pub block_time: u64,
    pub tx_id: Id,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionData {
    pub typ: TransactionType,
    pub bucket: String,
    pub key: String,
    pub value: Vec<u8>,
}

impl TransactionData {
    pub fn decode(&self) -> Result<Box<dyn Transaction + Send + Sync>> {
        let tx_param = self.clone();
        match tx_param.typ {
            TransactionType::Bucket => Ok(Box::new(bucket::Tx {
                base_tx: base::Tx::default(),
                bucket: tx_param.bucket,
            })),
            TransactionType::Set => Ok(Box::new(set::Tx {
                base_tx: base::Tx::default(),
                bucket: tx_param.bucket,
                key: tx_param.key,
                value: tx_param.value,
            })),
            TransactionType::Delete => Ok(Box::new(delete::Tx {
                base_tx: base::Tx::default(),
                bucket: tx_param.bucket,
                key: tx_param.key,
            })),
            TransactionType::Unknown => Err(Error::new(
                ErrorKind::Other,
                "transaction type Unknown is not valid",
            )),
        }
    }
}
