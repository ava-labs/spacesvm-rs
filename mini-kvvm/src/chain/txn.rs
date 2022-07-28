use std::{
    fmt::Debug,
    io::{Error, ErrorKind, Result},
};

use crate::chain::{
    crypto,
    genesis::Genesis,
    storage::set_transaction,
    unsigned_txn::{TransactionContext, UnsignedTransaction},
    vm::Context,
};
use avalanche_types::ids;
use avalanche_types::{ids::Id, rpcchainvm::database::Database};
use ethereum_types::Address;
use hex::ToHex;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

use super::{activity::Activity, block::StatelessBlock};

#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionInterior {
    unsigned_transaction: Box<dyn UnsignedTransaction + Send + Sync>,
    signature: Vec<u8>,

    #[serde(skip)]
    digest_hash: Vec<u8>,

    #[serde(skip)]
    bytes: Vec<u8>,

    #[serde(skip)]
    id: Id,

    #[serde(skip)]
    size: u64,

    #[serde(skip)]
    sender: Address,
}

#[tonic::async_trait]
#[typetag::serde]
pub trait Transaction: Debug + Send + Sync {
    async fn init(&self, genesis: &Genesis) -> Result<()>;
    async fn bytes(&self) -> Vec<u8>;
    async fn size(&self) -> u64;
    async fn id(&self) -> Id;
    async fn digest_hash(&self) -> Vec<u8>;
    async fn sender(&self) -> Address;
    async fn execute(
        &self,
        genesis: Genesis,
        database: Box<dyn Database + Send + Sync>,
        block: StatelessBlock,
        ctx: Context,
    ) -> Result<()>;
    async fn activity(&self) -> &Activity;
}

#[typetag::serde]
#[tonic::async_trait]
impl Transaction for TransactionInterior {
    async fn init(&self, genesis: &Genesis) -> Result<()> {
        let stx =
            serde_json::to_vec(&self).map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
        self.bytes = stx;

        self.id = Id::from_slice_with_sha256(&Keccak256::digest(&self.bytes));

        // Compute digest hash
        let digest_hash = digest_hash(self.unsigned_transaction)
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
        self.digest_hash = digest_hash.to_vec();

        // Derive sender
        let public_key = crypto::derive_sender(digest_hash, self.signature.as_slice())
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
        self.sender = crypto::public_to_address(&public_key);

        self.size = self.bytes.len() as u64;

        Ok(())
    }

    async fn bytes(&self) -> Vec<u8> {
        return self.bytes;
    }

    async fn size(&self) -> u64 {
        return self.size;
    }

    async fn id(&self) -> Id {
        return self.id;
    }

    async fn digest_hash(&self) -> Vec<u8> {
        return self.digest_hash;
    }

    async fn sender(&self) -> Address {
        return self.sender;
    }

    async fn execute(
        &self,
        genesis: Genesis,
        database: Box<dyn Database + Send + Sync>,
        block: StatelessBlock,
        ctx: Context,
    ) -> Result<()> {
        self.unsigned_transaction
            .execute_base(&genesis)
            .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;

        if !ctx
            .recent_block_ids
            .contains(&self.unsigned_transaction.get_block_id())
        {
            return Err(Error::new(ErrorKind::Other, "invalid blockId"));
        }
        if !ctx.recent_tx_ids.contains(&self.id) {
            return Err(Error::new(ErrorKind::Other, "duplicate transaction"));
        }

        // TODO Ensure sender has balance
        let tx_ctx = &TransactionContext {
            genesis,
            database,
            block_time: block.stateful_block.timestamp as u64,
            tx_id: self.id,
            sender: self.sender,
        };

        self.unsigned_transaction
            .execute(tx_ctx)
            .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;

        set_transaction(database, Box::new(*self))
            .await
            .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;

        Ok(())
    }

    async fn activity(&self) -> &Activity {
        let activity = self.unsigned_transaction.activity();
        activity.sender = self.sender.encode_hex();
        activity.tx_id = self.id;
        &activity
    }
}

pub fn new_tx(utx: Box<dyn UnsignedTransaction + Send + Sync>, sig: &[u8]) -> &TransactionInterior {
    return &TransactionInterior {
        unsigned_transaction: utx,
        signature: sig.to_vec(),

        digest_hash: vec![],
        bytes: vec![],
        id: ids::Id::empty(),
        size: 0,
        sender: ethereum_types::H160::zero(),
    };
}

pub fn digest_hash(utx: Box<dyn UnsignedTransaction>) -> Result<&'static [u8]> {
    return crate::tdata::digest_hash(utx.typed_data());
}
