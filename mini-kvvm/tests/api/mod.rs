use std::thread;

use avalanche_types::rpcchainvm::database::memdb::Database as MemDb;
use jsonrpc_core::futures::{self, FutureExt};
use jsonrpc_core_client::transports::local;
use mini_kvvm::{
    api,
    api::*,
    chain::{crypto, tx::decoder, tx::tx::TransactionType, tx::unsigned},
    vm,
};
use secp256k1::{rand, SecretKey};

use crate::common::create_genesis_block;

#[tokio::test]
async fn service_test() {
    let db = MemDb::new();
    let vm = vm::ChainVm::new_with_state(&db);

    // get a broadcast tx pending receiver for new blocks;
    let mempool = vm.mempool.read().await;
    let pending_rx = mempool.subscribe_pending();
    drop(mempool);
    // unblock channel
    thread::spawn(move || loop {
        crossbeam_channel::select! {
            recv(pending_rx) -> _ => {}
        }
    });

    // initialize genesis block
    let mut inner = vm.inner.write().await;
    let resp = create_genesis_block(&inner.state.clone(), vec![]).await;
    assert!(resp.is_ok());
    inner.preferred = resp.unwrap();
    drop(inner);

    let service = api::service::Service::new(vm);
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
