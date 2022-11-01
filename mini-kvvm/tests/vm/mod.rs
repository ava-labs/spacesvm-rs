pub mod client;

use std::collections::HashMap;
use std::io::{Error, ErrorKind};

use avalanche_types::rpcchainvm::common::http_handler::{HttpHandler, LockOptions};
use avalanche_types::{rpcchainvm, ids};
use avalanche_types::rpcchainvm::common::message::Message;
use avalanche_types::rpcchainvm::{common::vm::Vm, utils};
use jsonrpc_core::Response;
use mini_kvvm::genesis::Genesis;
use mini_kvvm::vm::{self, PUBLIC_API_ENDPOINT};
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::time::sleep;
use tokio::time::Duration;
use tonic::transport::Channel;
use tokio::sync::mpsc;

use crate::common::{self, decode_tx};

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn create_bucket_raw_json() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug"),
    );
    use crate::common::test_data;

    // new vm
    let mut vm = vm::ChainVm::new();

    // setup stop channel for grpc services.
    let (stop_ch_tx, stop_ch_rx): (Sender<()>, Receiver<()>) = tokio::sync::broadcast::channel(1);
    let vm_server = avalanche_types::rpcchainvm::vm::server::Server::new(
        Box::new(vm::ChainVm::new()),
        stop_ch_tx,
    );

    let addr = utils::new_socket_addr();

    tokio::spawn(async move {
        rpcchainvm::plugin::serve_with_address(vm_server, addr, stop_ch_rx)
            .await
            .expect("failed to start gRPC server");
    });

    // wait for server to start
    sleep(Duration::from_millis(100)).await;

    // create gRPC client for Vm client.
    let client_conn = Channel::builder(format!("http://{}", addr).parse().unwrap())
        .connect()
        .await
        .unwrap();

    let mut client = crate::vm::client::Client::new(client_conn);

    let db_manager = rpcchainvm::database::manager::DatabaseManager::new_from_databases(Vec::new());
    let app_sender = MockAppSender::new();
    let (tx_engine, mut rx_engine): (mpsc::Sender<Message>, mpsc::Receiver<Message>) = mpsc::channel(1);

     tokio::spawn(async move {
        loop {
            let _ = rx_engine.recv().await;
        }
    });

   let resp =  client.create_static_handlers().await;
   assert!(resp.is_ok());

   let handlers = resp.unwrap();

   // get the "/public" handler we assume it exists because it was created during initialize.
   let endpoint = handlers.get(PUBLIC_API_ENDPOINT).unwrap();

   let addr = endpoint.server_addr.as_ref().unwrap();

    // create gRPC client for Vm client.
    let client_conn = Channel::builder(format!("http://{}", addr).parse().unwrap())
        .connect()
        .await
        .unwrap();

    let mut http_client = avalanche_types::rpcchainvm::http::client::Client::new(client_conn);

    let genesis_bytes =
        "{\"author\":\"subnet creator\",\"welcome_message\":\"Hello from Rust VM!\"}".as_bytes();

    let resp = client.initialize(
        None,
        db_manager,
        genesis_bytes,
        &[],
        &[],
        tx_engine,
        &[()],
        app_sender,        
    ).await;

    assert!(resp.is_ok());

    sleep(Duration::from_secs(10)).await;

    // create a generic http request with json fixture to decode tx
    let data = decode_tx().as_bytes().to_vec();
    let req = http::request::Builder::new().body(data).unwrap();

    // pass the http request to the serve_http_simple RPC. this same process
    // takes place when the avalanchego router passes a request to the
    // subnet process. this process also simulates a raw JSON request from
    // curl or postman.
    let resp = http_client
        .serve_http_simple(req)
        .await
        .map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to initialize vm: {:?}", e),
            )
        })
        .unwrap();

    let resp_body_bytes = resp.body().to_owned();

    let json_response_str = std::str::from_utf8(&resp_body_bytes).unwrap();

    print!("{}",json_response_str);

        let data = test_data().as_bytes().to_vec();
    let req = http::request::Builder::new().body(data).unwrap();

    // pass the http request to the serve_http_simple RPC. this same process
    // takes place when the avalanchego router passes a request to the
    // subnet process. this process also simulates a raw JSON request from
    // curl or postman.
    let resp = http_client
        .serve_http_simple(req)
        .await
        .map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to initialize vm: {:?}", e),
            )
        })
        .unwrap();

    let resp_body_bytes = resp.body().to_owned();

     let json_response_str = std::str::from_utf8(&resp_body_bytes).unwrap();

    print!("{}",json_response_str);

    sleep(Duration::from_secs(25)).await;

    // TODO shutdown;

}


#[derive(Clone)] 
struct MockAppSender;

impl MockAppSender {
    fn new() -> Box<dyn rpcchainvm::common::appsender::AppSender + Send + Sync> {
        Box::new(MockAppSender{})
    }
}

#[tonic::async_trait]
impl rpcchainvm::common::appsender::AppSender for MockAppSender{
        async fn send_app_request(
        &self,
        _node_ids: ids::node::Set,
        _request_id: u32,
        _request: Vec<u8>,
    ) -> std::io::Result<()>{
        Ok(())
    }

    async fn send_app_response(
        &self,
        _node_if: ids::node::Id,
        _request_id: u32,
        _response: Vec<u8>,
    ) -> std::io::Result<()> {
        Ok(())
    }

    async fn send_app_gossip(&self, _msg: Vec<u8>) -> std::io::Result<()>{
        Ok(())
    }

    async fn send_app_gossip_specific(&self, _node_ids: ids::node::Set, _msg: Vec<u8>) -> std::io::Result<()>{
        Ok(())
    }
}