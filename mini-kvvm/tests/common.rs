use std::{
    io::{Error, ErrorKind, Result},
    sync::Arc,
    time::Duration,
};

use avalanche_types::{
    self,
    choices::status,
    ids,
    rpcchainvm::{
        self,
        common::vm::Vm,
        concensus::snowman::{Block, Initializer},
        database::manager::{versioned_database, DatabaseManager},
    },
};
use mini_kvvm::{
    block::{self, state::State},
    vm::runner::Bootstrap,
};
use serde::Deserialize;
use tokio::{
    net::TcpListener,
    sync::{mpsc, RwLock},
};
use tonic::transport::Channel;

use mini_kvvm::vm;

/// Returns genesis block for use in testing.
pub async fn create_genesis_block(state: &State, data: Vec<u8>) -> Result<ids::Id> {
    let mut genesis_block = block::Block::new(ids::Id::empty(), 0, &data, 0, state.to_owned());

    let bytes = genesis_block
        .to_bytes()
        .await
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
    genesis_block
        .init(&bytes, status::Status::Accepted)
        .await
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

    let genesis_block_id = genesis_block.id;
    state
        .set_last_accepted(genesis_block)
        .await
        .map_err(|e| Error::new(ErrorKind::Other, format!("failed to accept block: {:?}", e)))?;

    log::debug!("initialized from genesis block: {:?}\n", genesis_block_id);

    Ok(genesis_block_id)
}

pub async fn create_conn() -> Channel {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    Channel::builder(format!("http://{}", addr).parse().unwrap())
        .connect()
        .await
        .unwrap()
}

pub async fn initialize_vm() -> Result<(
    vm::ChainVm,
    mpsc::Receiver<rpcchainvm::common::message::Message>,
)> {
    let db = rpcchainvm::database::memdb::Database::new();

    let vm = &mut vm::ChainVm::new(Bootstrap {
        name: "test-vm".to_owned(),
        log_level: "debug".to_owned(),
        version: semver::Version::parse("0.0.0").unwrap(),
        testing: true,
    });

    let mut versioned_dbs: Vec<versioned_database::VersionedDatabase> = Vec::with_capacity(1);
    versioned_dbs.push(versioned_database::VersionedDatabase::new(
        db,
        semver::Version::parse("0.0.7").unwrap(),
    ));
    let db_manager = DatabaseManager::new_from_databases(versioned_dbs);

    let genesis_bytes =
        "{\"author\":\"subnet creator\",\"welcome_message\":\"Hello from Rust VM!\"}".as_bytes();

    // setup engine channel
    let (tx_engine, rx_engine): (
        mpsc::Sender<rpcchainvm::common::message::Message>,
        mpsc::Receiver<rpcchainvm::common::message::Message>,
    ) = mpsc::channel(100);

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
            rpcchainvm::common::appsender::client::Client::new(create_conn().await),
        )
        .await;


    Ok((vm.to_owned(), rx_engine))
}

pub(crate) fn test_data2() -> &'static str {
    let data = r#"
    {
  "jsonrpc": "2.0",
  "method": "issue_tx",
  "params": [
    {
      "signature": [
        202,
        194,
        218,
        188,
        97,
        16,
        222,
        37,
        32,
        242,
        249,
        6,
        121,
        103,
        3,
        217,
        129,
        32,
        211,
        90,
        126,
        11,
        19,
        94,
        125,
        139,
        137,
        121,
        242,
        171,
        65,
        23,
        50,
        121,
        2,
        180,
        194,
        238,
        5,
        77,
        225,
        188,
        145,
        253,
        235,
        241,
        41,
        105,
        142,
        83,
        160,
        126,
        81,
        74,
        174,
        251,
        217,
        212,
        236,
        2,
        222,
        250,
        24,
        246,
        28
      ],
      "typed_data": {
        "domain": {
          "magic": "0x00",
          "name": "MiniKvvm"
        },
        "message": {
          "blockId": "0000000000000000000000000000000000000000000000000000000000000000",
          "bucket": "666f6f"
        },
        "primary_type": {
          "type": "Bucket"
        },
        "types": {
          "EIP712Domain": [
            {
              "name": "name",
              "type": "string"
            },
            {
              "name": "magic",
              "type": "uint64"
            }
          ],
          "bucket": [
            {
              "name": "bucket",
              "type": "string"
            },
            {
              "name": "blockId",
              "type": "string"
            }
          ]
        }
      }
    }
  ],
  "id": 1
}"#;
    data
}

pub(crate) fn test_data() -> &'static str {
    let data = r#"
    {
  "jsonrpc": "2.0",
  "method": "issue_tx",
  "params": [
    {
      "typed_data": {
        "domain": {
          "magic": "0x00",
          "name": "MiniKvvm"
        },
        "message": {
          "blockId": "0000000000000000000000000000000000000000000000000000000000000000",
          "bucket": "666f6f"
        },
        "primary_type": {
          "type": "Bucket"
        },
        "types": {
          "EIP712Domain": [
            {
              "name": "name",
              "type": "string"
            },
            {
              "name": "magic",
              "type": "uint64"
            }
          ],
          "bucket": [
            {
              "name": "bucket",
              "type": "string"
            },
            {
              "name": "blockId",
              "type": "string"
            }
          ]
        }
      }
    }
  ],
  "id": 1
}"#;
    data
}
