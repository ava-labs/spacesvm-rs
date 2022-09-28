use std::{
    io::{Error, ErrorKind, Result},
    num::NonZeroUsize,
};

use avalanche_types::ids::{self, Id};
use lru::LruCache;

use crate::{
    chain::{self, vm::Vm},
    vm,
};

const GOSSIPED_TXS_LRU_SIZE: usize = 512;

pub struct Push {
    vm: vm::ChainVm,
    gossiped_tx: LruCache<Id, ()>,
}

impl Push {
    pub fn new(vm: vm::ChainVm) -> Self {
        let cache: LruCache<Id, ()> =
            LruCache::new(NonZeroUsize::new(GOSSIPED_TXS_LRU_SIZE).unwrap());
        Self {
            gossiped_tx: cache,
            vm,
        }
    }

    pub async fn send_txs(&self, txs: Vec<chain::tx::tx::Transaction>) -> Result<()> {
        if txs.is_empty() {
            return Ok(());
        }

        let b = serde_json::to_vec(&txs).map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to marshal txs: {}", e.to_string()),
            )
        })?;

        log::debug!("sending app gossip txs: {} size: {}", txs.len(), b.len());

        let appsender = self
            .vm
            .app_sender
            .clone()
            .expect("appsender should exist after initialize");
        appsender.send_app_gossip(b).await.map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("gossip txs failed: {}", e.to_string()),
            )
        })?;
        Ok(())
    }

    pub async fn gossip_new_txs(&mut self, new_txs: Vec<chain::tx::tx::Transaction>) -> Result<()> {
        let mut txs: Vec<chain::tx::tx::Transaction> = Vec::with_capacity(new_txs.len());

        for tx in new_txs.iter() {
            if self.gossiped_tx.contains(&tx.id) {
                log::debug!("already gossiped skipping id: {}", tx.id);
                continue;
            }

            self.gossiped_tx.put(tx.id, ());

            txs.push(tx.to_owned());
        }

        Ok(())
    }

    /// Triggers "AppGossip" on the pending transactions in the mempool.
    /// "force" is true to re-gossip whether recently gossiped or not.
    pub async fn regossip_txs(&mut self) -> Result<()> {
        let mut txs: Vec<chain::tx::tx::Transaction> = Vec::new();
        let mempool = self.vm.mempool.read().await;

        // Gossip at most the target units of a block at once
        while mempool.len() > 0 {
            match mempool.pop_back() {
                Some(tx) => {
                    // Note: when regossiping, we force resend even though we may have done it
                    // recently.
                    self.gossiped_tx.put(tx.id, ());
                    txs.push(tx);
                }
                None => return Ok(()),
            }
        }

        return self.send_txs(txs).await;
    }

    pub async fn app_gossip(&mut self, node_id: ids::node::Id, message: &[u8]) -> Result<()> {
        log::debug!(
            "appgossip message handler, sender: {} bytes: {:?}",
            node_id,
            message
        );

        let txs: Vec<chain::tx::tx::Transaction> = serde_json::from_slice(&message).unwrap();

        // submit incoming gossip
        log::debug!(
            "appgossip transactions are being submitted txs: {}",
            txs.len()
        );

        self.vm.submit(txs).await.map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!(
                    "appgossip failed to submit txs peer_id: {}: {}",
                    node_id,
                    e.to_string()
                ),
            )
        })?;

        Ok(())
    }
}