pub mod storage;
pub mod tx;
pub mod vm;

use std::{collections::VecDeque, sync::Arc};

use tokio::sync::{mpsc, RwLock};

use self::tx::tx::Transaction;

pub struct Mempool {
    pub inner: RwLock<VecDeque<Transaction>>,
    pub pending: mpsc::Receiver<()>,
}

impl Mempool {
    pub fn new(pending: mpsc::Receiver<()>) -> Arc<Self> {
        Arc::new(Self {
            inner: RwLock::new(VecDeque::new()),
            pending,
        })
    }
}
