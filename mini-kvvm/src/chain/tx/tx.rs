use std::{
    fmt::Debug,
    io::{Error, ErrorKind, Result}, vec,
};

use avalanche_types::{ids, rpcchainvm};
use ethereum_types::Address;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

use crate::{block::Block, chain::storage::set_transaction};

use super::unsigned::TransactionContext;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum TransactionType {
    /// Root namespace.
    Bucket,
    /// Create or update a key/value pair for a bucket.
    Set,
    /// Remove a key.
    Delete,
    /// Used for testing only
    Unknown,
}

impl Default for TransactionType {
    fn default() -> Self {
        TransactionType::Unknown
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub unsigned_transaction: Box<dyn super::unsigned::Transaction + Send + Sync>,
    pub signature: Vec<u8>,

    #[serde(skip)]
    pub digestHash: Vec<u8>,

    #[serde(skip)]
    pub bytes: Vec<u8>,

    #[serde(skip)]
    pub id: ids::Id,

    #[serde(skip)]
    pub size: u64,

    #[serde(skip)]
    pub sender: Address
}

impl Transaction {
    pub fn new(unsigned_transaction: Box<dyn super::unsigned::Transaction + Send + Sync>, signature: Vec<u8>) -> Self {
        Self {
            unsigned_transaction,
            signature,
            digestHash: vec![],
            bytes: vec![],
            id: ids::Id::empty(),
            size: 0,
            sender: Address::zero(),
        }
    }
}

#[typetag::serde]
#[tonic::async_trait]
impl crate::chain::tx::Transaction for Transaction {
    async fn init(&mut self) -> Result<()> {
        let stx =
            serde_json::to_vec(&self).map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
        self.bytes = stx;
        self.id = ids::Id::from_slice_with_sha256(&Sha3_256::digest(&self.bytes));
        self.size = self.bytes.len() as u64;

        Ok(())
    }

    async fn bytes(&self) -> &Vec<u8> {
        return &self.bytes;
    }

    async fn size(&self) -> u64 {
        return self.size;
    }

    async fn id(&self) -> ids::Id {
        return self.id;
    }

    async fn execute(
        &self,
        db: &Box<dyn rpcchainvm::database::Database + Send + Sync>,
        block: Block,
    ) -> Result<()> {
        let txn_ctx = TransactionContext {
            db: db.clone(),
            tx_id: self.id,
            block_time: block.timestamp,
        };

        self.unsigned_transaction
            .execute(txn_ctx)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

        set_transaction(db.clone(), self.to_owned())
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

        Ok(())
    }
}

pub fn new_tx(utx: Box<dyn super::unsigned::Transaction + Send + Sync>, signature: Vec<u8>) -> Transaction {
    return Transaction {
        unsigned_transaction: utx,
        signature,

        // defaults
        digestHash: vec![],
        bytes: vec![],
        id: ids::Id::empty(),
        size: 0,
        sender: Address::zero(),
    };
}
