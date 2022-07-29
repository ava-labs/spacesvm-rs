pub mod activity;
pub mod base_tx;
pub mod block;
pub mod common;
pub mod common_tx;
pub mod crypto;
pub mod genesis;
pub mod network;
pub mod serde;
pub mod set_tx;
pub mod storage;
pub mod txn;
pub mod unsigned_txn;
pub mod vm;

// use avalanche_types::{choices::status::Status, ids::Id};
// use chrono::Utc;

// use crate::chain::{
//     block::{StatefulBlock, StatelessBlock},
//     genesis::Genesis,
// };

// impl StatelessBlock {
//     fn new(source: &[u8], block: StatefulBlock, status: Status, genesis: Genesis) -> Self {
//         Self {
//             stateful_block: block,
//             t: Utc.timestamp(block.timestamp, 0),
//             bytes: source,
//             st: status,
//             genesis,

//             id: Id::empty(),
//             children: vec![],
//             on_accept_db: None,
//         }
//     }
// }
