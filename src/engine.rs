#![allow(dead_code)]
#![allow(unused_imports)]

use avalanche_types::ids;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time;

use crate::error::AvaxError;
use crate::vm;

// FIXME: dummies
pub type Health = ();
pub type DBManager = ();
pub type MessageChannel = ();
pub type Fx = ();
pub type AppSender = ();
pub type Block = ();

pub struct HTTPHandler {
    pub lock_options: u32,
    pub server_addr: String,
    // TODO pub handler
}

/// health.Checkable
pub trait Checkable {
    fn health_check() -> Result<Health, AvaxError>;
}

/// snow.validators.Connector
pub trait Connector {
    fn connected(id: &ids::ShortId) -> Result<(), AvaxError>;
    fn disconnected(id: &ids::ShortId) -> Result<(), AvaxError>;
}

/// snow.Context
pub struct Context {
    pub network_id: u32,
    pub subnet_id: ids::Id,
    pub chain_id: ids::Id,
    pub node_id: ids::ShortId,

    pub xchain_id: ids::Id,
    pub avax_asset_id: ids::Id,
}

/// snow.engine.common.AppHandler
pub trait AppHandler {
    fn app_request(
        node_id: &ids::ShortId,
        request_id: u32,
        deadline: time::Instant,
        request: &[u8],
    ) -> Result<(), AvaxError>;
    fn app_request_failed(node_id: &ids::ShortId, request_id: u32) -> Result<(), AvaxError>;
    fn app_response(
        node_id: &ids::ShortId,
        request_id: u32,
        response: &[u8],
    ) -> Result<(), AvaxError>;
    fn app_gossip(node_id: &ids::ShortId, msg: &[u8]) -> Result<(), AvaxError>;
}

/// snow.engine.common.VM
pub trait VM: AppHandler + Checkable + Connector {
    fn initialize(
        ctx: &Context,
        db_manager: &DBManager,
        genesis_bytes: &[u8],
        upgrade_bytes: &[u8],
        config_bytes: &[u8],
        to_engine: MessageChannel,
        fxs: &[Fx],
        app_sender: &AppSender,
    ) -> Result<(), AvaxError>;
    fn bootstrapping() -> Result<(), AvaxError>;
    fn bootstrapped() -> Result<(), AvaxError>;
    fn shutdown() -> Result<(), AvaxError>;
    fn version() -> Result<String, AvaxError>;
    fn create_static_handlers() -> Result<HashMap<String, HTTPHandler>, AvaxError>;
    fn create_handlers() -> Result<HashMap<String, HTTPHandler>, AvaxError>;
}

pub trait Getter {
    fn get_block(id: ids::Id) -> Result<Block, AvaxError>;
}

pub trait Parser {
    fn parse_block(bytes: &[u8]) -> Result<Block, AvaxError>;
}

pub trait ChainVM: VM + Getter + Parser {
    fn build_block() -> Result<Block, AvaxError>;
    fn set_preference(id: ids::Id) -> Result<(), AvaxError>;
    fn last_accepted() -> Result<ids::Id, AvaxError>;
}

pub struct VMServer<C> {
    vm: std::sync::Mutex<C>,
}

impl<C: ChainVM> VMServer<C> {
    pub fn new(chain_vm: C) -> Self {
        VMServer {
            vm: std::sync::Mutex::new(chain_vm),
        }
    }
}

#[tonic::async_trait]
impl<C: ChainVM + Send + 'static> vm::vm_server::Vm for VMServer<C> {
    async fn initialize(
        &self,
        request: tonic::Request<vm::InitializeRequest>,
    ) -> Result<tonic::Response<vm::InitializeResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("initialize"))
    }

    async fn shutdown(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        Err(tonic::Status::unimplemented("shutdown"))
    }

    async fn create_handlers(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<tonic::Response<vm::CreateHandlersResponse>, tonic::Status> {
        let handlers = C::create_handlers().expect("failed to get handlers");
        let mut resp = std::vec::Vec::new();
        for (prefix, h) in handlers {
            let handler = vm::Handler {
                prefix: prefix,
                lock_options: h.lock_options,
                server_addr: h.server_addr,
                // TODO: add handler
            };
            resp.push(handler);
        }

        let resp = vm::CreateHandlersResponse { handlers: resp };
        Ok(tonic::Response::new(resp))
    }

    async fn create_static_handlers(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<tonic::Response<vm::CreateStaticHandlersResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("create_static_handlers"))
    }

    async fn connected(
        &self,
        request: tonic::Request<vm::ConnectedRequest>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        Err(tonic::Status::unimplemented("connected"))
    }

    async fn disconnected(
        &self,
        request: tonic::Request<vm::DisconnectedRequest>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        Err(tonic::Status::unimplemented("disconnected"))
    }

    async fn build_block(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<tonic::Response<vm::BuildBlockResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("build_block"))
    }

    async fn parse_block(
        &self,
        request: tonic::Request<vm::ParseBlockRequest>,
    ) -> Result<tonic::Response<vm::ParseBlockResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("parse_block"))
    }

    async fn get_block(
        &self,
        request: tonic::Request<vm::GetBlockRequest>,
    ) -> Result<tonic::Response<vm::GetBlockResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("get_block"))
    }

    async fn set_state(
        &self,
        request: tonic::Request<vm::SetStateRequest>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        Err(tonic::Status::unimplemented("set_state"))
    }

    async fn verify_height_index(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<tonic::Response<vm::VerifyHeightIndexResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("verify_height_index"))
    }

    async fn get_block_id_at_height(
        &self,
        request: tonic::Request<vm::GetBlockIdAtHeightRequest>,
    ) -> Result<tonic::Response<vm::GetBlockIdAtHeightResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("get_block_id_at_height"))
    }

    async fn set_preference(
        &self,
        request: tonic::Request<vm::SetPreferenceRequest>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        Err(tonic::Status::unimplemented("set_preference"))
    }

    async fn health(
        &self,
        request: tonic::Request<(vm::HealthRequest)>,
    ) -> Result<tonic::Response<vm::HealthResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("health"))
    }

    async fn version(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<tonic::Response<vm::VersionResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("version"))
    }

    async fn app_request(
        &self,
        request: tonic::Request<vm::AppRequestMsg>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        Err(tonic::Status::unimplemented("app_request"))
    }

    async fn app_request_failed(
        &self,
        request: tonic::Request<vm::AppRequestFailedMsg>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        Err(tonic::Status::unimplemented("app_request_failed"))
    }

    async fn app_response(
        &self,
        request: tonic::Request<vm::AppResponseMsg>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        Err(tonic::Status::unimplemented("app_response"))
    }

    async fn app_gossip(
        &self,
        request: tonic::Request<vm::AppGossipMsg>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        Err(tonic::Status::unimplemented("app_gossip"))
    }

    async fn block_verify(
        &self,
        request: tonic::Request<vm::BlockVerifyRequest>,
    ) -> Result<tonic::Response<vm::BlockVerifyResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("block_verify"))
    }

    async fn block_accept(
        &self,
        request: tonic::Request<vm::BlockAcceptRequest>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        Err(tonic::Status::unimplemented("block_accept"))
    }
    async fn block_reject(
        &self,
        request: tonic::Request<vm::BlockRejectRequest>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        Err(tonic::Status::unimplemented("block_reject"))
    }

    async fn get_ancestors(
        &self,
        request: tonic::Request<vm::GetAncestorsRequest>,
    ) -> Result<tonic::Response<vm::GetAncestorsResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("get_ancestors"))
    }

    async fn batched_parse_block(
        &self,
        request: tonic::Request<vm::BatchedParseBlockRequest>,
    ) -> Result<tonic::Response<vm::BatchedParseBlockResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("batched_parse_block"))
    }

    async fn gather(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<tonic::Response<vm::GatherResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("gather"))
    }
}
