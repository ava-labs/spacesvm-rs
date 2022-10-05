use std::{
    io::{Error, ErrorKind, Result},
    num::NonZeroUsize,
};

use avalanche_types::{ids, rpcchainvm};
use lru::LruCache;
use semver::Version;
use tokio::sync::{broadcast, mpsc};

use crate::{block, genesis::Genesis, mempool};

use super::{ChainVmInner, BLOCKS_LRU_SIZE, MEMPOOL_SIZE};

pub struct Builder {
    pub ctx: Option<rpcchainvm::context::Context>,
    pub to_engine: Option<mpsc::Sender<rpcchainvm::common::message::Message>>,
    pub state: Option<block::state::State>,
    pub app_sender: Option<Box<dyn rpcchainvm::common::appsender::AppSender + Send + Sync>>,
    pub stop_ch: Option<broadcast::Sender<()>>,

    pub accepted_blocks: LruCache<ids::Id, block::Block>,
    pub bootstrapped: bool,
    pub version: Version,
    pub genesis: Genesis,
    pub preferred: ids::Id,
    pub last_accepted: block::Block,
    pub preferred_block_id: ids::Id,
    pub mempool: mempool::Mempool,

    pub builder_stop_rx: crossbeam_channel::Receiver<()>,
    pub builder_stop_tx: crossbeam_channel::Sender<()>,
    pub done_build_rx: crossbeam_channel::Receiver<()>,
    pub done_build_tx: crossbeam_channel::Sender<()>,
    pub done_gossip_rx: crossbeam_channel::Receiver<()>,
    pub done_gossip_tx: crossbeam_channel::Sender<()>,
    pub stop_rx: crossbeam_channel::Receiver<()>,
    pub stop_tx: crossbeam_channel::Sender<()>,
}

impl Builder {
    pub fn new() -> Builder {
        let (stop_tx, stop_rx): (
            crossbeam_channel::Sender<()>,
            crossbeam_channel::Receiver<()>,
        ) = crossbeam_channel::bounded(1);

        let (builder_stop_tx, builder_stop_rx): (
            crossbeam_channel::Sender<()>,
            crossbeam_channel::Receiver<()>,
        ) = crossbeam_channel::bounded(1);

        let (done_build_tx, done_build_rx): (
            crossbeam_channel::Sender<()>,
            crossbeam_channel::Receiver<()>,
        ) = crossbeam_channel::bounded(1);

        let (done_gossip_tx, done_gossip_rx): (
            crossbeam_channel::Sender<()>,
            crossbeam_channel::Receiver<()>,
        ) = crossbeam_channel::bounded(1);
        Builder {
            // required
            ctx: None,
            to_engine: None,
            app_sender: None,
            stop_ch: None,

            // defaults
            state: None,
            mempool: mempool::Mempool::new(MEMPOOL_SIZE),
            accepted_blocks: LruCache::new(NonZeroUsize::new(BLOCKS_LRU_SIZE).unwrap()),
            bootstrapped: false,
            version: Version::new(0, 0, 0),
            genesis: Genesis::default(),
            preferred: ids::Id::empty(),
            last_accepted: block::Block::default(),
            preferred_block_id: ids::Id::empty(),

            builder_stop_rx,
            builder_stop_tx,
            done_build_rx,
            done_build_tx,
            done_gossip_rx,
            done_gossip_tx,
            stop_rx,
            stop_tx,
        }
    }

    /// Required
    pub fn ctx(mut self, v: Option<rpcchainvm::context::Context>) -> Builder {
        self.ctx = v;
        self
    }

    /// Required.
    pub fn to_engine(mut self, v: mpsc::Sender<rpcchainvm::common::message::Message>) -> Builder {
        self.to_engine = Some(v);
        self
    }

    /// Required.
    pub fn stop_ch(mut self, v: broadcast::Sender<()>) -> Builder {
        self.stop_ch = Some(v);
        self
    }

    /// Required.
    pub fn state(mut self, v: Box<dyn rpcchainvm::database::Database + Sync + Send>) -> Builder {
        self.state = Some(block::state::State::new(v));
        self
    }

    /// Required.
    pub fn app_sender(
        mut self,
        v: Box<dyn rpcchainvm::common::appsender::AppSender + Send + Sync>,
    ) -> Builder {
        self.app_sender = Some(v);
        self
    }

    /// Default is false.
    pub fn bootstrapped(mut self, v: bool) -> Builder {
        // Set the name on the builder itself, and return the builder by value.
        self.bootstrapped = v;
        self
    }

    pub fn build(self) -> Result<ChainVmInner> {
        // Ensure required fields are set;
        if self.ctx.is_none() {
            Error::new(ErrorKind::InvalidInput, "ctx is required");
        }

        if self.to_engine.is_none() {
            return Err(Error::new(ErrorKind::InvalidInput, "to_engine is required"));
        }

        if self.app_sender.is_none() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "app_sender is required",
            ));
        }

        // if self.stop_ch.is_none() {
        //     return Err(Error::new(ErrorKind::InvalidInput, "stop_ch is required"));
        // }

        if self.state.is_none() {
            return Err(Error::new(ErrorKind::InvalidInput, "state is required"));
        }

        let ctx = self.ctx;
        let bootstrapped = self.bootstrapped;
        let version = self.version;
        let genesis = self.genesis;
        let preferred = self.preferred;
        let last_accepted = self.last_accepted;
        let to_engine = self.to_engine;
        let state = self.state.unwrap();
        let preferred_block_id = self.preferred_block_id;
        let stop_ch = self.stop_ch;
        let app_sender = self.app_sender;
        let mempool = self.mempool;
        let builder_stop_rx = self.builder_stop_rx;
        let builder_stop_tx = self.builder_stop_tx;
        let done_build_rx = self.done_build_rx;
        let done_build_tx = self.done_build_tx;
        let done_gossip_rx = self.done_gossip_rx;
        let done_gossip_tx = self.done_gossip_tx;
        let stop_rx = self.stop_rx;
        let stop_tx = self.stop_tx;

        Ok(ChainVmInner {
            ctx,
            bootstrapped,
            version,
            genesis,
            preferred,
            last_accepted,
            to_engine,
            state,
            preferred_block_id,
            stop_ch,
            app_sender,
            mempool,
            builder_stop_rx,
            builder_stop_tx,
            done_build_rx,
            done_build_tx,
            done_gossip_rx,
            done_gossip_tx,
            stop_rx,
            stop_tx,
            accepted_blocks: LruCache::new(NonZeroUsize::new(BLOCKS_LRU_SIZE).unwrap()),
        })
    }
}
