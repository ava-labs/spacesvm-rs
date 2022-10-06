use std::{
    io::{Error, ErrorKind, Result},
    num::NonZeroUsize,
    sync::Arc, time::Duration,
};

use avalanche_types::ids::{self, Id};
use lru::LruCache;
use tokio::{sync::RwLock, time::sleep};
use crossbeam_channel::TryRecvError;

use crate::{chain, vm};

const GOSSIPED_TXS_LRU_SIZE: usize = 512;

// TODO: make configurable
const GOSSIP_INTERVAL: Duration = Duration::from_secs(1);
const REGOSSIP_INTERVAL: Duration = Duration::from_secs(30);

pub struct Push {
    gossiped_tx: LruCache<Id, ()>,

    vm_inner: Arc<RwLock<vm::inner::Inner>>,
}

impl Push {
    pub fn new(vm_inner: Arc<RwLock<vm::inner::Inner>>) -> Self {
        let cache: LruCache<Id, ()> =
            LruCache::new(NonZeroUsize::new(GOSSIPED_TXS_LRU_SIZE).unwrap());
        Self {
            vm_inner,
            gossiped_tx: cache,
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
        let vm = self.vm_inner.read().await;
        let appsender = vm
            .app_sender
            .as_ref()
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "app_sender not found"))?;

        appsender.send_app_gossip(b).await.map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("gossip txs failed: {}", e.to_string()),
            )
        })?;
        Ok(())
    }

    pub async fn gossip_new_txs(&mut self) -> Result<()> {
        let mut inner = self.vm_inner.write().await;
        let new_txs = inner.mempool.new_txs().unwrap();
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
        let vm = self.vm_inner.read().await;

        let mempool = &vm.mempool;

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

        let mut txs: Vec<chain::tx::tx::Transaction> = serde_json::from_slice(&message).unwrap();

        // submit incoming gossip
        log::debug!(
            "appgossip transactions are being submitted txs: {}",
            txs.len()
        );

        let mut vm = self.vm_inner.write().await;

        chain::storage::submit(&vm.state, &mut txs)
            .await
            .map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!(
                        "appgossip failed to submit txs peer_id: {}: {}",
                        node_id,
                        e.to_string()
                    ),
                )
            })?;

        for tx in txs.iter_mut() {
            let _ = vm
                .mempool
                .add(tx.to_owned())
                .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
        }

        Ok(())
    }

      pub async fn regossip(&mut self) {
        log::debug!("starting regossip loop");

        let inner = self.vm_inner.read().await;
        let stop_ch = inner.stop_rx.clone();
        drop(inner);

        while stop_ch.try_recv() == Err(TryRecvError::Empty) {
            sleep(REGOSSIP_INTERVAL).await;
            log::info!("tick regossip");

            let _ = self.regossip_txs().await;
        }

        log::debug!("shutdown regossip loop");
    }

    pub async fn gossip(
        &mut self,
    ) {
        log::info!("starting gossip loops");
        let inner = self.vm_inner.read().await;
        let stop_ch = inner.stop_rx.clone();
        drop(inner);

        while stop_ch.try_recv() == Err(TryRecvError::Empty) {
            sleep(GOSSIP_INTERVAL).await;
            log::info!("tick gossip");

            let _ = self.gossip_new_txs().await;
        }
    }
}

