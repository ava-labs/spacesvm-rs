#![allow(dead_code)]
#![allow(unused_imports)]

use avalanche_proto::grpcutil;
use avalanche_types::ids;
use jsonrpc_http_server::jsonrpc_core::IoHandler;
use prost::bytes::Bytes;
use semver::Version;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Error;
use std::net::SocketAddr;
use std::sync::RwLock;
use std::sync::{Arc, Mutex};
use std::time;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::{Channel, Endpoint, Server};
use tonic::{Request, Response, Status};

use avalanche_proto::google::protobuf::{Empty, Timestamp};
use avalanche_proto::{
    aliasreader::alias_reader_client::AliasReaderClient,
    appsender::app_sender_client::AppSenderClient, http::http_server::HttpServer,
    keystore::keystore_client::KeystoreClient, messenger::messenger_client::MessengerClient,
    rpcdb::database_client::DatabaseClient, sharedmemory::shared_memory_client::SharedMemoryClient,
    subnetlookup::subnet_lookup_client::SubnetLookupClient, vm, vm::vm_server::Vm,
};

use crate::block::Block;
use crate::kvvm::ChainVMInterior;
use crate::genesis::Genesis;

// FIXME: dummies
pub type Health = ();
pub type Fx = ();

pub type DbManager = Vec<VersionedDatabase>;

pub struct VersionedDatabase {
    pub database: DatabaseClient<Channel>,
    version: Version,
}

/// snow.common.HTTPHandler
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/engine/common#HTTPHandler
pub struct HTTPHandler {
    pub lock_options: u32,
    pub handler: IoHandler,
}

/// health.Checkable
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/health#Checkable
pub trait Checkable {
    fn health_check() -> Result<Health, Error>;
}

/// snow.validators.Connector
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/validators#Connector
pub trait Connector {
    fn connected(id: &ids::ShortId) -> Result<(), Error>;
    fn disconnected(id: &ids::ShortId) -> Result<(), Error>;
}

/// snow.Context
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow#Context
pub struct Context {
    pub network_id: u32,
    pub subnet_id: ids::Id,
    pub chain_id: ids::Id,
    pub node_id: ids::ShortId,

    pub x_chain_id: ids::Id,
    pub avax_asset_id: ids::Id,

    pub keystore: KeystoreClient<Channel>,
    pub shared_memory: SharedMemoryClient<Channel>,
    pub bc_lookup: AliasReaderClient<Channel>,
    pub sn_lookup: SubnetLookupClient<Channel>,
    // TODO metrics
}

/// snow.engine.common.AppHandler
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/engine/common#AppHandler
pub trait AppHandler {
    fn app_request(
        node_id: &ids::ShortId,
        request_id: u32,
        deadline: time::Instant,
        request: &[u8],
    ) -> Result<(), Error>;
    fn app_request_failed(node_id: &ids::ShortId, request_id: u32) -> Result<(), Error>;
    fn app_response(node_id: &ids::ShortId, request_id: u32, response: &[u8]) -> Result<(), Error>;
    fn app_gossip(node_id: &ids::ShortId, msg: &[u8]) -> Result<(), Error>;
}

/// snow.engine.common.VM
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/engine/common#VM
pub trait VM: AppHandler + Checkable + Connector {
    fn initialize(
        vm_inner: &Arc<RwLock<ChainVMInterior>>,
        ctx: Option<Context>,
        db_manager: &DbManager,
        genesis_bytes: &[u8],
        upgrade_bytes: &[u8],
        config_bytes: &[u8],
        to_engine: &MessengerClient<Channel>,
        fxs: &[Fx],
        app_sender: &AppSenderClient<Channel>,
    ) -> Result<(), Error>;
    fn bootstrapping() -> Result<(), Error>;
    fn issue_tx(key: String, value: String) -> Result<(), Error>;
    fn bootstrapped() -> Result<(), Error>;
    fn shutdown() -> Result<(), Error>;
    fn version() -> Result<String, Error>;
    fn create_static_handlers() -> Result<HashMap<String, HTTPHandler>, Error>;
    fn create_handlers() -> Result<HashMap<String, HTTPHandler>, Error>;
    fn set_state(inner: &Arc<RwLock<ChainVMInterior>>) -> Result<(), Error>;
}

pub trait Getter {
    fn get_block(id: String) -> Result<Block, Error>;
}

pub trait Parser {
    fn parse_block(bytes: &[u8]) -> Result<Block, Error>;
}

pub trait ChainVM: VM + Getter + Parser {
    fn build_block() -> Result<Block, Error>;
    fn issue_tx() -> Result<Block, Error>;
    fn set_preference(id: ids::Id) -> Result<(), Error>;
    fn last_accepted(inner: &Arc<RwLock<ChainVMInterior>>) -> Result<ids::Id, Error>;
}

pub struct VMServer<C> {
    vm: C,
    interior: Arc<RwLock<ChainVMInterior>>,
}

impl<C: ChainVM> VMServer<C> {
    pub fn new(chain_vm: C) -> Self {
        Self {
            vm: chain_vm,
            interior: Arc::new(RwLock::new(ChainVMInterior::new())),
        }
    }
}

#[tonic::async_trait]
impl<C: ChainVM + Send + Sync + 'static> vm::vm_server::Vm for VMServer<C> {
    async fn initialize(
        &self,
        req: Request<vm::InitializeRequest>,
    ) -> Result<Response<vm::InitializeResponse>, Status> {
        // log::info!("testChainVM");

        let req = req.into_inner();
        let client_conn = Endpoint::from_shared(format!("http://{}", req.server_addr))
            .unwrap()
            .connect()
            .await
            .unwrap();

        // Create gRPC clients
        // Multiplexing in tonic is done by cloning the client which is very cheap.
        // ref. https://docs.rs/tonic/latest/tonic/transport/struct.Channel.html#multiplexing-requests
        let msg_client = MessengerClient::new(client_conn.clone());
        let keystore_client = KeystoreClient::new(client_conn.clone());
        let shared_memory_client = SharedMemoryClient::new(client_conn.clone());
        let bc_lookup_client = AliasReaderClient::new(client_conn.clone());
        let sn_lookup_client = SubnetLookupClient::new(client_conn.clone());
        let app_sender_client = AppSenderClient::new(client_conn.clone());

        let subnet_id = ids::Id::from_slice(&req.subnet_id);
        let chain_id = ids::Id::from_slice(&req.chain_id);
        let node_id = ids::ShortId::from_slice(&req.node_id);
        let x_chain_id = ids::Id::from_slice(&req.x_chain_id);
        let avax_asset_id = ids::Id::from_slice(&req.avax_asset_id);

        let ctx = Some(Context {
            network_id: req.network_id,
            subnet_id: subnet_id,
            chain_id: chain_id,
            node_id: node_id,
            x_chain_id: x_chain_id,
            avax_asset_id: avax_asset_id,
            keystore: keystore_client,
            shared_memory: shared_memory_client,
            bc_lookup: bc_lookup_client,
            sn_lookup: sn_lookup_client,
        });

        let mut db_clients = DbManager::with_capacity(req.db_servers.len());
        for db_server in req.db_servers.iter() {
            let semver = db_server.version.trim_start_matches('v');
            let version =
                Version::parse(semver).map_err(|e| tonic::Status::unknown(e.to_string()))?;
            let server_addr = db_server.server_addr.clone();
            let client_conn = Endpoint::from_shared(format!("http://{}", server_addr))
                .unwrap()
                .connect()
                .await
                .unwrap();

            let db_client = DatabaseClient::new(client_conn);
            db_clients.push(VersionedDatabase {
                database: db_client,
                version: version,
            });
        }

        // Initialize ChainVM
        C::initialize(
            &self.interior.clone(),
            ctx,
            &db_clients,
            &req.genesis_bytes,
            &req.upgrade_bytes,
            &req.config_bytes,
            &msg_client,
            &[()],
            &app_sender_client,
        )
        .map_err(|e| tonic::Status::unknown(format!("VM::initialize failed: {}", e.to_string())))?;

        let last_accepted = C::last_accepted(&self.interior.clone()).unwrap();
        let block = C::get_block(last_accepted.to_string()).unwrap();

        let parent_id = Vec::from(block.parent().as_ref());
        let id = Vec::from(block.id().as_ref());
        let bytes = Vec::from(block.bytes());
        let timestamp = grpcutil::timestamp_from_time(block.timestamp());

        Ok(Response::new(vm::InitializeResponse {
            last_accepted_id: Bytes::from(id),
            last_accepted_parent_id: Bytes::from(parent_id),
            bytes: Bytes::from(bytes),
            height: block.height(),
            timestamp: Some(timestamp),
        }))
    }

    async fn shutdown(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn create_handlers(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<vm::CreateHandlersResponse>, Status> {
        Err(Status::unimplemented("create_static_handlers"))
    }

    async fn create_static_handlers(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<vm::CreateStaticHandlersResponse>, Status> {
        Err(Status::unimplemented("create_static_handlers"))
    }

    async fn connected(
        &self,
        req: Request<vm::ConnectedRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();
        let id = String::from_utf8_lossy(&req.node_id);
        // TODO: finish
        let node_id = ids::vm_id_from_str(&id);

        Ok(Response::new(Empty {}))
    }

    async fn disconnected(
        &self,
        _request: Request<vm::DisconnectedRequest>,
    ) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("disconnected"))
    }

    async fn build_block(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<vm::BuildBlockResponse>, Status> {
        Err(Status::unimplemented("build_block"))
    }

    async fn parse_block(
        &self,
        req: Request<vm::ParseBlockRequest>,
    ) -> Result<Response<vm::ParseBlockResponse>, Status> {
        let req = req.into_inner();

        let block = C::parse_block(req.bytes.as_ref()).unwrap();
        let parent_id = Vec::from(block.parent().as_ref());
        let timestamp = grpcutil::timestamp_from_time(block.timestamp());
        let status = block.status() as u32;
        let id = Vec::from(block.id().as_ref());

        Ok(Response::new(vm::ParseBlockResponse {
            id: Bytes::from(id),
            parent_id: Bytes::from(parent_id),
            status: status,
            height: block.height(),
            timestamp: Some(timestamp),
        }))
    }

    async fn get_block(
        &self,
        _request: Request<vm::GetBlockRequest>,
    ) -> Result<Response<vm::GetBlockResponse>, Status> {
        Err(Status::unimplemented("get_block"))
    }

    async fn set_state(
        &self,
        _request: Request<vm::SetStateRequest>,
    ) -> Result<Response<vm::SetStateResponse>, Status> {
        C::set_state(&self.interior.clone()).map_err(|e| {
            tonic::Status::unknown(format!("VM::set_state failed: {}", e.to_string()))
        })?;

        let last_accepted = C::last_accepted(&self.interior.clone()).unwrap();
        let block = C::get_block(last_accepted.to_string()).unwrap();
        let parent_id = Vec::from(block.parent().as_ref());
        let id = Vec::from(block.id().as_ref());
        let bytes = Vec::from(block.bytes());
        let timestamp = grpcutil::timestamp_from_time(block.timestamp());

        Ok(Response::new(vm::SetStateResponse {
            last_accepted_id: Bytes::from(id),
            last_accepted_parent_id: Bytes::from(parent_id),
            bytes: Bytes::from(bytes),
            height: block.height(),
            timestamp: Some(timestamp),
        }))
    }

    async fn verify_height_index(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<vm::VerifyHeightIndexResponse>, Status> {
        Err(Status::unimplemented("verify_height_index"))
    }

    async fn get_block_id_at_height(
        &self,
        _request: Request<vm::GetBlockIdAtHeightRequest>,
    ) -> Result<Response<vm::GetBlockIdAtHeightResponse>, Status> {
        Err(Status::unimplemented("get_block_id_at_height"))
    }

    async fn set_preference(
        &self,
        _request: Request<vm::SetPreferenceRequest>,
    ) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("set_preference"))
    }

    async fn health(
        &self,
        _request: Request<vm::HealthRequest>,
    ) -> Result<Response<vm::HealthResponse>, Status> {
        Ok(Response::new(vm::HealthResponse {
            details: "mini-kvvm is healthy".to_string(),
        }))
    }

    async fn version(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<vm::VersionResponse>, Status> {
        let interior = &self.interior.read().unwrap();
        log::info!("called version!!");
        Ok(Response::new(vm::VersionResponse {
            version: interior.version.to_string(),
        }))
    }

    async fn app_request(
        &self,
        _request: Request<vm::AppRequestMsg>,
    ) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("app_request"))
    }

    async fn app_request_failed(
        &self,
        _request: Request<vm::AppRequestFailedMsg>,
    ) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("app_request_failed"))
    }

    async fn app_response(
        &self,
        _request: Request<vm::AppResponseMsg>,
    ) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("app_response"))
    }

    async fn app_gossip(
        &self,
        _request: Request<vm::AppGossipMsg>,
    ) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("app_gossip"))
    }

    async fn block_verify(
        &self,
        _request: Request<vm::BlockVerifyRequest>,
    ) -> Result<Response<vm::BlockVerifyResponse>, Status> {
        Err(Status::unimplemented("block_verify"))
    }

    async fn block_accept(
        &self,
        _request: Request<vm::BlockAcceptRequest>,
    ) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("block_accept"))
    }
    async fn block_reject(
        &self,
        _request: Request<vm::BlockRejectRequest>,
    ) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("block_reject"))
    }

    async fn get_ancestors(
        &self,
        _request: Request<vm::GetAncestorsRequest>,
    ) -> Result<Response<vm::GetAncestorsResponse>, Status> {
        Err(Status::unimplemented("get_ancestors"))
    }

    async fn batched_parse_block(
        &self,
        _request: Request<vm::BatchedParseBlockRequest>,
    ) -> Result<Response<vm::BatchedParseBlockResponse>, Status> {
        Err(Status::unimplemented("batched_parse_block"))
    }

    async fn gather(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<vm::GatherResponse>, Status> {
        Err(Status::unimplemented("gather"))
    }

    async fn state_sync_enabled(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<vm::StateSyncEnabledResponse>, Status> {
        Err(Status::unimplemented("state_sync_enabled"))
    }

    async fn get_ongoing_sync_state_summary(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<vm::GetOngoingSyncStateSummaryResponse>, Status> {
        Err(Status::unimplemented("get_ongoing_sync_state_summary"))
    }

    async fn parse_state_summary(
        &self,
        _request: Request<vm::ParseStateSummaryRequest>,
    ) -> Result<tonic::Response<vm::ParseStateSummaryResponse>, Status> {
        Err(Status::unimplemented("parse_state_summary"))
    }

    async fn get_state_summary(
        &self,
        _request: Request<vm::GetStateSummaryRequest>,
    ) -> Result<Response<vm::GetStateSummaryResponse>, Status> {
        Err(Status::unimplemented("get_state_summary"))
    }

    async fn get_last_state_summary(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<vm::GetLastStateSummaryResponse>, Status> {
        Err(Status::unimplemented("get_last_state_summary"))
    }

    async fn state_summary_accept(
        &self,
        _request: Request<vm::StateSummaryAcceptRequest>,
    ) -> Result<tonic::Response<vm::StateSummaryAcceptResponse>, Status> {
        Err(Status::unimplemented("state_summary_accept"))
    }
}
