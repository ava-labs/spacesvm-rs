use std::{
    fmt::{self, Debug},
    io::{Error, ErrorKind, Result},
};

use avalanche_types::{hash, ids, rpcchainvm};
use ethereum_types::Address;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

use crate::{block::Block, chain::crypto, chain::storage::set_transaction};

use super::{decoder, unsigned::TransactionContext};

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

impl fmt::Display for TransactionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TransactionType::Bucket => write!(f, "bucket"),
            TransactionType::Set => write!(f, "set"),
            TransactionType::Delete => write!(f, "delete"),
            TransactionType::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub unsigned_transaction: Box<dyn super::unsigned::Transaction + Send + Sync>,
    pub signature: Vec<u8>,

    #[serde(skip)]
    pub digest_hash: Vec<u8>,

    #[serde(skip)]
    pub bytes: Vec<u8>,

    #[serde(skip)]
    pub id: ids::Id,

    #[serde(skip)]
    pub size: u64,

    #[serde(skip)]
    pub sender: Address,
}

impl Transaction {
    pub fn new(
        unsigned_transaction: Box<dyn super::unsigned::Transaction + Send + Sync>,
        signature: Vec<u8>,
    ) -> Self {
        Self {
            unsigned_transaction,
            signature,
            digest_hash: vec![],
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
        let digest_hash =
            decoder::hash_structured_data(&self.unsigned_transaction.typed_data().await)?;
        let sender = crypto::derive_sender(digest_hash.as_bytes(), &self.signature)?;
        self.bytes = stx;
        // self.id = ids::Id::from_slice_with_sha256(&Sha3_256::digest(&self.bytes));
        self.id = ids::Id::from_slice(hash::keccak256(&self.bytes).as_bytes());

        self.size = self.bytes.len() as u64;
        self.digest_hash = digest_hash.as_bytes().to_vec();
        self.sender = crypto::public_to_address(&sender);

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
            sender: self.sender,
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

pub fn new_tx(
    utx: Box<dyn super::unsigned::Transaction + Send + Sync>,
    signature: Vec<u8>,
) -> Transaction {
    return Transaction {
        unsigned_transaction: utx,
        signature,

        // defaults
        digest_hash: vec![],
        bytes: vec![],
        id: ids::Id::empty(),
        size: 0,
        sender: Address::zero(),
    };
}

#[tokio::test]
async fn test_init() {
    let secret_key = secp256k1::SecretKey::new(&mut secp256k1::rand::thread_rng());

    let tx_data = crate::chain::tx::unsigned::TransactionData {
        typ: TransactionType::Bucket,
        bucket: "bar".to_string(),
        key: "".to_string(),
        value: vec![],
    };
    let resp = tx_data.decode();
    assert!(resp.is_ok());
    let utx = resp.unwrap();
    let dh = decoder::hash_structured_data(&utx.typed_data().await).unwrap();
    let sig = crypto::sign(&dh.as_bytes(), &secret_key).unwrap();
    let tx = Transaction::new(utx, sig);

    let tx_bytes = serde_json::to_string(&tx).unwrap();

    println!("json : {}", tx_bytes);

    let txs: Transaction = serde_json::from_slice(&tx_bytes.as_bytes())
        .map_err(|e| println!("failed to serialize {}", e.to_string()))
        .unwrap();
}

// #[tokio::test]
// async fn test_init2() {
//      let message = r#"{
//   "unsigned_transaction": {
//     "type": {
//       "base_tx": {
//         "block_id": "11111111111111111111111111111111LpoYY"
//       },
//       "bucket": "bar"
//     }
//   },
//   "signature": [
//     48,
//     166,
//     110,
//     163,
//     227,
//     13,
//     202,
//     64,
//     12,
//     102,
//     46,
//     185,
//     104,
//     153,
//     82,
//     223,
//     216,
//     64,
//     45,
//     207,
//     220,
//     102,
//     193,
//     5,
//     0,
//     118,
//     64,
//     74,
//     18,
//     121,
//     128,
//     3,
//     32,
//     8,
//     95,
//     212,
//     170,
//     178,
//     93,
//     139,
//     187,
//     5,
//     23,
//     2,
//     83,
//     77,
//     142,
//     45,
//     239,
//     28,
//     87,
//     115,
//     6,
//     135,
//     151,
//     116,
//     157,
//     88,
//     253,
//     155,
//     158,
//     82,
//     51,
//     102,
//     28
//   ]
// }"#;

//     // let tx_bytes = serde_json::to_string(&tx).unwrap();

//     // println!("json : {}", tx_bytes );

//     let txs: Transaction = serde_json::from_slice(&message.as_bytes())
//         .map_err(|e| println!("failed to serialize {}", e.to_string()))
//         .unwrap();
// }
