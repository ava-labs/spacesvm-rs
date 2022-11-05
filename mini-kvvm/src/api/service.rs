use std::{io::ErrorKind, sync::Arc};

use crate::{
    api::*,
    chain::{self, storage, tx::Transaction, vm::Vm},
    vm::{self, inner::Inner},
};

use tokio::sync::RwLock;
pub struct Service {
    pub vm_inner: Arc<RwLock<Inner>>,
}

impl Service {
    pub fn new(vm_inner: Arc<RwLock<Inner>>) -> Self {
        Self { vm_inner }
    }
}

impl crate::api::Service for Service {
    /// Returns true if the API is serving requests.
    fn ping(&self) -> BoxFuture<Result<PingResponse>> {
        log::debug!("ping called");

        Box::pin(async move { Ok(PingResponse { success: true }) })
    }

    /// Takes tx args and returns the tx id.
    fn issue_tx(&self, params: IssueTxArgs) -> BoxFuture<Result<IssueTxResponse>> {
        log::debug!("issue tx called");
        let vm = Arc::clone(&self.vm_inner);

        Box::pin(async move {
            let mut inner = vm.write().await;
            log::info!("params_ typed: {:?}", params.typed_data);

            let unsigned_tx = params
                .typed_data
                .parse_typed_data()
                .map_err(create_jsonrpc_error)?;
            log::info!("unsigned");
            // let sig_bytes = hex::decode(params.signature).map_err(|e| {
            //     create_jsonrpc_error(std::io::Error::new(
            //         std::io::ErrorKind::Other,
            //         e.to_string(),
            //     ))
            // })?;
            log::info!("sig: {:?}", params.signature);
            // log::info!("sig bytes");

            let mut tx = chain::tx::tx::Transaction::new(unsigned_tx, params.signature);
            tx.init().await.map_err(create_jsonrpc_error)?;
            let tx_id = tx.id().await;

            let mut txs = Vec::with_capacity(1);
            txs.push(tx);

            log::info!("issue_tx: submit");
            storage::submit(&inner.state, &mut txs).await.map_err(|e| {
                create_jsonrpc_error(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

            log::info!("issue_tx add to mempool");

            for tx in txs.iter().cloned() {
                let mempool = &mut inner.mempool;
                let out = mempool.add(tx).map_err(|e| {
                    create_jsonrpc_error(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string(),
                    ))
                })?;
                log::info!("issue_tx add to mempool: {}", out);
            }

            Ok(IssueTxResponse { tx_id })
        })
    }

    fn decode_tx(&self, params: DecodeTxArgs) -> BoxFuture<Result<DecodeTxResponse>> {
        log::debug!("decode input called");
        let vm = Arc::clone(&self.vm_inner);

        Box::pin(async move {
            let mut utx = params.tx_data.decode().map_err(create_jsonrpc_error)?;
            let inner = vm.write().await;
            let last_accepted = &inner.last_accepted;

            utx.set_block_id(last_accepted.id).await;
            let typed_data = utx.typed_data().await;

            let string = serde_json::to_string(&typed_data).unwrap();

            log::info!("decode_tx: {}", string);

            Ok(DecodeTxResponse { typed_data })
        })
    }

    fn resolve(&self, params: ResolveArgs) -> BoxFuture<Result<ResolveResponse>> {
        log::info!("resolve: called");
        let vm = Arc::clone(&self.vm_inner);

        Box::pin(async move {
            let inner = vm.read().await;
            let db = inner.state.get_db().await;
            let value = chain::storage::get_value(&db, &params.bucket, &params.key)
                .await
                .map_err(create_jsonrpc_error)?;
            if value.is_none() {
                return Ok(ResolveResponse::default());
            }

            let meta = chain::storage::get_value_meta(&db, &params.bucket, &params.key)
                .await
                .map_err(create_jsonrpc_error)?;
            if meta.is_none() {
                return Ok(ResolveResponse::default());
            }

            Ok(ResolveResponse {
                exists: true,
                value: value.unwrap(),
                meta: meta.unwrap(),
            })
        })
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn service_test() {
    use crate::api::Service;
    use crate::{block, vm};
    use avalanche_proto::grpcutil;
    use avalanche_types::rpcchainvm::database;
    use secp256k1::{rand, PublicKey, SecretKey};
    use tokio_stream::wrappers::TcpListenerStream;

    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug"),
    );

    // init and start memdb backed rpcdb server
    let server = database::rpcdb::server::Server::new(database::memdb::Database::new());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        grpcutil::default_server()
            .add_service(avalanche_proto::rpcdb::database_server::DatabaseServer::new(server))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .unwrap();
    });

    // create rpcdb client
    let client_conn =
        tonic::transport::Channel::builder(format!("http://{}", addr).parse().unwrap())
            .connect()
            .await
            .unwrap();

    // init inner
    let inner = Arc::new(RwLock::new(vm::inner::Inner::new()));
    // {
    let mut vm = inner.write().await;
    vm.state = block::state::State::new(database::rpcdb::client::DatabaseClient::new(client_conn));
    let pending_rx = vm.mempool.subscribe_pending();

    // drain the mempool pending channel
    tokio::spawn(async move {
        loop {
            let _ = pending_rx.recv();
        }
    });
    drop(vm);
    // }

    let api = self::Service::new(inner);

    // init keys
    let secret_key = SecretKey::new(&mut rand::thread_rng());
    let og_public_key = PublicKey::from_secret_key_global(&secret_key);


    //
    // create kvs bucket
    //
    let tx_data = unsigned::TransactionData {
        typ: tx::tx::TransactionType::Bucket,
        bucket: "kvs".to_owned(),
        key: String::new(),
        value: Vec::new(),
    };

    // decode
    let resp = api.decode_tx(DecodeTxArgs { tx_data }).await;
    assert!(resp.is_ok());

    let typed_data = resp.unwrap().typed_data;
    let resp = tx::decoder::hash_structured_data(&typed_data);
    assert!(resp.is_ok());
    let dh = resp.unwrap();

    let resp = chain::crypto::sign(&dh.as_bytes(), &secret_key);
    assert!(resp.is_ok());
    let signature = resp.unwrap();

    // derive public key from sig and data hash
    let resp = chain::crypto::derive_sender(dh.as_bytes(), &signature);
    assert!(resp.is_ok());
    let derived_public_key = resp.unwrap();

    // ensure derived is same as original
    assert_eq!(derived_public_key, og_public_key);

    // issue bucket tx
    let resp = api
        .issue_tx(IssueTxArgs {
            signature,
            typed_data,
        })
        .await;
    assert!(resp.is_ok());

    //
    // create key/value pair
    //

    let tx_data = unsigned::TransactionData {
        typ: tx::tx::TransactionType::Set,
        bucket: "kvs".to_owned(),
        key: "foo".to_owned(),
        value: "bar".as_bytes().to_vec(),
    };

    // decode tx
    let resp = api.decode_tx(DecodeTxArgs { tx_data }).await;
    assert!(resp.is_ok());

    let typed_data = resp.unwrap().typed_data;
    let resp = tx::decoder::hash_structured_data(&typed_data);
    assert!(resp.is_ok());

    let dh = resp.unwrap();
    let resp = chain::crypto::sign(&dh.as_bytes(), &secret_key);
    assert!(resp.is_ok());
    let signature = resp.unwrap();

    // derive public key from sig and data hash
    let resp = chain::crypto::derive_sender(dh.as_bytes(), &signature);
    assert!(resp.is_ok());
    let derived_public_key = resp.unwrap();

    // ensure derived is same as original
    assert_eq!(derived_public_key, og_public_key);

    // issue set tx
    let resp = api
        .issue_tx(IssueTxArgs {
            signature,
            typed_data,
        })
        .await;
    assert!(resp.is_ok());

    let resp = api
        .resolve(ResolveArgs {
            bucket: "kvs".as_bytes().to_vec(),
            key: "foo".as_bytes().to_vec(),
        })
        .await;
    // assert!(resp.is_ok());
    println!("err: {}", resp.unwrap_err().message)
}
