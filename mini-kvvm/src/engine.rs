#![allow(dead_code)]

use std::{collections::HashMap, io::Result, sync::Arc, time};

use avalanche_proto::{
    aliasreader::alias_reader_client::AliasReaderClient,
    appsender::app_sender_client::AppSenderClient, google::protobuf::Empty, grpcutil,
    keystore::keystore_client::KeystoreClient, messenger::messenger_client::MessengerClient,
    rpcdb::database_client::DatabaseClient, sharedmemory::shared_memory_client::SharedMemoryClient,
    subnetlookup::subnet_lookup_client::SubnetLookupClient, vm,
};
use avalanche_types::{
    choices::status::Status, ids::node::Id as NodeId, ids::Id, vm::state::State as VmState,
};
use jsonrpc_http_server::jsonrpc_core::IoHandler;
use prost::bytes::Bytes;
use semver::Version;
use tokio::sync::RwLock;
use tonic::transport::{Channel, Endpoint};
use tonic::{Request, Response};

use crate::block::Block;
use crate::kvvm::ChainVmInterior;

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
pub struct HttpHandler {
    pub lock_options: u32,
    pub handler: IoHandler,
}

/// health.Checkable
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/health#Checkable
pub trait Checkable {
    fn health_check() -> Result<Health>;
}

/// snow.validators.Connector
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/validators#Connector
pub trait Connector {
    fn connected(id: &NodeId) -> Result<()>;
    fn disconnected(id: &NodeId) -> Result<()>;
}

/// snow.Context
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow#Context
#[derive(Debug)]
pub struct Context {
    pub network_id: u32,
    pub subnet_id: Id,
    pub chain_id: Id,
    pub node_id: NodeId,
    pub x_chain_id: Id,
    pub avax_asset_id: Id,
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
        node_id: &NodeId,
        request_id: u32,
        deadline: time::Instant,
        request: &[u8],
    ) -> Result<()>;
    fn app_request_failed(node_id: &NodeId, request_id: u32) -> Result<()>;
    fn app_response(node_id: &NodeId, request_id: u32, response: &[u8]) -> Result<()>;
    fn app_gossip(node_id: &NodeId, msg: &[u8]) -> Result<()>;
}

/// snow.engine.common.VM
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/engine/common#VM
#[tonic::async_trait]
pub trait Vm: AppHandler + Checkable + Connector {
    async fn initialize(
        vm_inner: &Arc<RwLock<ChainVmInterior>>,
        ctx: Option<Context>,
        db_manager: &DbManager,
        genesis_bytes: &[u8],
        upgrade_bytes: &[u8],
        config_bytes: &[u8],
        to_engine: &MessengerClient<Channel>,
        fxs: &[Fx],
        app_sender: &AppSenderClient<Channel>,
    ) -> Result<()>;
    fn bootstrapping() -> Result<()>;
    fn bootstrapped() -> Result<()>;
    fn shutdown() -> Result<()>;
    fn version() -> Result<String>;
    fn create_static_handlers() -> Result<HashMap<String, HttpHandler>>;
    fn create_handlers() -> Result<HashMap<String, HttpHandler>>;
    async fn set_state(inner: &Arc<RwLock<ChainVmInterior>>, state: VmState) -> Result<()>;
}

/// snow/engine/snowman/block.Getter
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/engine/snowman/block#Getter
#[tonic::async_trait]
pub trait Getter {
    async fn get_block(inner: &Arc<RwLock<ChainVmInterior>>, id: Id) -> Result<Block>;
}

/// snow/engine/snowman/block.Parser
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/engine/snowman/block#Parser
#[tonic::async_trait]
pub trait Parser {
    async fn parse_block(inner: &Arc<RwLock<ChainVmInterior>>, bytes: &[u8]) -> Result<Block>;
}
#[tonic::async_trait]
pub trait ChainVm: Vm + Getter + Parser {
    async fn build_block(inner: &Arc<RwLock<ChainVmInterior>>) -> Result<Block>;
    async fn set_preference(inner: &Arc<RwLock<ChainVmInterior>>, id: Id) -> Result<()>;
    async fn last_accepted(inner: &Arc<RwLock<ChainVmInterior>>) -> Result<Id>;
}

pub struct VmServer<V> {
    vm: V,
    interior: Arc<RwLock<ChainVmInterior>>,
}

impl<V: ChainVm> VmServer<V> {
    pub fn new(chain_vm: V) -> Self {
        Self {
            vm: chain_vm,
            interior: Arc::new(RwLock::new(ChainVmInterior::new())),
        }
    }
}

#[tonic::async_trait]
impl<V: ChainVm + Send + Sync + 'static> vm::vm_server::Vm for VmServer<V> {
    async fn initialize(
        &self,
        req: Request<vm::InitializeRequest>,
    ) -> std::result::Result<Response<vm::InitializeResponse>, tonic::Status> {
        log::info!("initialize called");

        let req = req.into_inner();
        let client_conn = Endpoint::from_shared(format!("http://{}", req.server_addr))
            .unwrap()
            .connect()
            .await
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        // Create gRPC clients
        // Multiplexing in tonic is done by cloning the client which is very cheap.
        // ref. https://docs.rs/tonic/latest/tonic/transport/struct.Channel.html#multiplexing-requests
        let message = MessengerClient::new(client_conn.clone());
        let keystore = KeystoreClient::new(client_conn.clone());
        let shared_memory = SharedMemoryClient::new(client_conn.clone());
        let bc_lookup = AliasReaderClient::new(client_conn.clone());
        let sn_lookup = SubnetLookupClient::new(client_conn.clone());
        let app_sender_client = AppSenderClient::new(client_conn.clone());

        let ctx = Some(Context {
            network_id: req.network_id,
            subnet_id: Id::from_slice(&req.subnet_id),
            chain_id: Id::from_slice(&req.chain_id),
            node_id: NodeId::from_slice(&req.node_id),
            x_chain_id: Id::from_slice(&req.x_chain_id),
            avax_asset_id: Id::from_slice(&req.avax_asset_id),
            keystore: keystore,
            shared_memory: shared_memory,
            bc_lookup: bc_lookup,
            sn_lookup: sn_lookup,
        });

        let mut db_clients = DbManager::with_capacity(req.db_servers.len());
        for db_server in req.db_servers.iter() {
            let semver = db_server.version.trim_start_matches('v');
            let version =
                Version::parse(semver).map_err(|e| tonic::Status::unknown(e.to_string()))?;
            let server_addr = db_server.server_addr.clone();
            let client_conn = Endpoint::from_shared(format!("http://{}", server_addr))
                .map_err(|e| tonic::Status::unknown(e.to_string()))?
                .connect()
                .await
                .map_err(|e| tonic::Status::unknown(e.to_string()))?;

            let db_client = DatabaseClient::new(client_conn);
            db_clients.push(VersionedDatabase {
                database: db_client,
                version: version,
            });
        }

        // Initialize ChainVm
        V::initialize(
            &self.interior.clone(),
            ctx,
            &db_clients,
            &req.genesis_bytes,
            &req.upgrade_bytes,
            &req.config_bytes,
            &message,
            &[()],
            &app_sender_client,
        )
        .await
        .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        let last_accepted = V::last_accepted(&self.interior).await?;

        let mut last_accepted_block = V::get_block(&self.interior, last_accepted).await?;
        log::debug!("last_accepted_block: {:?}", last_accepted_block);

        let last_accepted_block_id = last_accepted_block
            .initialize()
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        let parent_id = last_accepted_block.parent.to_vec();
        log::debug!("parent_id: {}", Id::from_slice(parent_id.as_ref()));

        let resp = vm::InitializeResponse {
            last_accepted_id: Bytes::from(last_accepted_block_id.to_vec()),
            last_accepted_parent_id: Bytes::from(parent_id),
            bytes: Bytes::from(last_accepted_block.bytes().to_vec()),
            height: last_accepted_block.height(),
            timestamp: Some(grpcutil::timestamp_from_time(
                last_accepted_block.timestamp(),
            )),
        };
        log::debug!("initialize response: {:#?}", resp);

        Ok(Response::new(resp))
    }

    async fn shutdown(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<Response<Empty>, tonic::Status> {
        Ok(Response::new(Empty {}))
    }

    // create_handlers executes create_handlers on the underlying vm implementation.
    // The handlers for the vms services will be returned in CreateStaticHandlersResponse.
    async fn create_handlers(
        &self,
        _req: Request<Empty>,
    ) -> std::result::Result<Response<vm::CreateHandlersResponse>, tonic::Status> {
        log::debug!("create_handlers called");
        Ok(Response::new(vm::CreateHandlersResponse::default()))
    }

    // create_static_handlers executes create_static_handlers on the underlying vm implementation.
    // The handlers for the static service will be returned in CreateStaticHandlersResponse.
    async fn create_static_handlers(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<Response<vm::CreateStaticHandlersResponse>, tonic::Status> {
        log::debug!("create_static_handlers called");
        Ok(Response::new(vm::CreateStaticHandlersResponse::default()))
    }

    // Connected is not implemented in rust VM currently.
    async fn connected(
        &self,
        _req: Request<vm::ConnectedRequest>,
    ) -> std::result::Result<Response<Empty>, tonic::Status> {
        log::debug!("connected called");
        Ok(Response::new(Empty {}))
    }

    async fn disconnected(
        &self,
        _request: Request<vm::DisconnectedRequest>,
    ) -> std::result::Result<Response<Empty>, tonic::Status> {
        log::debug!("disconnected called");
        Err(tonic::Status::unimplemented("disconnected"))
    }

    // build_block executes the build_block on the underlying vm implementation. If a new block is created
    // the block  data is returned in BuildBlockResponse.
    async fn build_block(
        &self,
        _req: Request<Empty>,
    ) -> std::result::Result<Response<vm::BuildBlockResponse>, tonic::Status> {
        log::debug!("build_block called");

        let mut block = V::build_block(&self.interior)
            .await
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        let block_id = block
            .initialize()
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        Ok(Response::new(vm::BuildBlockResponse {
            id: Bytes::from(block_id.to_vec()),
            parent_id: Bytes::from(block.parent.to_vec()),
            bytes: Bytes::from(block.bytes().to_vec()),
            height: block.height(),
            timestamp: Some(grpcutil::timestamp_from_time(block.timestamp())),
        }))
    }

    // parse_block takes the bytes from ParseBlockRequest and passes it to parse_block
    // of the underlying vm. If the block is parsed the block data is returned
    // in ParseBlockResponse.
    async fn parse_block(
        &self,
        req: Request<vm::ParseBlockRequest>,
    ) -> std::result::Result<Response<vm::ParseBlockResponse>, tonic::Status> {
        log::debug!("parse_block called");
        let req = req.into_inner();

        let mut block = V::parse_block(&self.interior, req.bytes.as_ref())
            .await
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        let block_id = block
            .initialize()
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        let status = block
            .status()
            .bytes()
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        Ok(Response::new(vm::ParseBlockResponse {
            id: Bytes::from(block_id.to_vec()),
            parent_id: Bytes::from(block.parent.to_vec()),
            status: Status::u32_from_slice(&status),
            height: block.height(),
            timestamp: Some(grpcutil::timestamp_from_time(block.timestamp())),
        }))
    }

    // get_block takes the Id from GetBlockRequest and passes it to parse_block
    // of the underlying vm. If a corresponding block it is returned
    // in GetBlockResponse.
    async fn get_block(
        &self,
        _request: Request<vm::GetBlockRequest>,
    ) -> std::result::Result<Response<vm::GetBlockResponse>, tonic::Status> {
        log::debug!("get_block called");
        Err(tonic::Status::unimplemented("get_block"))
    }

    async fn set_state(
        &self,
        req: Request<vm::SetStateRequest>,
    ) -> std::result::Result<Response<vm::SetStateResponse>, tonic::Status> {
        log::debug!("set_state called");
        let req = req.into_inner();

        let snow_state = VmState::try_from(req.state)
            .map_err(|_| tonic::Status::unknown("failed to convert to vm state"))?;

        V::set_state(&self.interior, snow_state)
            .await
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        let last_accepted = V::last_accepted(&self.interior)
            .await
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        let mut block = V::get_block(&self.interior, last_accepted)
            .await
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        let block_id = block
            .initialize()
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        Ok(Response::new(vm::SetStateResponse {
            last_accepted_id: Bytes::from(block_id.to_vec()),
            last_accepted_parent_id: Bytes::from(block.parent.to_vec()),
            bytes: Bytes::from(block.bytes().to_vec()),
            height: block.height(),
            timestamp: Some(grpcutil::timestamp_from_time(block.timestamp())),
        }))
    }

    // Currently state sync is not supported
    async fn verify_height_index(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<Response<vm::VerifyHeightIndexResponse>, tonic::Status> {
        log::debug!("verify_height_index called");
        Err(tonic::Status::unimplemented(
            "vm does not implement HeightIndexedChainVm interface",
        ))
    }

    async fn get_block_id_at_height(
        &self,
        _request: Request<vm::GetBlockIdAtHeightRequest>,
    ) -> std::result::Result<Response<vm::GetBlockIdAtHeightResponse>, tonic::Status> {
        log::debug!("get_block_id_at_height called");
        Err(tonic::Status::unimplemented("get_block_id_at_height"))
    }

    async fn set_preference(
        &self,
        req: Request<vm::SetPreferenceRequest>,
    ) -> std::result::Result<Response<Empty>, tonic::Status> {
        let req = req.into_inner();
        let id = Id::from_slice(&req.id);
        log::debug!("set_preference called id: {}", id);

        V::set_preference(&self.interior, id)
            .await
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        Ok(Response::new(Empty {}))
    }

    async fn health(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<Response<vm::HealthResponse>, tonic::Status> {
        Ok(Response::new(vm::HealthResponse {
            details: Bytes::from("mini-kvvm is healthy"),
        }))
    }

    async fn version(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<Response<vm::VersionResponse>, tonic::Status> {
        let interior = &self.interior.read().await;
        log::debug!("called version!!");
        Ok(Response::new(vm::VersionResponse {
            version: interior.version.to_string(),
        }))
    }

    async fn app_request(
        &self,
        _request: Request<vm::AppRequestMsg>,
    ) -> std::result::Result<Response<Empty>, tonic::Status> {
        log::debug!("app_request called");
        Err(tonic::Status::unimplemented("app_request"))
    }

    async fn app_request_failed(
        &self,
        _request: Request<vm::AppRequestFailedMsg>,
    ) -> std::result::Result<Response<Empty>, tonic::Status> {
        log::debug!("app_request_failed called");
        Err(tonic::Status::unimplemented("app_request_failed"))
    }

    async fn app_response(
        &self,
        _request: Request<vm::AppResponseMsg>,
    ) -> std::result::Result<Response<Empty>, tonic::Status> {
        log::debug!("app_response called");

        Err(tonic::Status::unimplemented("app_response"))
    }

    async fn app_gossip(
        &self,
        _request: Request<vm::AppGossipMsg>,
    ) -> std::result::Result<Response<Empty>, tonic::Status> {
        log::debug!("app_gossip called");

        Err(tonic::Status::unimplemented("app_gossip"))
    }

    async fn block_verify(
        &self,
        _request: Request<vm::BlockVerifyRequest>,
    ) -> std::result::Result<Response<vm::BlockVerifyResponse>, tonic::Status> {
        log::debug!("block_verify called");

        Err(tonic::Status::unimplemented("block_verify"))
    }

    async fn block_accept(
        &self,
        _request: Request<vm::BlockAcceptRequest>,
    ) -> std::result::Result<Response<Empty>, tonic::Status> {
        log::debug!("block_accept called");

        Err(tonic::Status::unimplemented("block_accept"))
    }
    async fn block_reject(
        &self,
        _request: Request<vm::BlockRejectRequest>,
    ) -> std::result::Result<Response<Empty>, tonic::Status> {
        log::debug!("block_reject called");

        Err(tonic::Status::unimplemented("block_reject"))
    }

    async fn get_ancestors(
        &self,
        _request: Request<vm::GetAncestorsRequest>,
    ) -> std::result::Result<Response<vm::GetAncestorsResponse>, tonic::Status> {
        log::debug!("get_ancestors called");

        Err(tonic::Status::unimplemented("get_ancestors"))
    }

    async fn batched_parse_block(
        &self,
        _request: Request<vm::BatchedParseBlockRequest>,
    ) -> std::result::Result<Response<vm::BatchedParseBlockResponse>, tonic::Status> {
        log::debug!("batched_parse_block called");

        Err(tonic::Status::unimplemented("batched_parse_block"))
    }

    async fn gather(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<Response<vm::GatherResponse>, tonic::Status> {
        log::debug!("gather called");

        Err(tonic::Status::unimplemented("gather"))
    }

    async fn state_sync_enabled(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<Response<vm::StateSyncEnabledResponse>, tonic::Status> {
        log::debug!("state_sync_enabled called");

        Err(tonic::Status::unimplemented("state_sync_enabled"))
    }

    async fn get_ongoing_sync_state_summary(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<Response<vm::GetOngoingSyncStateSummaryResponse>, tonic::Status> {
        log::debug!("get_ongoing_sync_state_summary called");

        Err(tonic::Status::unimplemented(
            "get_ongoing_sync_state_summary",
        ))
    }

    async fn parse_state_summary(
        &self,
        _request: Request<vm::ParseStateSummaryRequest>,
    ) -> std::result::Result<tonic::Response<vm::ParseStateSummaryResponse>, tonic::Status> {
        log::debug!("parse_state_summary called");

        Err(tonic::Status::unimplemented("parse_state_summary"))
    }

    async fn get_state_summary(
        &self,
        _request: Request<vm::GetStateSummaryRequest>,
    ) -> std::result::Result<Response<vm::GetStateSummaryResponse>, tonic::Status> {
        log::debug!("get_state_summary called");

        Err(tonic::Status::unimplemented("get_state_summary"))
    }

    async fn get_last_state_summary(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<Response<vm::GetLastStateSummaryResponse>, tonic::Status> {
        log::debug!("get_last_state_summary called");

        Err(tonic::Status::unimplemented("get_last_state_summary"))
    }

    async fn state_summary_accept(
        &self,
        _request: Request<vm::StateSummaryAcceptRequest>,
    ) -> std::result::Result<tonic::Response<vm::StateSummaryAcceptResponse>, tonic::Status> {
        log::debug!("state_summary_accept called");

        Err(tonic::Status::unimplemented("state_summary_accept"))
    }
}
