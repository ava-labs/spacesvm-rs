use serde::{Serialize, Deserialize};

use super::unsigned_txn::UnsignedTransaction;

#[derive(Serialize, Deserialize)]
pub struct SetTx {
    // base_tx: BaseTx;
    value: Vec<u8>,
}

impl UnsignedTransaction for SetTx{

}