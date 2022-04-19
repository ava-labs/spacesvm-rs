#![allow(dead_code)]
#![allow(unused_imports)]

use avalanche_types::ids;
use jsonrpc_http_server::jsonrpc_core::IoHandler;
use semver::Version;
use std::collections::HashMap;
use std::io::{self, Error, ErrorKind};
use std::sync::{Arc, Mutex};
use std::time;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Channel;
use tonic::transport::Server;

use crate::http;
use crate::kvvm;
use crate::util;

use crate::aliasreaderpb::alias_reader_client::AliasReaderClient;
use crate::appsenderpb::app_sender_client::AppSenderClient;
use crate::keystorepb::keystore_client::KeystoreClient;
use crate::messengerpb::messenger_client::MessengerClient;
use crate::rpcdbpb::database_client::DatabaseClient;
use crate::sharedmemorypb::shared_memory_client::SharedMemoryClient;
use crate::subnetlookuppb::subnet_lookup_client::SubnetLookupClient;

use crate::httppb::http_server::HttpServer;
use crate::vmpb;

// FIXME: dummies
pub type Health = ();
pub type MessageChannel = ();
pub type Fx = ();
pub type AppSender = ();
pub type Block = ();
pub type DbManager = HashMap<Version, DatabaseClient<Channel>>;

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
        ctx: &Context,
        db_manager: &HashMap<Version, DatabaseClient<Channel>>,
        genesis_bytes: &[u8],
        upgrade_bytes: &[u8],
        config_bytes: &[u8],
        to_engine: MessageChannel,
        fxs: &[Fx],
        app_sender: &AppSender,
    ) -> Result<(), Error>;
    fn bootstrapping() -> Result<(), Error>;
    fn bootstrapped() -> Result<(), Error>;
    fn shutdown() -> Result<(), Error>;
    fn version() -> Result<String, Error>;
    fn create_static_handlers() -> Result<HashMap<String, HTTPHandler>, Error>;
    fn create_handlers() -> Result<HashMap<String, HTTPHandler>, Error>;
}

pub trait Getter {
    fn get_block(id: ids::Id) -> Result<Block, Error>;
}

pub trait Parser {
    fn parse_block(bytes: &[u8]) -> Result<Block, Error>;
}

pub trait ChainVM: VM + Getter + Parser {
    fn build_block() -> Result<Block, Error>;
    fn set_preference(id: ids::Id) -> Result<(), Error>;
    fn last_accepted() -> Result<ids::Id, Error>;
}

pub struct VMServer<C> {
    vm: C,
}

impl<C: ChainVM> VMServer<C> {
    pub fn new(chain_vm: C) -> Self {
        Self { vm: chain_vm }
    }
}

#[tonic::async_trait]
impl<C: ChainVM + Send + Sync + 'static> vmpb::vm_server::Vm for VMServer<C> {
    async fn initialize(
        &self,
        req: tonic::Request<vmpb::InitializeRequest>,
    ) -> Result<tonic::Response<vmpb::InitializeResponse>, tonic::Status> {
        let req = req.into_inner();

        // TODO: handle errors
        let subnet_id = ids::Id::from_slice(&req.subnet_id);
        let chain_id = ids::Id::from_slice(&req.chain_id);
        let node_id = ids::ShortId::from_slice(&req.node_id);
        let x_chain_id = ids::Id::from_slice(&req.x_chain_id);
        let avax_asset_id = ids::Id::from_slice(&req.avax_asset_id);
        let server_addr = req.server_addr;

        let client_conn = util::dial(&server_addr).await;

        // Create gRPC clients
        let msg_client = MessengerClient::new(client_conn);
        let keystore_client = KeystoreClient::new(client_conn);
        let shared_memory_client = SharedMemoryClient::new(client_conn);
        let bc_lookup_client = AliasReaderClient::new(client_conn);
        let sn_lookup_client = SubnetLookupClient::new(client_conn);
        let app_sender_client = AppSenderClient::new(client_conn);

        let network_id = req.network_id;

        let ctx = &Context {
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
        };

        let mut db_clients: DbManager = HashMap::new();
        for db_server in req.db_servers.iter() {
            let semver = db_server.version.trim_start_matches('v');
            let version = Version::parse(semver).expect("failed to parse semver");
            let client_conn = util::dial(&db_server.server_addr).await;
            let db_client = DatabaseClient::new(client_conn);
            db_clients.insert(version, db_client);
        }

        Ok(C::initialize(
            ctx,
            &db_clients,
            &req.genesis_bytes,
            &req.upgrade_bytes,
            &req.config_bytes,
            &app_sender_client,
            &app_sender_client,
            &app_sender_client,
        ));

        let resp = vmpb::InitializeResponse { handlers: handlers };
        Ok(tonic::Response::new(resp));
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
    ) -> Result<tonic::Response<vmpb::CreateHandlersResponse>, tonic::Status> {
        let chain_handlers = C::create_handlers().expect("failed to get handlers");
        let mut handlers = std::vec::Vec::new();

        for (prefix, h) in chain_handlers {
            let listener = util::new_listener().await;
            let server_addr = listener.local_addr().unwrap().to_string();
            tokio::spawn(async move {
                Server::builder()
                    .add_service(HttpServer::new(http::Server::new(h.handler)))
                    .serve_with_incoming(TcpListenerStream::new(listener))
                    .await
                    .unwrap();
            });

            let handler = vmpb::Handler {
                prefix: prefix,
                lock_options: h.lock_options,
                server_addr: server_addr,
            };
            handlers.push(handler);
        }

        let resp = vmpb::CreateHandlersResponse { handlers: handlers };
        Ok(tonic::Response::new(resp))
    }

    async fn create_static_handlers(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<tonic::Response<vmpb::CreateStaticHandlersResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("create_static_handlers"))
    }

    async fn connected(
        &self,
        _request: tonic::Request<vmpb::ConnectedRequest>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        Err(tonic::Status::unimplemented("connected"))
    }

    async fn disconnected(
        &self,
        _request: tonic::Request<vmpb::DisconnectedRequest>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        Err(tonic::Status::unimplemented("disconnected"))
    }

    async fn build_block(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<tonic::Response<vmpb::BuildBlockResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("build_block"))
    }

    async fn parse_block(
        &self,
        _request: tonic::Request<vmpb::ParseBlockRequest>,
    ) -> Result<tonic::Response<vmpb::ParseBlockResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("parse_block"))
    }

    async fn get_block(
        &self,
        _request: tonic::Request<vmpb::GetBlockRequest>,
    ) -> Result<tonic::Response<vmpb::GetBlockResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("get_block"))
    }

    async fn set_state(
        &self,
        _request: tonic::Request<vmpb::SetStateRequest>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        Err(tonic::Status::unimplemented("set_state"))
    }

    async fn verify_height_index(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<tonic::Response<vmpb::VerifyHeightIndexResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("verify_height_index"))
    }

    async fn get_block_id_at_height(
        &self,
        _request: tonic::Request<vmpb::GetBlockIdAtHeightRequest>,
    ) -> Result<tonic::Response<vmpb::GetBlockIdAtHeightResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("get_block_id_at_height"))
    }

    async fn set_preference(
        &self,
        _request: tonic::Request<vmpb::SetPreferenceRequest>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        Err(tonic::Status::unimplemented("set_preference"))
    }

    async fn health(
        &self,
        _request: tonic::Request<(vmpb::HealthRequest)>,
    ) -> Result<tonic::Response<vmpb::HealthResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("health"))
    }

    async fn version(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<tonic::Response<vmpb::VersionResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("version"))
    }

    async fn app_request(
        &self,
        _request: tonic::Request<vmpb::AppRequestMsg>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        Err(tonic::Status::unimplemented("app_request"))
    }

    async fn app_request_failed(
        &self,
        _request: tonic::Request<vmpb::AppRequestFailedMsg>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        Err(tonic::Status::unimplemented("app_request_failed"))
    }

    async fn app_response(
        &self,
        _request: tonic::Request<vmpb::AppResponseMsg>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        Err(tonic::Status::unimplemented("app_response"))
    }

    async fn app_gossip(
        &self,
        _request: tonic::Request<vmpb::AppGossipMsg>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        Err(tonic::Status::unimplemented("app_gossip"))
    }

    async fn block_verify(
        &self,
        _request: tonic::Request<vmpb::BlockVerifyRequest>,
    ) -> Result<tonic::Response<vmpb::BlockVerifyResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("block_verify"))
    }

    async fn block_accept(
        &self,
        _request: tonic::Request<vmpb::BlockAcceptRequest>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        Err(tonic::Status::unimplemented("block_accept"))
    }
    async fn block_reject(
        &self,
        _request: tonic::Request<vmpb::BlockRejectRequest>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        Err(tonic::Status::unimplemented("block_reject"))
    }

    async fn get_ancestors(
        &self,
        _request: tonic::Request<vmpb::GetAncestorsRequest>,
    ) -> Result<tonic::Response<vmpb::GetAncestorsResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("get_ancestors"))
    }

    async fn batched_parse_block(
        &self,
        _request: tonic::Request<vmpb::BatchedParseBlockRequest>,
    ) -> Result<tonic::Response<vmpb::BatchedParseBlockResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("batched_parse_block"))
    }

    async fn gather(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<tonic::Response<vmpb::GatherResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("gather"))
    }
}
