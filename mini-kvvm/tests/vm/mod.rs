pub mod client;

use std::io::{Error, ErrorKind};

use avalanche_types::rpcchainvm;
use avalanche_types::rpcchainvm::{common::vm::Vm, utils};
use mini_kvvm::vm;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::time::sleep;
use tokio::time::Duration;
use tonic::transport::Channel;

use crate::common;

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn create_bucket_raw_json() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug"),
    );
    use crate::common::test_data;

    // new vm
    let mut vm = vm::ChainVm::new();

    // NOTE: order is important! static handlers will be called before the vm
    // is initialized.
    let resp = vm.create_static_handlers().await;
    assert!(resp.is_ok());

    let handlers = resp.unwrap();

    let resp = common::initialize_vm(&mut vm).await;
    assert!(resp.is_ok());

    // setup stop channel for grpc services.
    let (stop_ch_tx, stop_ch_rx): (Sender<()>, Receiver<()>) = tokio::sync::broadcast::channel(1);
    let vm_server = avalanche_types::rpcchainvm::vm::server::Server::new(
        Box::new(vm::ChainVm::new()),
        stop_ch_tx,
    );

    let (stop_ch_tx, stop_ch_rx): (Sender<()>, Receiver<()>) = tokio::sync::broadcast::channel(1);

    tokio::spawn(async move {
        rpcchainvm::plugin::serve(vm_server, stop_ch_rx)
            .await
            .expect("failed to start gRPC server");
    });

    let addr = utils::new_socket_addr();

    // simulate rpcchainvm http service creation for handler
    for (_, handler) in handlers {
        let http_service =
            avalanche_types::rpcchainvm::http::server::Server::new(handler.handler.clone());
        let server = utils::grpc::Server::new(addr, stop_ch_tx.subscribe());
        let resp = server.serve(avalanche_proto::http::http_server::HttpServer::new(
            http_service,
        ));
        assert!(resp.is_ok());
    }

    // wait for server to start
    sleep(Duration::from_millis(10)).await;

    // create gRPC client for http service
    let client_conn = Channel::builder(format!("http://{}", addr).parse().unwrap())
        .connect()
        .await
        .unwrap();

    let mut client = avalanche_types::rpcchainvm::http::client::Client::new(client_conn);

    // create a generic http request with json fixture
    let data = test_data().as_bytes().to_vec();
    let req = http::request::Builder::new().body(data).unwrap();

    // pass the http request to the serve_http_simple RPC. this same process
    // takes place when the avalanchego router passes a request to the
    // subnet process. this process also simulates a raw JSON request from
    // curl or postman.
    let resp = client
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

    let json_response = std::str::from_utf8(&resp_body_bytes).unwrap();

    let inner = vm.inner.read().await;
    // shutdown builder and network threads.
    inner.stop_tx.send(()).unwrap();

    // stop_ch_tx.send(()).unwrap();
    // let builder = vm.network.as_ref().unwrap();
    sleep(Duration::from_secs(15)).await;
    // let network = builder.read().await;
    // assert_eq!(network.len().await, 1);

    // let vm = vm.clone();

    // let mut inner = vm.inner.read().await;

    // assert_eq!(inner.mempool.len(), 1);
    // let txs = inner.mempool.new_txs().unwrap();
    // assert_eq!(txs.len(), 1);

    log::info!("{}", json_response);
}

#[tokio::test]
async fn network_and_build_test() {
    use crate::common;
    // new vm
    let vm = vm::ChainVm::new();
    // let resp = common::initialize_vm(vm).await;
}
