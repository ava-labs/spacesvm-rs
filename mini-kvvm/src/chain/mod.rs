pub mod activity;
pub mod block;
pub mod genesis;
pub mod storage;
pub mod txn;
pub mod unsigned_txn;
pub mod vm;

use avalanche_types::{choices::status::Status, ids::Id};
use chrono::Utc;

use crate::chain::block::{StatefulBlock, StatelessBlock};

impl StatelessBlock {
    fn new(source: &[u8], block: StatefulBlock, status: Status) -> Self {
        Self {
            stateful_block: block,
            t: Utc.timestamp(block.timestamp, 0),
            bytes: source,
            st: status,

            id: Id::empty(),
            children: vec![],
            on_accept_db: None,
        }
    }
}
