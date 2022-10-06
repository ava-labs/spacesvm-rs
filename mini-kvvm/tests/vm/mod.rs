use std::{
    io::{Error, ErrorKind},
    thread,
};

use avalanche_types::rpcchainvm::{self, common::vm::Vm, utils};
use mini_kvvm::{
    block::{self, builder},
    chain::{tx::tx::TransactionType, tx::unsigned},
    genesis::Genesis,
    vm,
};
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::time::Duration;
use tokio::{sync::mpsc, time::sleep};
use tonic::transport::Channel;

use crate::common;

#[tokio::test]
async fn vm_test() {
    use crate::common::test_data;

    let vm = vm::ChainVm::new();

    // NOTE: order is important! static handlers will be called before the vm
    // is initialized.
    let resp = vm.create_static_handlers().await;
    assert!(resp.is_ok());
    let handlers = resp.unwrap();

    let (_) = common::initialize_vm(vm);
        // let handlers = resp.unwrap();

    // setup stop channel for grpc services.
    let (stop_ch_tx, _): (Sender<()>, Receiver<()>) = tokio::sync::broadcast::channel(1);

    let addr = utils::new_socket_addr();
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

    let data = test_data().as_bytes().to_vec();
    let client_conn = Channel::builder(format!("http://{}", addr).parse().unwrap())
        .connect()
        .await
        .unwrap();

    let mut client = avalanche_types::rpcchainvm::http::client::Client::new(client_conn);

    let req = http::request::Builder::new().body(data).unwrap();

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

    let t = resp.body().to_owned();

    let out = std::str::from_utf8(&t).unwrap();

    println!("{}", out)
}
