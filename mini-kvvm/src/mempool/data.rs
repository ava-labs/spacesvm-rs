use std::{
    collections::{HashMap, VecDeque},
    io::Result,
};

use avalanche_types::ids;

use crate::chain::tx::tx::Transaction;

/// In memory representation of mempool.
#[derive(Debug)]
pub struct Data {
    pub items: VecDeque<Entry>,
    pub lookup: HashMap<ids::Id, Entry>,
    /// Vec of [Tx] that are ready to be gossiped.
    pub new_txs: Vec<Transaction>,
}

/// Object representing a transaction stored in mempool.
#[derive(Debug, Default, Clone)]
pub struct Entry {
    pub id: ids::Id,
    pub tx: Option<Transaction>,
    pub index: usize,
}

impl Data {
    pub fn new(max_size: usize) -> Self {
        Self {
            items: VecDeque::with_capacity(max_size),
            lookup: HashMap::new(),
            new_txs: Vec::new(),
        }
    }

    pub fn push_new_tx(&mut self, tx: Transaction) {
        self.new_txs.push(tx);
        log::info!("new tx added")
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.len() == 0
    }

    pub fn swap(&mut self, i: usize, j: usize) {
        self.items.swap(i, j);
        self.items[i].index = i;
        self.items[j].index = j;
    }

    pub fn push(&mut self, entry: &Entry) -> Result<()> {
        if self.has(&entry.id)? {
            return Ok(());
        }
        self.items.push_front(entry.to_owned());

        // insert key only if it does not already exist.
        self.lookup.insert(entry.id, entry.to_owned());

        Ok(())
    }

    pub fn pop(&mut self) -> Result<Option<Entry>> {
        Ok(self.items.pop_front())
    }

    pub fn pop_back(&mut self) -> Result<Option<Entry>> {
        Ok(self.items.pop_back())
    }

    pub fn get(&self, id: &ids::Id) -> Result<Option<Entry>> {
        match self.lookup.get(id) {
            Some(v) => Ok(Some(v.to_owned())),
            None => Ok(None),
        }
    }

    pub fn has(&self, id: &ids::Id) -> Result<bool> {
        match self.get(id) {
            Ok(resp) => match resp {
                Some(_) => Ok(true),
                None => Ok(false),
            },
            Err(e) => Err(e),
        }
    }
}
