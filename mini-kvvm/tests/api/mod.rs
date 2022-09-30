use std::{sync::Arc, thread, time::Duration};

use avalanche_types::rpcchainvm::{database::memdb::Database as MemDb, common::message::Message};
use jsonrpc_core::futures::{self, FutureExt};
use jsonrpc_core_client::transports::local;
use mini_kvvm::{
    api,
    api::*,
    block,
    chain::{crypto, tx::decoder, tx::tx::TransactionType, tx::unsigned},
    vm,
};
use secp256k1::{rand, SecretKey};
use tokio::sync::{RwLock, mpsc};

use crate::common::create_genesis_block;

#[tokio::test]
async fn service_test() {
    let db = MemDb::new();
    let vm = &mut vm::ChainVm::new();
    vm.db = Some(db);

    // get a broadcast tx pending receiver for new blocks;
    let pending_rx = vm.mempool_pending_rx.clone();
    // unblock channel
    thread::spawn(move || loop {
        crossbeam_channel::select! {
            recv(pending_rx) -> _ => {}
        }
    });


    let (tx_engine, mut rx_engine): (mpsc::Sender<Message>, mpsc::Receiver<Message>) = mpsc::channel(100);

            tokio::spawn(async move {
                loop {
                    let _ = rx_engine.recv().await;
                }
            });

    // initialize genesis block
    let mut inner = vm.inner.write().await;
    inner.state = block::state::State::new(vm.db.as_ref().unwrap().clone());
    let resp = create_genesis_block(&inner.state.clone(), vec![]).await;
    assert!(resp.is_ok());
    inner.preferred = resp.unwrap();
    inner.to_engine = Some(tx_engine);
    vm.builder = Some(block::builder::Timed {
        mempool_pending_ch: vm.mempool_pending_rx.clone(),
        vm_mempool: vm.mempool.clone(),
        vm_network: None,
        vm_engine_tx: inner.to_engine.as_ref().unwrap().clone(),
        vm_builder_stop_rx: vm.builder_stop_rx.clone(),
        vm_stop_rx: vm.stop_rx.clone(),
        build_block_timer: block::builder::Timer::new(),
        build_interval: Duration::from_millis(1),
        status: Arc::new(RwLock::new(block::builder::Status::DontBuild)),
    });
    drop(inner);

    let service = api::service::Service::new(vm.to_owned());
    let mut io = jsonrpc_core::IoHandler::new();
    io.extend_with(service.to_delegate());
    let (client, server) = local::connect(io);

    futures::executor::block_on(async {
        let client = test_rpc_client(client).fuse();
        let server = server.fuse();

        futures::pin_mut!(client);
        futures::pin_mut!(server);

        futures::select! {
            _server = server => {},
            _client = client => {},
        }
    });
}

async fn test_rpc_client(client: gen_client::Client) {
    // ping
    let resp = client.ping().await;
    assert!(resp.is_ok());
    assert!(resp.unwrap().success);

    // bucket tx: create kvs bucket
    let tx_data = unsigned::TransactionData {
        typ: TransactionType::Bucket,
        bucket: "kvs".to_string(),
        key: "".to_string(),
        value: vec![],
    };

    let resp = client.decode_tx(DecodeTxArgs { tx_data }).await;
    assert!(resp.is_ok());

    let secret_key = SecretKey::new(&mut rand::thread_rng());
    let typed_data = resp.unwrap().typed_data;
    let dh = decoder::hash_structured_data(&typed_data).unwrap();
    let signature = crypto::sign(&dh.as_bytes(), &secret_key).unwrap();

    let resp = client
        .issue_tx(IssueTxArgs {
            typed_data,
            signature,
        })
        .await;
    assert!(resp.is_ok());

    // set tx: add kv pair to kvs bucket
    let tx_data = unsigned::TransactionData {
        typ: TransactionType::Set,
        bucket: "kvs".to_string(),
        key: "foo".to_string(),
        value: "bar".as_bytes().to_vec(),
    };

    let resp = client.decode_tx(DecodeTxArgs { tx_data }).await;
    assert!(resp.is_ok());

    let typed_data = resp.unwrap().typed_data;
    let dh = decoder::hash_structured_data(&typed_data).unwrap();
    let signature = crypto::sign(&dh.as_bytes(), &secret_key).unwrap();

    let resp = client
        .issue_tx(IssueTxArgs {
            typed_data,
            signature,
        })
        .await;
    assert!(resp.is_ok());

    // build block
    let resp = client.build_block(BuildBlockArgs {}).await;
    assert!(resp.is_ok());

    // resolve: check kvs bucket for foo key value
    let args = ResolveArgs {
        bucket: "kvs".as_bytes().to_vec(),
        key: "foo".as_bytes().to_vec(),
    };
    let resp = client.resolve(args).await;
    assert!(resp.is_ok());
    let value = resp.unwrap().value;
    assert_eq!(value, "bar".as_bytes());

    // delete tx:
    let tx_data = unsigned::TransactionData {
        typ: TransactionType::Delete,
        bucket: "kvs".to_string(),
        key: "foo".to_string(),
        value: vec![],
    };

    let resp = client.decode_tx(DecodeTxArgs { tx_data }).await;
    assert!(resp.is_ok());

    let typed_data = resp.unwrap().typed_data;
    let dh = decoder::hash_structured_data(&typed_data).unwrap();
    let signature = crypto::sign(&dh.as_bytes(), &secret_key).unwrap();

    let resp = client
        .issue_tx(IssueTxArgs {
            typed_data,
            signature,
        })
        .await;
    assert!(resp.is_ok());

    // build block
    let resp = client.build_block(BuildBlockArgs {}).await;
    assert!(resp.is_ok());

    // resolve: check kvs bucket for foo key value
    let args = ResolveArgs {
        bucket: "kvs".as_bytes().to_vec(),
        key: "foo".as_bytes().to_vec(),
    };
    let resp = client.resolve(args).await;
    assert!(resp.unwrap_err().to_string().contains("not found"));
}
