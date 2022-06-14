#![allow(dead_code)]

use std::{collections::HashMap, io::Result, sync::Arc, time};

use avalanche_proto::{
    aliasreader::alias_reader_client::AliasReaderClient,
    appsender::app_sender_client::AppSenderClient, google::protobuf::Empty, grpcutil,
    keystore::keystore_client::KeystoreClient, messenger::messenger_client::MessengerClient,
    rpcdb::database_client::DatabaseClient, sharedmemory::shared_memory_client::SharedMemoryClient,
    subnetlookup::subnet_lookup_client::SubnetLookupClient, vm,
};
use avalanche_types::{ids::short::Id as ShortId, ids::Id};
use jsonrpc_http_server::jsonrpc_core::IoHandler;
use prost::bytes::Bytes;
use semver::Version;
use tokio::sync::RwLock;
use tonic::transport::{Channel, Endpoint};
use tonic::{Request, Response, Status};

use crate::block::Block;
use crate::kvvm::ChainVMInterior;
use crate::state::VmState;

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
    fn health_check() -> Result<Health>;
}

/// snow.validators.Connector
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/validators#Connector
pub trait Connector {
    fn connected(id: &ShortId) -> Result<()>;
    fn disconnected(id: &ShortId) -> Result<()>;
}

/// snow.Context
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow#Context
/// The context for which the VM will operate
#[derive(Debug)]
pub struct Context {
    pub network_id: u32,
    pub subnet_id: Id,
    pub chain_id: Id,
    pub node_id: ShortId,
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

    /// Notifies this engine of a request for data from [node_id].
    /// Requests are VM specific, so there is no guarantee requests are well-formed.
    fn app_request(
        node_id: &ShortId,
        request_id: u32,
        deadline: time::Instant,
        request: &[u8],
    ) -> Result<()>;

    /// Notifies this engine that a request sent to [node_id] has failed.
    fn app_request_failed(node_id: &ShortId, request_id: u32) -> Result<()>;

    /// Notifies this engine of a response sent by a request to [node_id].
    /// Does not guarantee that [response] is well-formed or expected
    fn app_response(node_id: &ShortId, request_id: u32, response: &[u8]) -> Result<()>;

    /// Notifes the engine of a gossip message
    /// Gossip messages are not responses from this engine, and also do not need to be responded to
    /// Nodes may gossip multiple times, so app_gossip may be called multiple times  
    fn app_gossip(node_id: &ShortId, msg: &[u8]) -> Result<()>;
}

/// snow.engine.common.VM
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/engine/common#VM
#[tonic::async_trait]
pub trait VM: AppHandler + Checkable + Connector {

    /// Initialize the VM.
    /// [vm_inner]: 
    /// [ctx]: Metadata about the VM
    /// [db_manager]: Manager of the database this VM will run on
    /// [genesis_bytes]: Byte-encoding of genesis data for the VM.
    ///                  This is data the VM uses to intialize its
    ///                  state.
    /// [upgrade_bytes]: Byte-encoding of update data
    /// [config_bytes]: Byte-encoding of configuration data
    /// [to_engine]: Channel used to send messages to the consensus engine
    /// [fxs]: Feature extensions that attach to this VM.
    /// [app_sender]: Channel used to send app requests
    async fn initialize(
        vm_inner: &Arc<RwLock<ChainVMInterior>>,
        ctx: Option<Context>,
        db_manager: &DbManager,
        genesis_bytes: &[u8],
        upgrade_bytes: &[u8],
        config_bytes: &[u8],
        to_engine: &MessengerClient<Channel>,
        fxs: &[Fx],
        app_sender: &AppSenderClient<Channel>,
    ) -> Result<()>;

    // Retruns if currently bootstrapping
    fn bootstrapping() -> Result<()>;

    // Retruns if done bootstrapping
    fn bootstrapped() -> Result<()>;

    /// Called when node is shutting down
    fn shutdown() -> Result<()>;

    /// Returns version this VM node is running
    fn version() -> Result<String>;

    /// Creates HTTP handlers for custom VM network calls
    fn create_static_handlers() -> Result<HashMap<String, HTTPHandler>>;

    /// Creates HTTP handlers for custom chain network calls
    fn create_handlers() -> Result<HashMap<String, HTTPHandler>>;

    /// Communicates to the VM the next state which it should be in
    async fn set_state(inner: &Arc<RwLock<ChainVMInterior>>, state: VmState) -> Result<()>;
}

/// snow/engine/snowman/block.Getter
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/engine/snowman/block#Getter
#[tonic::async_trait]
pub trait Getter {
    async fn get_block(inner: &Arc<RwLock<ChainVMInterior>>, id: Id) -> Result<Block>;
}

/// snow/engine/snowman/block.Parser
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/engine/snowman/block#Parser
#[tonic::async_trait]
pub trait Parser {
    async fn parse_block(inner: &Arc<RwLock<ChainVMInterior>>, bytes: &[u8]) -> Result<Block>;
}

/// snow/engine/snowmman/block.ChainVM
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/engine/snowman/block#ChainVM
#[tonic::async_trait]
pub trait ChainVM: VM + Getter + Parser {

    /// Attempt to create a new block from ChainVM data
    /// Returns either a block or an error
    async fn build_block(inner: &Arc<RwLock<ChainVMInterior>>) -> Result<Block>;

    /// Issues a transaction to the chain
    async fn issue_tx() -> Result<Block>;

    /// Notify the VM of the currently preferred block.
    async fn set_preference(inner: &Arc<RwLock<ChainVMInterior>>, id: Id) -> Result<()>;

    /// Returns the ID of the last accepted block.
    /// If no blocks have been accepted, this should return the genesis block
    async fn last_accepted(inner: &Arc<RwLock<ChainVMInterior>>) -> Result<Id>;
}

/// Server struct containing [vm], the virtual machine, and [interior], the interior data.
/// generic type [V] will mostly likely contain a ChainVM, as initialization functions currently 
/// only accept ChainVM data
pub struct VMServer<V> {
    vm: V,
    interior: Arc<RwLock<ChainVMInterior>>,
}


impl<V: ChainVM> VMServer<V> {

    /// Create a ChainVMInterior in this VMServer
    pub fn new(chain_vm: V) -> Self {
        Self {
            vm: chain_vm,
            interior: Arc::new(RwLock::new(ChainVMInterior::new())),
        }
    }
}

/// Implementation of functionality for VMServer
/// Note:  V is most likely a ChainVMInterior object from kvvm.rs, and as such any
/// calls to functions from V (e.g. V::initialize) use the method defined in
/// kvvm.rs.
#[tonic::async_trait]
impl<V: ChainVM + Send + Sync + 'static> vm::vm_server::Vm for VMServer<V> {

    async fn initialize(
        &self,
        req: Request<vm::InitializeRequest>,
    ) -> std::result::Result<Response<vm::InitializeResponse>, Status> {
        log::info!("initialize called");

        // From gRPC request, generate a client connection
        let req = req.into_inner();
        let client_conn = Endpoint::from_shared(format!("http://{}", req.server_addr))
            .unwrap()
            .connect()
            .await
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        // Create gRPC clients
        // Multiplexing in tonic is done by cloning the client which is very cheap.
        // ref. https://docs.rs/tonic/latest/tonic/transport/struct.Channel.html#multiplexing-requests
        let msg_client = MessengerClient::new(client_conn.clone());
        let keystore_client = KeystoreClient::new(client_conn.clone());
        let shared_memory_client = SharedMemoryClient::new(client_conn.clone());
        let bc_lookup_client = AliasReaderClient::new(client_conn.clone());
        let sn_lookup_client = SubnetLookupClient::new(client_conn.clone());
        let app_sender_client = AppSenderClient::new(client_conn.clone());

        // Generate metadata from the request
        let ctx = Some(Context {
            network_id: req.network_id,
            subnet_id: Id::from_slice(&req.subnet_id),
            chain_id: Id::from_slice(&req.chain_id),
            node_id: ShortId::from_slice(&req.node_id),
            x_chain_id: Id::from_slice(&req.x_chain_id),
            avax_asset_id: Id::from_slice(&req.avax_asset_id),
            keystore: keystore_client,
            shared_memory: shared_memory_client,
            bc_lookup: bc_lookup_client,
            sn_lookup: sn_lookup_client,
        });

        // Generate database clients for every db_server in the request, along with an 
        // open stream for every db_server.
        let mut db_clients = DbManager::with_capacity(req.db_servers.len());
        for db_server in req.db_servers.iter() {
            // Get version
            let semver = db_server.version.trim_start_matches('v');
            let version =
                Version::parse(semver).map_err(|e| tonic::Status::unknown(e.to_string()))?;
            let server_addr = db_server.server_addr.clone();

            // Create a client connection with the server address
            let client_conn = Endpoint::from_shared(format!("http://{}", server_addr))
                .unwrap()
                .connect()
                .await
                .map_err(|e| tonic::Status::unknown(e.to_string()))?;

            // If succesfull, push the new db_client into db_clients
            let db_client = DatabaseClient::new(client_conn);
            db_clients.push(VersionedDatabase {
                database: db_client,
                version: version,
            });
        }

        // Initialize ChainVM from ChainVMInterior initialization function
        V::initialize(
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
        .await
        .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        // Get last accepted block on the chain
        let last_accepted = V::last_accepted(&self.interior).await?;

        let mut last_accepted_block = V::get_block(&self.interior, last_accepted).await?;
        log::info!("last_accepted_block: {:?}", last_accepted_block);

        let last_accepted_block_id = last_accepted_block
            .initialize()
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        let parent_id = last_accepted_block.parent.to_vec();
        log::info!("parent_id: {}", Id::from_slice(parent_id.as_ref()));

        // If no problems occurred, pass back a valid InitializeResponse as a gRCP response
        let resp = vm::InitializeResponse {
            last_accepted_id: Bytes::from(last_accepted_block_id.to_vec()),
            last_accepted_parent_id: Bytes::from(parent_id),
            bytes: Bytes::from(last_accepted_block.bytes().to_vec()),
            height: last_accepted_block.height(),
            timestamp: Some(grpcutil::timestamp_from_time(
                last_accepted_block.timestamp(),
            )),
        };
        log::debug!("init resp: {:#?}", resp);

        Ok(Response::new(resp))
    }

    async fn shutdown(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn create_handlers(
        &self,
        _req: Request<Empty>,
    ) -> std::result::Result<Response<vm::CreateHandlersResponse>, Status> {
        log::info!("create_handlers called");
        //TODO
        Ok(Response::new(vm::CreateHandlersResponse::default()))
    }

    async fn create_static_handlers(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<Response<vm::CreateStaticHandlersResponse>, Status> {
        log::info!("create_static_handlers called");
        Ok(Response::new(vm::CreateStaticHandlersResponse::default()))
    }

    // Connected is not implemented in rust VM currently.
    async fn connected(
        &self,
        _req: Request<vm::ConnectedRequest>,
    ) -> std::result::Result<Response<Empty>, Status> {
        log::info!("connected called");
        Ok(Response::new(Empty {}))
    }

    async fn disconnected(
        &self,
        _request: Request<vm::DisconnectedRequest>,
    ) -> std::result::Result<Response<Empty>, Status> {
        log::info!("disconnected called");
        Err(Status::unimplemented("disconnected"))
    }

    async fn build_block(
        &self,
        _req: Request<Empty>,
    ) -> std::result::Result<Response<vm::BuildBlockResponse>, Status> {
        log::debug!("build_block called");

        // Build block based on ChainVMInterior data
        let mut block = V::build_block(&self.interior)
            .await
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        let block_id = block
            .initialize()
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        // If no problems occurred, pass back a valid BuildBlockResponse as a gRCP response
        Ok(Response::new(vm::BuildBlockResponse {
            id: Bytes::from(block_id.to_vec()),
            parent_id: Bytes::from(block.parent.to_vec()),
            bytes: Bytes::from(block.bytes().to_vec()),
            height: block.height(),
            timestamp: Some(grpcutil::timestamp_from_time(block.timestamp())),
        }))
    }

    async fn parse_block(
        &self,
        req: Request<vm::ParseBlockRequest>,
    ) -> std::result::Result<Response<vm::ParseBlockResponse>, Status> {
        log::debug!("parse_block called");
        let req = req.into_inner();

        // Take the byte stream and attempt to compile into a block
        let mut block = V::parse_block(&self.interior, req.bytes.as_ref())
            .await
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        // Get information about block
        let block_id = block
            .initialize()
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        let status = block
            .status()
            .bytes()
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        // If no problems occurred, pass a ParseBlockResponse as a gRCP response
        Ok(Response::new(vm::ParseBlockResponse {
            id: Bytes::from(block_id.to_vec()),
            parent_id: Bytes::from(block.parent.to_vec()),
            status: u32::from_ne_bytes(status.to_vec().try_into().unwrap()),
            height: block.height(),
            timestamp: Some(grpcutil::timestamp_from_time(block.timestamp())),
        }))
    }

    async fn get_block(
        &self,
        _request: Request<vm::GetBlockRequest>,
    ) -> std::result::Result<Response<vm::GetBlockResponse>, Status> {
        log::info!("get_block called");
        Err(Status::unimplemented("get_block"))
    }

    async fn set_state(
        &self,
        req: Request<vm::SetStateRequest>,
    ) -> std::result::Result<Response<vm::SetStateResponse>, Status> {
        log::debug!("set_state called");
        let req = req.into_inner();

        // From request, generate a VmState from 
        // 0 - 3 (Initializing, StateSyncing, Bootstrapping, and NormalOp respectively
        let snow_state = VmState::try_from(req.state).unwrap();
        V::set_state(&self.interior, snow_state)
            .await
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        // Get the last generated block from ChainVM data
        let last_accepted = V::last_accepted(&self.interior)
            .await
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        let mut block = V::get_block(&self.interior, last_accepted)
            .await
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        let block_id = block
            .initialize()
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        // If no errors occurred, return a valid SetStateResponse as a gRCP response
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
    ) -> std::result::Result<Response<vm::VerifyHeightIndexResponse>, Status> {
        log::info!("verify_height_index called");
        Err(Status::unimplemented(
            "vm does not implement HeightIndexedChainVM interface",
        ))
    }

    async fn get_block_id_at_height(
        &self,
        _request: Request<vm::GetBlockIdAtHeightRequest>,
    ) -> std::result::Result<Response<vm::GetBlockIdAtHeightResponse>, Status> {
        log::info!("get_block_id_at_height called");
        Err(Status::unimplemented("get_block_id_at_height"))
    }

    async fn set_preference(
        &self,
        req: Request<vm::SetPreferenceRequest>,
    ) -> std::result::Result<Response<Empty>, Status> {

        // Get id of the preferred block
        let req = req.into_inner();
        let id = Id::from_slice(&req.id);

        log::debug!("set_preference called id: {}", id);

        // Set ChainVMInterior block preference
        V::set_preference(&self.interior, id)
            .await
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        // If no errors occurred, return empty response
        Ok(Response::new(Empty {}))
    }

    async fn health(
        &self,
        _request: Request<vm::HealthRequest>,
    ) -> std::result::Result<Response<vm::HealthResponse>, Status> {
        Ok(Response::new(vm::HealthResponse {
            details: "mini-kvvm is healthy".to_string(),
        }))
    }

    async fn version(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<Response<vm::VersionResponse>, Status> {

        // Attempt to read interior data
        let interior = &self.interior.read().await;
        log::info!("called version!!");

        // If no errors occurred, return a valid VersionResponse as a gRCP response
        Ok(Response::new(vm::VersionResponse {
            version: interior.version.to_string(),
        }))
    }

    async fn app_request(
        &self,
        _request: Request<vm::AppRequestMsg>,
    ) -> std::result::Result<Response<Empty>, Status> {
        log::info!("app_request called");
        Err(Status::unimplemented("app_request"))
    }

    async fn app_request_failed(
        &self,
        _request: Request<vm::AppRequestFailedMsg>,
    ) -> std::result::Result<Response<Empty>, Status> {
        log::info!("app_request_failed called");
        Err(Status::unimplemented("app_request_failed"))
    }

    async fn app_response(
        &self,
        _request: Request<vm::AppResponseMsg>,
    ) -> std::result::Result<Response<Empty>, Status> {
        log::info!("app_response called");

        Err(Status::unimplemented("app_response"))
    }

    async fn app_gossip(
        &self,
        _request: Request<vm::AppGossipMsg>,
    ) -> std::result::Result<Response<Empty>, Status> {
        log::info!("app_gossip called");

        Err(Status::unimplemented("app_gossip"))
    }

    async fn block_verify(
        &self,
        _request: Request<vm::BlockVerifyRequest>,
    ) -> std::result::Result<Response<vm::BlockVerifyResponse>, Status> {
        log::info!("block_verify called");

        Err(Status::unimplemented("block_verify"))
    }

    async fn block_accept(
        &self,
        _request: Request<vm::BlockAcceptRequest>,
    ) -> std::result::Result<Response<Empty>, Status> {
        log::info!("block_accept called");

        Err(Status::unimplemented("block_accept"))
    }
    async fn block_reject(
        &self,
        _request: Request<vm::BlockRejectRequest>,
    ) -> std::result::Result<Response<Empty>, Status> {
        log::info!("block_reject called");

        Err(Status::unimplemented("block_reject"))
    }

    async fn get_ancestors(
        &self,
        _request: Request<vm::GetAncestorsRequest>,
    ) -> std::result::Result<Response<vm::GetAncestorsResponse>, Status> {
        log::info!("get_ancestors called");

        Err(Status::unimplemented("get_ancestors"))
    }

    async fn batched_parse_block(
        &self,
        _request: Request<vm::BatchedParseBlockRequest>,
    ) -> std::result::Result<Response<vm::BatchedParseBlockResponse>, Status> {
        log::info!("batched_parse_block called");

        Err(Status::unimplemented("batched_parse_block"))
    }

    async fn gather(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<Response<vm::GatherResponse>, Status> {
        log::info!("gather called");

        Err(Status::unimplemented("gather"))
    }

    async fn state_sync_enabled(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<Response<vm::StateSyncEnabledResponse>, Status> {
        log::info!("state_sync_enabled called");

        Err(Status::unimplemented("state_sync_enabled"))
    }

    async fn get_ongoing_sync_state_summary(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<Response<vm::GetOngoingSyncStateSummaryResponse>, Status> {
        log::info!("get_ongoing_sync_state_summary called");

        Err(Status::unimplemented("get_ongoing_sync_state_summary"))
    }

    async fn parse_state_summary(
        &self,
        _request: Request<vm::ParseStateSummaryRequest>,
    ) -> std::result::Result<tonic::Response<vm::ParseStateSummaryResponse>, Status> {
        log::info!("parse_state_summary called");

        Err(Status::unimplemented("parse_state_summary"))
    }

    async fn get_state_summary(
        &self,
        _request: Request<vm::GetStateSummaryRequest>,
    ) -> std::result::Result<Response<vm::GetStateSummaryResponse>, Status> {
        log::info!("get_state_summary called");

        Err(Status::unimplemented("get_state_summary"))
    }

    async fn get_last_state_summary(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<Response<vm::GetLastStateSummaryResponse>, Status> {
        log::info!("get_last_state_summary called");

        Err(Status::unimplemented("get_last_state_summary"))
    }

    async fn state_summary_accept(
        &self,
        _request: Request<vm::StateSummaryAcceptRequest>,
    ) -> std::result::Result<tonic::Response<vm::StateSummaryAcceptResponse>, Status> {
        log::info!("state_summary_accept called");

        Err(Status::unimplemented("state_summary_accept"))
    }
}
