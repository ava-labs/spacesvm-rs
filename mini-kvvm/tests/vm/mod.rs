use std::{
    io::{Error, ErrorKind},
};

use avalanche_types::rpcchainvm::{common::vm::Vm, utils};
use mini_kvvm::{
    vm,
};
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::time::Duration;
use tokio::time::sleep;
use tonic::transport::Channel;

#[tokio::test]
async fn create_bucket_raw_json() {
    use crate::common::test_data;

    // new vm
    let vm = vm::ChainVm::new();

    // NOTE: order is important! static handlers will be called before the vm
    // is initialized.
    let resp = vm.create_static_handlers().await;
    assert!(resp.is_ok());

    let handlers = resp.unwrap();

    // setup stop channel for grpc services.
    let (stop_ch_tx, _): (Sender<()>, Receiver<()>) = tokio::sync::broadcast::channel(1);

    let addr = utils::new_socket_addr();

    // simulate rpcchainvm http service creation for handler 
    for (_, handler) in handlers {
        let http_service = avalanche_types::rpcchainvm::http::server::Server::new(handler.handler);
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
    println!("{}", json_response);
}
