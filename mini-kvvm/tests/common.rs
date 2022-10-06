use std::io::{Error, ErrorKind, Result};

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
use mini_kvvm::block::{self, state::State};
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

pub async fn initialize_vm(mut vm: vm::ChainVm) -> Result<mpsc::Receiver<rpcchainvm::common::message::Message>> {
    let db = rpcchainvm::database::memdb::Database::new();

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

        assert!(resp.is_ok());

    Ok(rx_engine)
}

pub(crate) fn test_data() -> &'static str {
    let data = r#"
    {
      "jsonrpc": "2.0",
      "method": "issue_tx",
      "params": [
        {
          "signature": "fc493984569024997814a03662d1a76d3fc0e896d528e19d31ec503a1ef2c3191dfd75af4feac1dc11d8d1195cb88567cde1b1c09a6efb85524abfd6aecfd56a1b",
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
      "id": 2
    }
   "#;
    data
}

pub(crate) fn test_data3() -> &'static str {
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

