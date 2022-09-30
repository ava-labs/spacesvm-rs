use std::thread;

use avalanche_types::rpcchainvm::{
    self,
    common::vm::Vm,
    database::{
        manager::{versioned_database, DatabaseManager},
        memdb::Database as MemDb,
    },
};
use jsonrpc_core::futures::{self, FutureExt};
use jsonrpc_core_client::transports::local;
use mini_kvvm::{
    api,
    api::*,
    block::{self, builder},
    chain::{tx::tx::TransactionType, tx::unsigned},
    genesis::Genesis,
    vm,
};
use tokio::time::Duration;
use tokio::{sync::mpsc, time::sleep};

#[tokio::test]
async fn vm_test() {
    let db = MemDb::new();
    let mut vm = &mut vm::ChainVm::new();
    let mut versioned_dbs: Vec<versioned_database::VersionedDatabase> = Vec::with_capacity(1);
    versioned_dbs.push(versioned_database::VersionedDatabase::new(
        db,
        semver::Version::parse("0.0.7").unwrap(),
    ));
    let db_manager = DatabaseManager::new_from_databases(versioned_dbs);

    let genesis_bytes =
        "{\"author\":\"subnet creator\",\"welcome_message\":\"Hello from Rust VM!\"}".as_bytes();

    // setup engine channel
    let (tx_engine, mut rx_engine): (
        mpsc::Sender<rpcchainvm::common::message::Message>,
        mpsc::Receiver<rpcchainvm::common::message::Message>,
    ) = mpsc::channel(100);

    let mut events = 0;
    tokio::spawn(async move {
        // wait for a channel to have a message
        loop {
            tokio::select! {
                _ = rx_engine.recv() => {
                    events += 1
                }
            }
        }
    });

    // init vm
    let resp = vm
        .initialize(
            None,
            db_manager,
            genesis_bytes,
            &[],
            &[],
            tx_engine,
            &[],
            rpcchainvm::common::appsender::client::Client::new(crate::common::create_conn().await),
        )
        .await;

    sleep(Duration::from_secs(35)).await;

    assert!(resp.is_ok());

    // last test
    assert_eq!(events, 0);
}
