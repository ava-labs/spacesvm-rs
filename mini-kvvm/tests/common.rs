use std::io::{Error, ErrorKind, Result};

use avalanche_types::{
    choices::status,
    ids,
    rpcchainvm::concensus::snowman::{Block, Initializer},
};
use mini_kvvm::block::{self, state::State};
use tokio::net::TcpListener;
use tonic::transport::Channel;

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
