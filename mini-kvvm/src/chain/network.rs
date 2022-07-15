use std::io::Result;

use avalanche_types::ids::Id;
use lru::LruCache;

use crate::chain::txn::TransactionInterior;

const GOSSIPED_TXS_LRU_SIZE: usize = 512;

#[tonic::async_trait]
pub trait PushNetwork {
    async fn send_txs(&self, txs: Vec<TransactionInterior>) -> Result<()>;
    async fn gossip_new_tx(&self, new_tx: Vec<TransactionInterior>) -> Result<()>;
    async fn regossip_tx(&self) -> Result<()>;
}

pub struct Network {
    gossiped_tx: LruCache<Id, ()>,
}

impl Network {
    pub fn new() -> Self {
        let mut cache: LruCache<Id, ()> = LruCache::new(GOSSIPED_TXS_LRU_SIZE);
        Self { gossiped_tx: cache }
    }
}
