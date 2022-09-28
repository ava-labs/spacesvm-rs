pub mod data;

use std::{
    io::Result,
    sync::{Arc, RwLock},
};

use avalanche_types::ids;

use crate::chain::tx::tx::Transaction;

use self::data::{Data, Entry};

pub struct Mempool {
    data: Arc<RwLock<Data>>,

    /// Channel of length one, which the mempool ensures has an item on
    /// it as long as there is an unissued transaction remaining in [txs].
    pending_tx: crossbeam_channel::Sender<()>,
    pending_rx: crossbeam_channel::Receiver<()>,

    /// Vec of [Tx] that are ready to be gossiped.
    new_txs: Vec<Transaction>,
}

impl Mempool {
    pub fn new(max_size: usize) -> Self {
        // initialize broadcast channel
        let (pending_tx, pending_rx): (
            crossbeam_channel::Sender<()>,
            crossbeam_channel::Receiver<()>,
        ) = crossbeam_channel::bounded(1);
        Self {
            data: Arc::new(RwLock::new(Data::new(max_size))),
            pending_tx,
            pending_rx,
            new_txs: Vec::new(),
        }
    }

    /// Returns a broadcast receiver for the pending tx channel.
    pub fn subscribe_pending(&self) -> crossbeam_channel::Receiver<()> {
        self.pending_rx.clone()
    }

    /// Returns Tx from Id if it exists.
    pub fn get(&self, id: &ids::Id) -> Result<Option<Transaction>> {
        let data = self.data.read().unwrap();
        if let Some(entry) = data.get(id)? {
            if let Some(tx) = entry.tx {
                return Ok(Some(tx));
            }
        }
        Ok(None)
    }

    /// Adds a Tx Entry to mempool and writes to the pending channel.
    pub fn add(&mut self, tx: Transaction) -> Result<bool> {
        let tx_id = &tx.id;

        let mut data = self.data.write().unwrap();
        if data.has(tx_id)? {
            return Ok(false);
        }
        let old_len = data.len();

        let entry = &Entry {
            id: tx_id.to_owned(),
            tx: Some(tx.clone()),
            index: old_len,
        };

        // Optimistically add tx to mempool
        data.push(entry)?;

        self.new_txs.push(tx);

        self.add_pending();

        Ok(true)
    }

    /// Return
    pub fn pop_back(&self) -> Option<Transaction> {
        let mut data = self.data.write().unwrap();
        match data.items.pop_back() {
            Some(entry) => entry.tx,
            None => None,
        }
    }

    /// Returns len of mempool data.
    pub fn len(&self) -> usize {
        let data = self.data.read().unwrap();
        data.len()
    }

    pub fn is_empty(&self) -> bool {
        let data = self.data.read().unwrap();
        data.is_empty()
    }

    /// Returns the vec of transactions ready to gossip and replaces it with an empty vec.
    pub fn new_txs(&mut self) -> Result<Vec<Transaction>> {
        let data = self.data.read().unwrap();

        let mut selected: Vec<Transaction> = Vec::new();

        // It is possible that a block may have been accepted that contains some
        // new transactions before [new_txs] is called.
        for tx in self.new_txs.iter() {
            if data.has(&tx.id)? {
                continue;
            }
            selected.push(tx.to_owned())
        }
        self.new_txs = Vec::new();

        Ok(selected)
    }

    /// Prunes any Ids not included in valid hashes set.
    pub fn prune(&self, valid_hashes: ids::Set) {
        let mut to_remove: Vec<ids::Id> = Vec::with_capacity(valid_hashes.len());

        let data = self.data.write().unwrap();

        for entry in data.items.iter() {
            if let Some(tx) = &entry.tx {
                if !valid_hashes.contains(&tx.id) {
                    to_remove.push(entry.id);
                }
            }
        }
        // drop write lock
        drop(data);

        for id in to_remove.iter() {
            log::debug!("attempting to prune id: {}", id);
            if self.remove(id.to_owned()).is_some() {
                log::debug!("id deleted: {}", id);
            } else {
                log::debug!("failed to delete id: {}: not found", id);
            }
        }
    }

    /// Removes Tx entry from mempool data if it exists.
    pub fn remove(&self, id: ids::Id) -> Option<Transaction> {
        let mut data = self.data.write().unwrap();

        // TODO: try to optimize.
        // find the position of the entry in vec and remove
        match data.items.iter().position(|e| e.id == id) {
            Some(index) => {
                data.items.remove(index);
            }
            None => return None,
        }

        // remove entry from lookup
        match data.lookup.remove(&id) {
            Some(entry) => entry.tx,
            None => {
                // should not happen
                log::error!("failed to remove id: {}: mempool is out of balance", id);
                None
            }
        }
    }

    fn add_pending(&self) {
        self.pending_tx.send(()).unwrap();
    }
}

#[tokio::test]
async fn test_mempool() {
    use crate::chain::tx::{tx::TransactionType, unsigned};

    // init mempool
    let mut mempool = Mempool::new(10);
    let pending_rx = mempool.subscribe_pending();

    // create tx_1
    let tx_data_1 = unsigned::TransactionData {
        typ: TransactionType::Bucket,
        bucket: "foo".to_string(),
        key: "".to_string(),
        value: vec![],
    };
    let resp = tx_data_1.decode();
    assert!(resp.is_ok());
    let utx_1 = resp.unwrap();
    let tx_1 = Transaction::new(utx_1);

    // add tx_1 to mempool
    let tx_1_id = tx_1.id;
    assert_eq!(mempool.add(tx_1).unwrap(), true);
    // drain channel
    let resp = pending_rx.recv();
    assert!(resp.is_ok());
    assert_eq!(mempool.len(), 1);

    // add tx_1 as valid
    let mut valid_txs = ids::new_set(2);
    valid_txs.insert(tx_1_id);

    // create tx_2
    let tx_data_2 = unsigned::TransactionData {
        typ: TransactionType::Bucket,
        bucket: "bar".to_string(),
        key: "".to_string(),
        value: vec![],
    };
    let resp = tx_data_2.decode();
    assert!(resp.is_ok());
    let utx_2 = resp.unwrap();
    let mut tx_2 = Transaction::new(utx_2);
    tx_2.id = ids::Id::from_slice("sup".as_bytes());

    // add tx_2 to mempool
    assert_eq!(mempool.add(tx_2).unwrap(), true);
    assert_eq!(mempool.len(), 2);

    // drain channel
    let resp = pending_rx.recv();
    assert!(resp.is_ok());

    // prune tx_2 as invalid
    mempool.prune(valid_txs);

    // verify one tx entry removed
    assert_eq!(mempool.len(), 1);

    // verify tx_1 exists
    let resp = mempool.get(&tx_1_id);
    assert!(resp.is_ok());

    assert_eq!(resp.unwrap().unwrap().id, tx_1_id);
}