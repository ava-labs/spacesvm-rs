use std::io::{Error, ErrorKind};

use avalanche_proto::rpcdb::{database_client::*, GetRequest, PutRequest};
use avalanche_types::ids::Id;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use tonic::transport::Channel;
use chrono::{DateTime, NaiveDateTime, Utc};


use crate::block::{Block, Status};

pub type Database = DatabaseClient<Channel>;

const LAST_ACCEPTED_BLOCK_ID_KEY: &[u8] = b"last_accepted_key";
const STATE_INITIALIZED_KEY: &[u8] = b"state_initialized";
const STATE_INITIALIZED_VALUE: &[u8] = b"state_has_infact_been_initialized";
// const BLOCK_STATE_PREFIX: &[u8] = b"blockStatePrefix";
const BLOCK_STATE_PREFIX: &[u8] = b"snowman_accepted";
const SINGLETON_STATE_PREFIX: &[u8] = b"singleton";

pub const BLOCK_DATA_LEN: usize = 32;

/// snow.engine.common.AppHandler
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/vms/components/state#State
// #[tonic::async_trait]
// pub trait State<'a> {
//     async fn get(&'a self, key: Id) -> Result<Option<Vec<u8>>, Error>;
//     async fn put(&'a mut self, key: Id, value: Vec<u8>) -> Result<(), Error>;
//     async fn get_block(&'a mut self, id: Id) -> Result<Option<Block>, Error>;
//     async fn get_last_accepted_block_id(&'a mut self) -> Result<Option<Id>, Error>;
// }

// #[derive(Debug)]
pub struct State {
    client: Database,

    last_accepted_block_id_key: Vec<u8>,
    state_initialized_key: Vec<u8>,
}

impl State {
    pub fn new(client: Database) -> Self {
        Self {
            client,
            last_accepted_block_id_key: Self::prefix(
                BLOCK_STATE_PREFIX,
                LAST_ACCEPTED_BLOCK_ID_KEY,
            ),
            state_initialized_key: Self::prefix(SINGLETON_STATE_PREFIX, STATE_INITIALIZED_KEY),
        }
    }
    fn prefix(prefix: &[u8], data: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(prefix.len() + data.len());
        result.extend_from_slice(prefix);
        result.extend_from_slice(data);

        result
    }

    pub async fn get(&mut self, key: Id) -> Result<Option<Vec<u8>>, Error> {
        log::info!("get 1");
        let key = prost::bytes::Bytes::from(Vec::from(key.as_ref()));
        log::info!("get 2 key: {:?}", key);

        let mut client = self.client.clone();
        let resp = client.get(GetRequest { key }).await.unwrap().into_inner();
        log::info!("get 3 resp: {:?}", resp);

        let err = DatabaseError::from_u32(resp.err);
        match err {
            Some(DatabaseError::Closed) => Err(Error::new(
                ErrorKind::Other,
                format!("failed to get: {:?}", err),
            )),
            Some(DatabaseError::NotFound) => Ok(None),
            _ => Ok(Some(Vec::from(resp.value.as_ref()))),
        }
    }

    pub async fn put(&mut self, key: Id, value: Vec<u8>) -> Result<(), Error> {
        let key = prost::bytes::Bytes::from(Vec::from(key.as_ref()));
        let value = prost::bytes::Bytes::from(value);
        let mut client = self.client.clone();

        let resp = client
            .put(PutRequest { key, value })
            .await
            .unwrap()
            .into_inner();

        let err = DatabaseError::from_u32(resp.err);
        match err {
            Some(DatabaseError::None) => Ok(()),
            Some(DatabaseError::Closed) => Err(Error::new(
                ErrorKind::Other,
                format!("failed to put: {:?}", err),
            )),
            Some(DatabaseError::NotFound) => Err(Error::new(
                ErrorKind::NotFound,
                format!("failed to put: {:?}", err),
            )),
            _ => Err(Error::new(
                ErrorKind::Other,
                format!("failed to put: {:?}", resp.err),
            )),
        }
    }

    pub async fn get_block(&mut self, id: Id) -> Result<Option<Block>, Error> {
        let key = Self::prefix(BLOCK_STATE_PREFIX, id.as_ref());
        let value = self.get(Id::from_slice(&key)).await?;

        Ok(match value {
            Some(v) => serde_json::from_slice(&v)?,
            None => None,
        })
    }

    pub async fn get_last_accepted_block_id(&mut self) -> Result<Option<Id>, Error> {
        log::info!("get_last_accepted_block_id 1");

        let key = &self.last_accepted_block_id_key.clone();
        log::info!("get_last_accepted_block_id key: {:?}", key);
        log::info!("get_last_accepted_block_id key len: {:?}", key.len());

        let block = self.get(Id::from_slice(key)).await?;
        log::info!("get_last_accepted_block_id: {:?}", block);
        match block {
            Some(block_id_bytes) => Ok(Some(Id::from_slice(&block_id_bytes))),
            None => Ok(None),
        }
    }

    //  pub async fn is_state_initialized(&mut self) -> Result<bool, Error> {
    //     let maybe_state_initialized_bytes = self.get(self.state_initialized_key.clone()).await?;

    //     Ok(match maybe_state_initialized_bytes {
    //         Some(state_initialized_bytes) => !state_initialized_bytes.is_empty(),
    //         None => false,
    //     })
    // }

     pub async fn init_genesis(&mut self, genesis_bytes: &[u8]) -> Result<(), Error> {
        log::trace!("initialize genesis called");

        // if self.is_state_initialized().await? {
        //     // State is already initialized - no need to init genesis block
        //     log::info!("state is already initialized. No further work to do.");
        //     return Ok(());
        // }

        // if genesis_bytes.len() > BLOCK_DATA_LEN {
        //     return Err(LandslideError::Other(anyhow!(
        //         "Genesis data byte length {} is greater than the expected block byte length of {}. Genesis bytes: {:#?} as a string: {}",
        //         genesis_bytes.len(),
        //         BLOCK_DATA_LEN,
        //         genesis_bytes,
        //         String::from_utf8(Vec::from(genesis_bytes)).unwrap(),
        //     )));
        // }

        let mut padded_genesis_vec = Vec::from(genesis_bytes);
        padded_genesis_vec.resize(BLOCK_DATA_LEN, 0);

        let padded_genesis_data: [u8; BLOCK_DATA_LEN] = padded_genesis_vec.as_slice().try_into().unwrap();

        log::info!(
            "Genesis block created with length {} by padding up from data length {}",
            padded_genesis_data.len(),
            genesis_bytes.len()
        );
        let mut genesis_block = Block::new(
            Id::default(),
            0,
            padded_genesis_data,
            DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc),
            Status::Processing,
        )?;

         log::info!(
            "Genesis storage block created with Id: {:?}",
            genesis_block
        );



        // let genesis_block_id = genesis_block.generate_id()?.clone();

        // log::info!(
        //     "Genesis storage block created with Id: {}",
        //     genesis_block_id
        // );
        // state.put_block(genesis_block.clone()).await?;
        // log::info!(
        //     "Genesis storage block with Id {} put in database successfully.",
        //     genesis_block_id
        // );
        // self.accept_block(genesis_block).await?;
        // log::info!(
        //     "Genesis storage block with Id {} was accepted by this node.",
        //     genesis_block_id
        // );

        // // reacquire state since we need to release writable_interior to pass into accept_block
        // let state = self.mut_state_status().await?;
        // state.set_state_initialized().await?;
        // log::info!("State set to initialized, so it won't hapen again.");

        Ok(())
    }
}

/// database/errors
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/database#ErrClosed
#[derive(Debug, FromPrimitive, Clone, Copy)]
pub enum DatabaseError {
    None = 0,
    Closed = 1,
    NotFound = 2,
}
