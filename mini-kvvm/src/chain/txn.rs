use std::io::{Error, ErrorKind, Result};

use avalanche_types::{ids::Id, rpcchainvm::database::Database};

use chrono::{DateTime, Utc};
use ethereum_types::Address;
use serde::{Deserialize, Serialize};
use sha3::Keccak256;

use crate::chain::{genesis::Genesis, unsigned_txn::UnsignedTransaction, vm::Context};

use super::{activity::Activity, block::StatelessBlock};

#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct TransactionInterior {
    pub unsigned_transaction: Box<dyn UnsignedTransaction>,
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

pub trait Transaction {
    fn init(&self, genesis: &Genesis) -> Result<()>;
    fn bytes(&self) -> Vec<u8>;
    fn size(&self) -> u64;
    fn id(&self) -> Id;
    fn digest_hash(&self) -> Vec<u8>;
    fn sender(&self) -> Address;
    fn execute(
        &self,
        genesis: Genesis,
        database: Box<dyn Database>,
        block: StatelessBlock,
        ctx: Context,
    ) -> Result<()>;
    fn activity(&self) -> &Activity;
}

impl Transaction for TransactionInterior {
    fn init(&self, genesis: &Genesis) -> Result<()> {
        let stx = serde_json::to_string(&self);
        if stx.is_err() {
            return Err(Error::new(ErrorKind::Other, stx.unwrap_err()));
        }
        self.bytes = stx.unwrap();
        self.id = Id::from_slice_sha256(&Keccak256::digest(&self.bytes));

        self.t = Utc.timestamp(self.s, 0);

        //   for tx in block.txs.iter() {
        Ok(())
    }

    fn bytes(&self) -> Vec<u8> {
        return self.bytes;
    }

    fn size(&self) -> u64 {
        return self.size;
    }

    fn id(&self) -> Id {
        return self.id;
    }

    fn digest_hash(&self) -> Vec<u8> {
        return self.digest_hash;
    }

    fn sender(&self) -> Address {
        return self.sender;
    }

    fn execute(
        &self,
        genesis: Genesis,
        database: Box<dyn Database>,
        block: StatelessBlock,
        ctx: Context,
    ) -> Result<()> {
        Ok(())
    }

    fn activity(&self) -> &Activity {}
}

pub struct TransactionContext {
    genesis: Genesis,
    database: Box<dyn Database + Send + Sync>,
    block_time: u64,
    tx_id: Id,
    sender: Address,
}

pub fn new_tx(utx: Box<dyn UnsignedTransaction>, sig: &[u8]) -> &TransactionInterior {
    return &TransactionInterior {
        unsigned_transaction: utx,
        signature: sig.to_vec(),

        digest_hash: (),
        bytes: (),
        id: (),
        size: (),
        sender: (),
    };
}
