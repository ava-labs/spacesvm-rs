use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
    sync::{Arc, RwLock},
};

use avalanche_types::ids;

use super::tx::Transaction;

// TOOO: price is not implemented for this vm.
/// Used to track pending transactions by [price].
#[derive(Debug)]
pub struct Heap {
    pub is_min_heap: bool,
    pub items: Vec<Entry>,
    pub lookup: Arc<RwLock<HashMap<ids::Id, Entry>>>,
}

#[derive(Debug, Default, Clone)]
pub struct Entry {
    pub id: ids::Id,
    pub tx: Option<Transaction>,
    pub index: usize,
    pub price: u64,
}

impl Heap {
    pub fn new(items: usize, is_min_heap: bool) -> Self {
        Self {
            is_min_heap,
            items: Vec::with_capacity(items),
            lookup: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn len(&self) -> usize {
        self.items.len()
    }

    async fn less(&self, i: usize, j: usize) -> bool {
        if self.is_min_heap {
            return self.items[i].price < self.items[j].price;
        }
        return self.items[i].price > self.items[j].price;
    }

    async fn swap(&mut self, i: usize, j: usize) {
        self.items.swap(i, j);
        self.items[i].index = i;
        self.items[j].index = j;
    }

    async fn push(&mut self, entry: &Entry) -> Result<()> {
        if self.has(entry.id).await? {
            return Ok(());
        }
        self.items.push(entry.to_owned());

        let mut lookup = self
            .lookup
            .write()
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
        // insert key only if it does not already exist.
        *lookup.entry(entry.id).or_insert(Entry::default()) = entry.clone();

        Ok(())
    }

    async fn pop(&mut self) -> Result<Option<Entry>> {
        Ok(self.items.pop())
    }

    async fn get(&self, id: ids::Id) -> Result<Option<Entry>> {
        let lookup = self.lookup.read().unwrap();
        match lookup.get(&id).clone() {
            Some(v) => Ok(Some(v.to_owned())),
            None => Ok(None),
        }
    }

    async fn has(&self, id: ids::Id) -> Result<bool> {
        match self.get(id).await {
            Ok(resp) => match resp {
                Some(_) => Ok(true),
                None => Ok(false),
            },
            Err(e) => Err(e),
        }
    }
}
