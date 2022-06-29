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
use jsonrpc_core::{Error as JsonRPCError, ErrorCode as JRPCErrorCode, Params, Value};
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
/// The context for which the Vm will operate
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
    /// Notifies this engine of a request for data from [node_id].
    /// Requests are Vm specific, so there is no guarantee requests are well-formed.
    fn app_request(
        node_id: &NodeId,
        request_id: u32,
        deadline: time::Instant,
        request: &[u8],
    ) -> Result<()>;

    /// Notifies this engine that a request sent to [node_id] has failed.
    fn app_request_failed(node_id: &NodeId, request_id: u32) -> Result<()>;

    /// Notifies this engine of a response sent by a request to [node_id].
    /// Does not guarantee that [response] is well-formed or expected
    fn app_response(node_id: &NodeId, request_id: u32, response: &[u8]) -> Result<()>;

    /// Notifes the engine of a gossip message
    /// Gossip messages are not responses from this engine, and also do not need to be responded to
    /// Nodes may gossip multiple times, so app_gossip may be called multiple times  
    fn app_gossip(node_id: &NodeId, msg: &[u8]) -> Result<()>;
}

/// snow.engine.common.Vm
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/engine/common#Vm
#[tonic::async_trait]
pub trait Vm: AppHandler + Checkable + Connector {
    /// Initialize the Vm.
    /// [vm_inner]:
    /// [ctx]: Metadata about the Vm
    /// [db_manager]: Manager of the database this Vm will run on
    /// [genesis_bytes]: Byte-encoding of genesis data for the Vm.
    ///                  This is data the Vm uses to intialize its
    ///                  state.
    /// [upgrade_bytes]: Byte-encoding of update data
    /// [config_bytes]: Byte-encoding of configuration data
    /// [to_engine]: Channel used to send messages to the consensus engine
    /// [fxs]: Feature extensions that attach to this Vm.
    /// [app_sender]: Channel used to send app requests
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

    // Retruns if currently bootstrapping
    fn bootstrapping() -> Result<()>;

    // Retruns if done bootstrapping
    fn bootstrapped() -> Result<()>;

    /// Called when node is shutting down
    fn shutdown() -> Result<()>;

    /// Returns version this Vm node is running
    fn version() -> Result<String>;

    /// Creates HTTP handlers for custom Vm network calls
    fn create_static_handlers() -> Result<HashMap<String, HttpHandler>>;

    /// Creates HTTP handlers for custom chain network calls
    async fn create_handlers(
        inner: &'static Arc<RwLock<ChainVmInterior>>,
    ) -> Result<HashMap<String, HttpHandler>>;

    /// Communicates to the Vm the next state which it should be in
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

/// snow/engine/snowmman/block.ChainVm
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/engine/snowman/block#ChainVm
#[tonic::async_trait]
pub trait ChainVm: Vm + Getter + Parser {
    /// Attempt to create a new block from ChainVm data
    /// Returns either a block or an error
    async fn build_block(inner: &Arc<RwLock<ChainVmInterior>>) -> Result<Block>;

    /// Issues a transaction to the chain
    async fn issue_tx() -> Result<Block>;

    /// Notify the Vm of the currently preferred block.
    async fn set_preference(inner: &Arc<RwLock<ChainVmInterior>>, id: Id) -> Result<()>;

    /// Returns the ID of the last accepted block.
    /// If no blocks have been accepted, this should return the genesis block
    async fn last_accepted(inner: &Arc<RwLock<ChainVmInterior>>) -> Result<Id>;
}

/// Server struct containing [vm], the virtual machine, and [interior], the interior data.
/// generic type [V] will mostly likely contain a ChainVm, as initialization functions currently
/// only accept ChainVm data
pub struct VmServer<V> {
    vm: V,
    interior: Arc<RwLock<ChainVmInterior>>,
}

impl<V: ChainVm> VmServer<V> {
    /// Create a ChainVmInterior in this VmServer
    pub fn new(chain_vm: V) -> Self {
        Self {
            vm: chain_vm,
            interior: Arc::new(RwLock::new(ChainVmInterior::new())),
        }
    }
}

/// Implementation of functionality for VmServer
/// Note:  V is most likely a ChainVmInterior object from kvvm.rs, and as such any
/// calls to functions from V (e.g. V::initialize) use the method defined in
/// kvvm.rs.
#[tonic::async_trait]
impl<V: ChainVm + Send + Sync + 'static> vm::vm_server::Vm for VmServer<V> {
    async fn initialize(
        &self,
        req: Request<vm::InitializeRequest>,
    ) -> std::result::Result<Response<vm::InitializeResponse>, tonic::Status> {
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
        let message = MessengerClient::new(client_conn.clone());
        let keystore = KeystoreClient::new(client_conn.clone());
        let shared_memory = SharedMemoryClient::new(client_conn.clone());
        let bc_lookup = AliasReaderClient::new(client_conn.clone());
        let sn_lookup = SubnetLookupClient::new(client_conn.clone());
        let app_sender_client = AppSenderClient::new(client_conn.clone());

        // Generate metadata from the request
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

            // Create a client connection with the server address
            let client_conn = Endpoint::from_shared(format!("http://{}", server_addr))
                .map_err(|e| tonic::Status::unknown(e.to_string()))?
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

        // Get last accepted block on the chain
        let last_accepted = V::last_accepted(&self.interior).await?;

        let mut last_accepted_block = V::get_block(&self.interior, last_accepted).await?;
        log::debug!("last_accepted_block: {:?}", last_accepted_block);

        let last_accepted_block_id = last_accepted_block
            .initialize()
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        let parent_id = last_accepted_block.parent.to_vec();
        log::debug!("parent_id: {}", Id::from_slice(parent_id.as_ref()));

        // If no problems occurred, pass back a valid InitializeResponse as a gRPC response
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
        req: Request<Empty>,
    ) -> std::result::Result<Response<vm::CreateHandlersResponse>, tonic::Status> {
        use crate::publicserviceeng::*;
        let mut handlermap = HashMap::new();
        let handler = jsonrpc_core::IoHandler::new();

        async fn get_jsonrpc_error(code: JRPCErrorCode) -> JsonRPCError {
            JsonRPCError::new(code)
        }

        /// Converts serde_json result to a jsonrpc_core result
        async fn match_serialized(data: serde_json::Result<Value>) -> jsonrpc_core::Result<Value> {
            match data {
                Ok(x) => Ok(x),
                Err(e) => Err(get_jsonrpc_error(JRPCErrorCode::ParseError).await),
            }
        }

        /// Converts any serializable response [T] to json format
        async fn response_to_serialized<T: serde::Serialize>(
            response: &T,
        ) -> jsonrpc_core::Result<Value> {
            match_serialized(serde_json::to_value(response)).await
        }

        fn bytes_to_vec(bytes: Bytes) -> Vec<u8> {
            bytes.as_ref().to_vec()
        }

        // Unimplemented
        handler.add_method("initialize", |params: Params| async move {
            let parsed: InitializeArgs = params.parse()?;

            let req: vm::InitializeRequest = match parsed.try_into() {
                Ok(x) => Ok(x),
                Err(e) => Err(get_jsonrpc_error(JRPCErrorCode::InternalError).await),
            }?;

            let req = Request::new(req);
            let result = self.initialize(req).await;
            let result = match result {
                Ok(x) => Ok(x),
                Err(e) => Err(get_jsonrpc_error(JRPCErrorCode::InternalError).await),
            }?;

            let resp = result.into_inner();

            let resp = InitializeResponseEng::try_from(resp).unwrap();

            response_to_serialized(&resp).await
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

            let req = Request::new(vm::SetStateRequest {
                state: parsed.state,
            });
            let result = self.set_state(req).await;

            let result = match result {
                Ok(x) => Ok(x),
                Err(e) => Err(get_jsonrpc_error(JRPCErrorCode::InternalError).await),
            }?;

            let resp = result.into_inner();

            let resp = SetStateResponseEng {
                last_accepted_id: bytes_to_vec(resp.last_accepted_id),
                last_accepted_parent_id: bytes_to_vec(resp.last_accepted_parent_id),
                height: resp.height,
                bytes: bytes_to_vec(resp.bytes),
            };

            response_to_serialized(&resp).await
        });

        handler.add_method("get_block", |params: Params| async move {
            let parsed: GetBlockArgs = params.parse()?;

            let req = Request::new(vm::GetBlockRequest {
                id: Bytes::from_iter(parsed.id),
            });

            let result = self.get_block(req).await;
            let result = match result {
                Ok(x) => Ok(x),
                Err(e) => Err(get_jsonrpc_error(JRPCErrorCode::InternalError).await),
            }?;

            let resp = result.into_inner();

            let resp = GetBlockResponseEng {
                parent_id: bytes_to_vec(resp.parent_id),
                bytes: bytes_to_vec(resp.bytes),
                status: resp.status,
                height: resp.height,
                err: resp.err,
            };

            response_to_serialized(&resp).await
        });

        handler.add_method("parse_block", |params: Params| async move {
            let parsed: ParseBlockArgs = params.parse()?;

            let req = Request::new(vm::ParseBlockRequest {
                bytes: Bytes::from_iter(parsed.bytes),
            });

            let result = self.parse_block(req).await;
            let result = match result {
                Ok(x) => Ok(x),
                Err(e) => Err(get_jsonrpc_error(JRPCErrorCode::InternalError).await),
            }?;

            let resp = result.into_inner();

            let resp = ParseBlockResponseEng {
                id: bytes_to_vec(resp.id),
                parent_id: bytes_to_vec(resp.parent_id),
                status: resp.status,
                height: resp.height,
            };

            response_to_serialized(&resp).await
        });

        handler.add_method("build_block", |_params: Params| async move {
            let request = Request::new(Empty {});
            let result = self.build_block(req).await;

            let result = match result {
                Ok(x) => Ok(x),
                Err(e) => Err(get_jsonrpc_error(JRPCErrorCode::InternalError).await),
            }?;

            let resp: vm::BuildBlockResponse = result.into_inner();

            let resp = BuildBlockResponseEng {
                id: bytes_to_vec(resp.id),
                parent_id: bytes_to_vec(resp.parent_id),
                bytes: bytes_to_vec(resp.bytes),
                height: resp.height,
            };

            response_to_serialized(&resp).await
        });

        handler.add_method("set_preference", |params: Params| async move {
            let parsed: SetPreferenceArgs = params.parse()?;

            let req = Request::new(vm::SetPreferenceRequest {
                id: Bytes::from_iter(parsed.id),
            });

            let result = self.set_preference(req).await;

            let result = match result {
                Ok(x) => Ok(x),
                Err(e) => Err(get_jsonrpc_error(JRPCErrorCode::InternalError).await),
            }?;

            match_serialized(serde_json::from_str("")).await
        });

        handler.add_method("get_last_state_summary", |_params: Params| async move {
            let req = Request::new(Empty {});

            let result = self.get_last_state_summary(req).await;

            let result = match result {
                Ok(x) => Ok(x),
                Err(e) => Err(get_jsonrpc_error(JRPCErrorCode::InternalError).await),
            }?;

            let resp = result.into_inner();

            let resp = LastStateSummaryResponseEng {
                id: bytes_to_vec(resp.id),
                height: resp.height,
                bytes: bytes_to_vec(resp.bytes),
                err: resp.err,
            };

            response_to_serialized(&resp).await
        });

        handler.

        let handler = HttpHandler {
            lock_options: 0,
            handler,
        };

        handler.

        handlermap.insert(crate::publicservicevm::PUBLICENDPOINT, handler);

        let resp = Response::new(vm::CreateHandlersResponse {
            handlers: handlermap
        });
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

    // Connected is not implemented in rust Vm currently.
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
    ) -> std::result::Result<Response<Empty>, Status> {
        log::info!("disconnected called");
        Err(Status::unimplemented("disconnected"))
    }

    async fn build_block(
        &self,
        _req: Request<Empty>,
    ) -> std::result::Result<Response<vm::BuildBlockResponse>, Status> {
        log::debug!("build_block called");

        // Build block based on ChainVmInterior data
        let mut block = V::build_block(&self.interior)
            .await
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        let block_id = block
            .initialize()
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        // If no problems occurred, pass back a valid BuildBlockResponse as a gRPC response
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

        // Get information about block
        let block_id = block
            .initialize()
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        let status = block
            .status()
            .bytes()
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        // If no problems occurred, pass a ParseBlockResponse as a gRPC response
        Ok(Response::new(vm::ParseBlockResponse {
            id: Bytes::from(block_id.to_vec()),
            parent_id: Bytes::from(block.parent.to_vec()),
            status: Status::u32::from_ne_bytes(status.to_vec().try_into().unwrap()),
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

        // Get the last generated block from ChainVm data
        let last_accepted = V::last_accepted(&self.interior)
            .await
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        let mut block = V::get_block(&self.interior, last_accepted)
            .await
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        let block_id = block
            .initialize()
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

        // If no errors occurred, return a valid SetStateResponse as a gRPC response
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

        // If no errors occurred, return empty response
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
        // Attempt to read interior data
        let interior = &self.interior.read().await;
        log::debug!("called version!!");

        // If no errors occurred, return a valid VersionResponse as a gRPC response
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