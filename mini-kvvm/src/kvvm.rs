#![allow(dead_code)]

use std::{
    collections::HashMap,
    convert::TryInto,
    io::{Error, ErrorKind, Result},
    sync::Arc,
    time,
};
use avalanche_proto::{
    appsender::app_sender_client::AppSenderClient, messenger::messenger_client::MessengerClient,
    vm
};
use avalanche_types::{
    choices::status::Status,
    ids::{node::Id as NodeId, Id},
    vm::state::State as VmState,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use semver::Version;
use tokio::sync::RwLock;
use tonic::transport::Channel;

use crate::block::Block;
use crate::engine::*;
use crate::genesis::Genesis;
use crate::state::{Database, State};

use jsonrpc_core::{Error as JsonRPCError, ErrorCode as JRPCErrorCode, Value, Params};


#[derive(Debug)]
pub struct ChainVmInterior {
    pub ctx: Option<Context>,
    pub bootstrapped: bool,
    pub version: Version,
    pub genesis: Genesis,
    pub db: Option<Database>,
    pub state: State,
    pub preferred: Id,
    pub mempool: Vec<Vec<u8>>,
    pub verified_blocks: HashMap<Id, Block>,
    pub last_accepted: Block,
    preferred_block_id: Option<Id>,
}

impl ChainVmInterior {
    pub fn new() -> Self {
        Self {
            ctx: None,
            bootstrapped: false,
            version: Version::new(0, 0, 1),
            genesis: Genesis::default(),
            db: None,
            state: State::default(),
            preferred: Id::empty(),
            mempool: Vec::new(),
            verified_blocks: HashMap::new(),
            last_accepted: Block::default(),
            preferred_block_id: None,
        }
    }
}

// This VM doesn't (currently) have any app-specific messages
impl AppHandler for ChainVmInterior {
    fn app_request(
        _node_id: &NodeId,
        _request_id: u32,
        _deadline: time::Instant,
        _request: &[u8],
    ) -> Result<()> {
        Ok(())
    }

    fn app_request_failed(_node_id: &NodeId, _request_id: u32) -> Result<()> {
        Ok(())
    }

    fn app_response(_node_id: &NodeId, _request_id: u32, _response: &[u8]) -> Result<()> {
        Ok(())
    }

    fn app_gossip(_node_id: &NodeId, _msg: &[u8]) -> Result<()> {
        Ok(())
    }
}

// This VM doesn't implement Connector these methods are noop.
impl Connector for ChainVmInterior {
    fn connected(_id: &NodeId) -> Result<()> {
        Ok(())
    }
    fn disconnected(_id: &NodeId) -> Result<()> {
        Ok(())
    }
}

impl Checkable for ChainVmInterior {
    fn health_check() -> Result<Health> {
        Ok(())
    }
}

#[tonic::async_trait]
impl Vm for ChainVmInterior {
    async fn initialize(
        vm_inner: &Arc<RwLock<ChainVmInterior>>,
        ctx: Option<Context>,
        db_manager: &DbManager,
        genesis_bytes: &[u8],
        _upgrade_bytes: &[u8],
        _config_bytes: &[u8],
        _to_engine: &MessengerClient<Channel>,
        _fxs: &[Fx],
        _app_sender: &AppSenderClient<Channel>,
    ) -> Result<()> {
        let mut vm = vm_inner.write().await;
        vm.ctx = ctx;

        let current_db = &db_manager[0].database;
        vm.db = Some(current_db.clone());

        let state = State::new(Some(current_db.clone()));
        vm.state = state;

        let genesis = Genesis::from_json(genesis_bytes)?;

        vm.genesis = genesis;
        vm.genesis.verify().map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to verify genesis: {:?}", e),
            )
        })?;

        // Check if last accepted block exists
        if vm.state.has_last_accepted_block().await? {
            let maybe_last_accepted_block_id = vm.state.get_last_accepted_block_id().await?;
            if maybe_last_accepted_block_id.is_none() {
                return Err(Error::new(ErrorKind::Other, "invalid block no id found"));
            }
            let last_accepted_block_id = maybe_last_accepted_block_id.unwrap();

            let maybe_last_accepted_block = vm.state.get_block(last_accepted_block_id).await?;
            if maybe_last_accepted_block.is_none() {
                return Err(Error::new(ErrorKind::Other, "invalid block no id found"));
            }
            let last_accepted_block = maybe_last_accepted_block.unwrap();

            vm.preferred = last_accepted_block_id;
            vm.last_accepted = last_accepted_block;

            log::info!(
                "initialized from last accepted block {}",
                last_accepted_block_id
            );
        } else {
            let genesis_block_vec = genesis_bytes.to_vec();
            let genesis_block_bytes = genesis_block_vec.try_into().unwrap();

            let mut genesis_block = Block::new(
                Id::empty(),
                0,
                genesis_block_bytes,
                DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc),
                Status::Processing,
            )?;

            let genesis_block_id = genesis_block.initialize()?;

            let accepted_block_id = vm.state.accept_block(genesis_block).await.map_err(|e| {
                Error::new(ErrorKind::Other, format!("failed to accept block: {:?}", e))
            })?;
            // Remove accepted block now that it is accepted
            vm.verified_blocks.remove(&accepted_block_id);

            log::info!("initialized from genesis block: {:?}", genesis_block_id)
        }

        Ok(())
    }
    fn bootstrapping() -> Result<()> {
        Ok(())
    }
    fn bootstrapped() -> Result<()> {
        Ok(())
    }
    fn shutdown() -> Result<()> {
        Ok(())
    }

    async fn set_state(inner: &Arc<RwLock<ChainVmInterior>>, snow_state: VmState) -> Result<()> {
        let mut vm = inner.write().await;
        match snow_state.try_into() {
            // Initializing is called by chains manager when it is creating the chain.
            Ok(VmState::Initializing) => {
                log::debug!("set_state: initializing");
                vm.bootstrapped = false;
                Ok(())
            }
            Ok(VmState::StateSyncing) => {
                log::debug!("set_state: state syncing");
                Err(Error::new(ErrorKind::Other, "state sync is not supported"))
            }
            // Bootstrapping is called by the bootstrapper to signal bootstrapping has started.
            Ok(VmState::Bootstrapping) => {
                log::debug!("set_state: bootstrapping");
                vm.bootstrapped = false;
                Ok(())
            }
            // NormalOp os called when consensus has started signalling bootstrap phase is complete
            Ok(VmState::NormalOp) => {
                log::debug!("set_state: normal op");
                vm.bootstrapped = true;
                Ok(())
            }
            Err(_) => Err(Error::new(ErrorKind::Other, "failed to accept block")),
        }
    }

    /// Returns this VM's version
    fn version() -> Result<String> {
        Ok("".to_string())
    }
    fn create_static_handlers() -> Result<HashMap<String, HttpHandler>> {
        Ok(HashMap::new())
    }

    async fn create_handlers(inner: &'static Arc<RwLock<ChainVmInterior>>) -> Result<HashMap<String, HttpHandler>> {
        use crate::publicservicevm::*;
        let mut handlermap = HashMap::new();
        let handler = jsonrpc_core::IoHandler::new();

        async fn get_jsonrpc_error(code: JRPCErrorCode) -> JsonRPCError {
            JsonRPCError::new(code)
        }

        /// Converts serde_json result to a jsonrpc_core result
        async fn match_serialized(data: serde_json::Result<Value>) -> jsonrpc_core::Result<Value> {
            match data {
                Ok(x) => Ok(x),
                Err(e) => Err(get_jsonrpc_error(JRPCErrorCode::ParseError).await)
            }
        }

        /// Converts any serializable response [T] to json format
        async fn response_to_serialized<T: serde::Serialize> (response: &T) -> jsonrpc_core::Result<Value>{
            match_serialized(serde_json::to_value(response)).await
        }

        // Unimplemented
        handler.add_method("initialize", |params: Params| async move {
            Err(get_jsonrpc_error(JRPCErrorCode::MethodNotFound).await)
        });

        // Unimplemented
        handler.add_method("bootstrapping", |params: Params| async move {
            Err(get_jsonrpc_error(JRPCErrorCode::MethodNotFound).await)
        });

        // Unimplemented
        handler.add_method("bootstrapped", |params: Params| async move {
            Err(get_jsonrpc_error(JRPCErrorCode::MethodNotFound).await)
        });
        // Unimplemented
        handler.add_method("shutdown", |params: Params| async move {
            Err(get_jsonrpc_error(JRPCErrorCode::MethodNotFound).await)
        });

        handler.add_method("set_state", |params: Params| async move {
            let parsed: SetStateArgs = params.parse()?;
            let state = VmState::try_from(parsed.state);
            let state = match state { //if state does not match, propogate error
                Ok(x) => Ok(x),
                Err(e) => Err(get_jsonrpc_error(JRPCErrorCode::InvalidParams).await)
            }?;

            let result = ChainVmInterior::set_state(inner, state).await;
            let result: bool = match result {
                Ok(result) => true,
                Err(e) => false
            };

            let resp = SetStateResponse{
                accepted: result
            };

            response_to_serialized(&resp).await
        });

        handler.add_method("get_block", |params: Params| async move {
            let parsed: GetBlockArgs = params.parse()?;
            let result = ChainVmInterior::get_block(inner, parsed.id).await;
            let result = match result {
                Ok(x) => Ok(x),
                Err(e) => Err(get_jsonrpc_error(JRPCErrorCode::InternalError).await)
            }?;

            let resp = GetBlockResponse{
                block: result
            };

            response_to_serialized(&resp).await
        });

        handler.add_method("parse_block", |params: Params| async move {
            let parsed: ParseBlockArgs = params.parse()?;
            
            let result = ChainVmInterior::parse_block(inner, &parsed.bytes).await;
            let result = match result {
                Ok(x) => Ok(x),
                Err(e) => Err(get_jsonrpc_error(JRPCErrorCode::InternalError).await)
            }?;

            let resp = ParseBlockResponse {
                block: result
            };

            response_to_serialized(&resp).await
        });

        handler.add_method("build_block", |_params: Params| async move {
            let result = ChainVmInterior::build_block(inner).await;
            let result = match result {
                Ok(x) => Ok(x),
                Err(e) => Err(get_jsonrpc_error(JRPCErrorCode::InternalError).await)
            }?;

            let resp = BuildBlockResponse {
                block: result
            };

            response_to_serialized(&resp).await
        });

        handler.add_method("set_preference", |params: Params| async move {
            let parsed: SetPreferenceArgs = params.parse()?;
            let result = ChainVmInterior::set_preference(inner, parsed.id).await;

            let result = match result {
                Ok(x) => Ok(x),
                Err(e) => Err(get_jsonrpc_error(JRPCErrorCode::InternalError).await)
            }?;

            let resp = SetPreferenceResponse{};

            response_to_serialized(&resp).await
        });

        handler.add_method("last_accepted", |_params: Params| async move {
            let result = ChainVmInterior::last_accepted(inner).await;

            let result = match result {
                Ok(x) => Ok(x),
                Err(e) => Err(get_jsonrpc_error(JRPCErrorCode::InternalError).await)
            }?;

            let resp = LastAcceptedResponse{
                id: result
            };

            response_to_serialized(&resp).await
        });

        let handler = HttpHandler {
            lock_options: 0,
            handler
        };

        handlermap.insert(crate::publicservicevm::PUBLICENDPOINT, handler);
        Ok(handlermap)
    }
}

#[tonic::async_trait]
impl Getter for ChainVmInterior {
    async fn get_block(inner: &Arc<RwLock<ChainVmInterior>>, id: Id) -> Result<Block> {
        log::debug!("kvvm get_block called");
        let vm = inner.read().await;
        let state = crate::state::State::new(vm.db.clone());

        match state.get_block(id).await? {
            Some(mut block) => {
                let block_id = block.initialize()?;
                log::debug!("found old block id: {}", block_id.to_string());
                Ok(block)
            }
            None => Err(Error::new(
                ErrorKind::NotFound,
                format!("failed to get block id: {}", id),
            )),
        }
    }
}

#[tonic::async_trait]
impl Parser for ChainVmInterior {
    async fn parse_block(inner: &Arc<RwLock<ChainVmInterior>>, bytes: &[u8]) -> Result<Block> {
        log::debug!(
            "kvvm parse_block called: {}",
            String::from_utf8_lossy(&bytes)
        );
        let mut new_block: Block = serde_json::from_slice(bytes.as_ref())?;

        new_block.status = Status::Processing;

        let vm = inner.read().await;
        let state = crate::state::State::new(vm.db.clone());

        let new_block_id = new_block
            .initialize()
            .map_err(|e| Error::new(ErrorKind::Other, format!("failed to init block: {:?}", e)))?;

        match state.get_block(new_block_id).await? {
            Some(mut old_block) => {
                let old_block_id = old_block.initialize()?;
                log::debug!("parsed old block id: {}", old_block_id.to_string());
                Ok(old_block)
            }
            None => {
                log::debug!("parsed new block id: {}", new_block_id);
                Ok(new_block)
            }
        }
    }
}
#[tonic::async_trait]
impl ChainVm for ChainVmInterior {
    async fn build_block(inner: &Arc<RwLock<ChainVmInterior>>) -> Result<Block> {
        log::debug!("build_block called");
        let mut vm = inner.write().await;

        // Pop next block from mempool error if empty
        let block_value = vm
            .mempool
            .pop()
            .ok_or_else(|| Error::new(ErrorKind::Other, "there is no block to propose"))?;

        // Get Preferred Block
        let preferred_block = Self::get_block(inner, vm.preferred).await?;

        let mut new_block = Block::new(
            vm.preferred,
            preferred_block.height() + 1,
            block_value,
            chrono::offset::Utc::now(),
            Status::Processing,
        )
        .map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to create new block: {:?}", e),
            )
        })?;

        let new_block_id = new_block.initialize()?;

        new_block.verify(inner).await.map_err(|e| {
            Error::new(ErrorKind::Other, format!("failed to verify block: {:?}", e))
        })?;

        // Add block as verified
        vm.verified_blocks.insert(new_block_id, preferred_block);
        log::debug!("block verified {:?}", new_block.id());

        Ok(new_block)
    }

    async fn set_preference(inner: &Arc<RwLock<ChainVmInterior>>, id: Id) -> Result<()> {
        log::info!("setting preferred block id...");
        let mut vm = inner.write().await;
        vm.preferred_block_id = Some(id);
        Ok(())
    }

    async fn last_accepted(inner: &Arc<RwLock<ChainVmInterior>>) -> Result<Id> {
        let vm = inner.read().await;
        let state = crate::state::State::new(vm.db.clone());

        match state.get_last_accepted_block_id().await? {
            Some(last_accepted_block_id) => Ok(last_accepted_block_id),
            None => Err(Error::new(
                ErrorKind::NotFound,
                "failed to get last accepted block",
            )),
        }
    }
}
