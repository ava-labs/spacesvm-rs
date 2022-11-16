use std::{
    fmt::Debug,
    io::{Error, ErrorKind, Result},
};

use avalanche_types::{ids::Id, subnet};
use dyn_clone::DynClone;
use ethers_core::types::{transaction::eip712::TypedData, Address};
use serde::{Deserialize, Serialize};

use super::{base, claim, delete, set, tx::TransactionType};

#[typetag::serde(tag = "type")]
#[tonic::async_trait]
pub trait Transaction: Debug + DynClone + Send + Sync {
    async fn get_block_id(&self) -> Id;
    async fn set_block_id(&mut self, id: Id);
    async fn get_value(&self) -> Option<Vec<u8>>;
    async fn set_value(&mut self, value: Vec<u8>) -> Result<()>;
    async fn execute(&self, txn_ctx: TransactionContext) -> Result<()>;
    async fn typed_data(&self) -> Result<TypedData>;
    async fn typ(&self) -> TransactionType;
}

// ref. https://docs.rs/dyn-clone/latest/dyn_clone/macro.clone_trait_object.html
dyn_clone::clone_trait_object!(Transaction);

pub struct TransactionContext {
    pub db: Box<dyn subnet::rpc::database::Database + Send + Sync>,
    pub block_time: u64,
    pub tx_id: Id,
    pub sender: Address,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionData {
    pub typ: TransactionType,
    pub space: String,
    pub key: String,
    pub value: Vec<u8>,
}

impl TransactionData {
    pub fn decode(&self) -> Result<Box<dyn Transaction + Send + Sync>> {
        let tx_param = self.clone();
        match tx_param.typ {
            TransactionType::Claim => Ok(Box::new(claim::Tx {
                base_tx: base::Tx::default(),
                space: tx_param.space,
            })),
            TransactionType::Set => Ok(Box::new(set::Tx {
                base_tx: base::Tx::default(),
                space: tx_param.space,
                key: tx_param.key,
                value: tx_param.value,
            })),
            TransactionType::Delete => Ok(Box::new(delete::Tx {
                base_tx: base::Tx::default(),
                space: tx_param.space,
                key: tx_param.key,
            })),
            TransactionType::Unknown => Err(Error::new(
                ErrorKind::Other,
                "transaction type Unknown is not valid",
            )),
        }
    }
}

#[tokio::test]
async fn test_hash_claim_tx() {
    use ethers_core::types::transaction::eip712::Eip712;

    let tx_data = crate::chain::tx::unsigned::TransactionData {
        typ: TransactionType::Claim,
        space: "kvs".to_string(),
        key: "foo".to_string(),
        value: Vec::new(),
    };
    let resp = tx_data.decode();
    assert!(resp.is_ok());
    let mut utx = resp.unwrap();
    utx.set_block_id(avalanche_types::ids::Id::from_slice("duuuu".as_bytes()))
        .await;
    let typed_data = utx.typed_data().await.unwrap();
    let resp = typed_data.struct_hash();
    assert!(resp.is_ok());
}

#[tokio::test]
async fn test_hash_set_tx() {
    use ethers_core::types::transaction::eip712::Eip712;

    let tx_data = crate::chain::tx::unsigned::TransactionData {
        typ: TransactionType::Set,
        space: "kvs".to_string(),
        key: "foo".to_string(),
        value: "bar".as_bytes().to_vec(),
    };
    let resp = tx_data.decode();
    assert!(resp.is_ok());
    let mut utx = resp.unwrap();
    utx.set_block_id(avalanche_types::ids::Id::from_slice("duuuu".as_bytes()))
        .await;
    let typed_data = utx.typed_data().await.unwrap();
    let resp = typed_data.struct_hash();
    assert!(resp.is_ok());
}
