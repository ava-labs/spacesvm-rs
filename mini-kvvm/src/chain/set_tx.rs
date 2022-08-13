use std::io::{Error, ErrorKind, Result};

use serde::{Deserialize, Serialize};

use super::{
    base_tx::BaseTx,
    common_tx::value_hash,
    unsigned_txn::{TransactionContext, UnsignedTransaction},
};

#[derive(Serialize, Deserialize)]
pub struct SetTx {
    base_tx: BaseTx,
    value: Vec<u8>,
}

impl UnsignedTransaction for SetTx {
    fn execute(&self, t: &TransactionContext) -> Result<()> {
        let g = t.genesis;
        if self.value.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidData, format!("value empty")));
        }

        if self.value.len() as u64 > g.max_value_size {
            return Err(Error::new(ErrorKind::InvalidData, format!("value empty")));
        }

        let k = value_hash(&self.value);

        Ok(())
    }
}
