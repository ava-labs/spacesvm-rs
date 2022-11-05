use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
    sync::Arc,
    time,
};

use avalanche_proto::{
    google::protobuf::Empty,
    vm::{CreateStaticHandlersResponse, InitializeRequest, VersionedDbServer},
};
use avalanche_types::{
    ids,
    rpcchainvm::{
        self,
        common::{
            appsender,
            http_handler::{HttpHandler, LockOptions},
            vm::Fx,
        },
        snow::State,
    },
};
use chrono::{DateTime, Utc};
use prost::bytes::Bytes;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tonic::transport::Channel;

use crate::common::serve_test_database;

/// Test Vm client which interacts with rpcchainvm server service.
pub struct Client {
    inner: avalanche_proto::vm::vm_client::VmClient<Channel>,
    stop_ch: tokio::sync::broadcast::Sender<()>,

    db_server_addr: String,

    http_server_addr: String,
}

impl Client {
    pub fn new(client_conn: Channel) -> Box<dyn rpcchainvm::common::vm::Vm + Send + Sync> {
        // Initialize broadcast stop channel used to terminate gRPC servers during shutdown.
        let (stop_ch, _): (
            tokio::sync::broadcast::Sender<()>,
            tokio::sync::broadcast::Receiver<()>,
        ) = tokio::sync::broadcast::channel(1);

        Box::new(Self {
            inner: avalanche_proto::vm::vm_client::VmClient::new(client_conn),
            stop_ch,
            db_server_addr: String::new(),
            http_server_addr: String::new(),
        })
    }
}

#[tonic::async_trait]
impl rpcchainvm::common::vm::Vm for Client {
    async fn initialize(
        &mut self,
        _ctx: Option<rpcchainvm::context::Context>,
        _db_manager: Box<dyn rpcchainvm::database::manager::Manager + Send + Sync>,
        genesis_bytes: &[u8],
        _upgrade_bytes: &[u8],
        _config_bytes: &[u8],
        _to_engine: mpsc::Sender<rpcchainvm::common::message::Message>,
        _fxs: &[Fx],
        _app_sender: Box<dyn rpcchainvm::common::appsender::AppSender + Send + Sync>,
    ) -> Result<()> {
        // memdb wrapped in rpcdb
        let db = rpcchainvm::database::rpcdb::server::Server::new(
            rpcchainvm::database::memdb::Database::new(),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            serve_test_database(db, listener).await.unwrap();
        });

        let versiondb_servers = VersionedDbServer {
            server_addr: addr.clone().to_string(),
            version: "0.0.7".to_owned(),
        };

        let mut db_servers = Vec::with_capacity(1);
        db_servers.push(versiondb_servers);

        let genesis_bytes = Bytes::from(
            "{\"author\":\"subnet creator\",\"welcome_message\":\"Hello from Rust VM!\"}"
                .as_bytes(),
        );

        let request = InitializeRequest {
            network_id: 0,
            subnet_id: Bytes::from(ids::Id::empty().to_vec()),
            chain_id: Bytes::from(ids::Id::empty().to_vec()),
            node_id: Bytes::from(ids::node::Id::empty().to_vec()),
            x_chain_id: Bytes::from(ids::Id::empty().to_vec()),
            avax_asset_id: Bytes::from(ids::Id::empty().to_vec()),
            genesis_bytes,
            upgrade_bytes: Bytes::from(""),
            config_bytes: Bytes::from(""),
            db_servers,
            server_addr: addr.to_string(), //dummmy
        };

        let resp = self.inner.initialize(request).await.map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("initialize request failed: {:?}", e),
            )
        })?;

        Ok(())
    }

    async fn set_state(&self, _state: State) -> Result<()> {
        // TODO:
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        // TODO:
        Ok(())
    }

    async fn version(&self) -> Result<String> {
        Ok(String::new())
    }

    async fn create_handlers(
        &mut self,
    ) -> Result<HashMap<String, rpcchainvm::common::http_handler::HttpHandler>> {
        let resp = self.inner.create_handlers(Empty {}).await.map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("create handler request failed: {:?}", e),
            )
        })?;

        let resp = resp.into_inner();

        let mut http_handler: HashMap<String, rpcchainvm::common::http_handler::HttpHandler> =
            HashMap::new();

        for h in resp.handlers.iter() {
            let lock_option = LockOptions::try_from(h.lock_options)
                .map_err(|_| Error::new(ErrorKind::Other, "invalid lock option"))?;
            http_handler.insert(
                h.prefix.clone(),
                HttpHandler {
                    lock_option,
                    handler: None,
                    server_addr: Some(h.server_addr.clone()),
                },
            );
        }

        Ok(http_handler)
    }

    async fn create_static_handlers(
        &mut self,
    ) -> Result<HashMap<String, rpcchainvm::common::http_handler::HttpHandler>> {
        Ok(HashMap::new())
    }
}

#[tonic::async_trait]
impl rpcchainvm::health::Checkable for Client {
    async fn health_check(&self) -> Result<Vec<u8>> {
        // TODO:
        Ok(Vec::new())
    }
}

#[tonic::async_trait]
impl rpcchainvm::common::vm::Connector for Client {
    async fn connected(&self, id: &ids::node::Id) -> Result<()> {
        Ok(())
    }

    async fn disconnected(&self, id: &ids::node::Id) -> Result<()> {
        Ok(())
    }
}

#[tonic::async_trait]
impl rpcchainvm::common::apphandler::AppHandler for Client {
    async fn app_request(
        &self,
        node_id: &ids::node::Id,
        request_id: u32,
        deadline: DateTime<Utc>,
        request: &[u8],
    ) -> Result<()> {
        Ok(())
    }

    async fn app_request_failed(&self, node_id: &ids::node::Id, request_id: u32) -> Result<()> {
        Ok(())
    }

    async fn app_response(
        &self,
        node_id: &ids::node::Id,
        request_id: u32,
        response: &[u8],
    ) -> Result<()> {
        Ok(())
    }

    async fn app_gossip(&self, node_id: &ids::node::Id, msg: &[u8]) -> Result<()> {
        Ok(())
    }
}
