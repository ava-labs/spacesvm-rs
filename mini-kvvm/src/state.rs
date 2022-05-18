use std::io::{Error, ErrorKind};

use avalanche_proto::rpcdb::{database_client::*, GetRequest, PutRequest};
use avalanche_types::ids::Id;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use tonic::transport::Channel;

use crate::block::{Block, Status};

pub type Database = DatabaseClient<Channel>;

const LAST_ACCEPTED_BLOCK_ID_KEY: &[u8] = b"last_accepted";
const STATE_INITIALIZED_KEY: &[u8] = b"state_initialized";
const STATE_INITIALIZED_VALUE: &[u8] = b"state_has_infact_been_initialized";
const BLOCK_STATE_PREFIX: &[u8] = b"blockStatePrefix";
const SINGLETON_STATE_PREFIX: &[u8] = b"singleton";

pub const BLOCK_DATA_LEN: usize = 32;

#[derive(Debug, Default)]
pub struct State {
    client: Option<Database>,
    last_accepted_block_id_key: Vec<u8>,
    state_initialized_key: Vec<u8>,
}

impl State {
    pub fn new(client: Option<Database>) -> Self {
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

    pub async fn get(&mut self, key: Vec<u8>) -> Result<Option<Vec<u8>>, Error> {
        let key = prost::bytes::Bytes::from(key);
        let mut client = self.client.clone().unwrap();
        let resp = client.get(GetRequest { key }).await.unwrap().into_inner();
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

    pub async fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<(), Error> {
        let key = prost::bytes::Bytes::from(key);
        let value = prost::bytes::Bytes::from(value);
        let mut client = self.client.clone().unwrap();

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
        let value = self.get(key).await?;

        Ok(match value {
            Some(v) => serde_json::from_slice(&v)?,
            None => None,
        })
    }

    pub async fn put_block(&mut self, mut block: Block) -> Result<(), Error> {
        let value = serde_json::to_vec(&block)?;
        let key = Self::prefix(BLOCK_STATE_PREFIX, block.init()?.as_ref());

        log::info!("put_block key {:?}", key);

        self.put(key, value).await
    }

    pub async fn has_last_accepted_block(&mut self) -> Result<bool, Error> {
        let last = self.get_last_accepted_block_id().await?;

        Ok(match last {
            Some(last_accepted_block) => !last_accepted_block.is_empty(),
            None => false,
        })
    }

    pub async fn get_last_accepted_block_id(&mut self) -> Result<Option<Id>, Error> {
        match self.get(self.last_accepted_block_id_key.clone()).await? {
            Some(block_id_bytes) => Ok(Some(Id::from_slice(&block_id_bytes))),
            None => Ok(None),
        }
    }

    pub async fn set_last_accepted_block_id(&mut self, id: &Id) -> Result<(), Error> {
        log::info!("Setting last accepted block id bytes: {:?}", id.as_ref());
        self.put(
            self.last_accepted_block_id_key.clone(),
            Vec::from(id.as_ref()),
        )
        .await
    }

    pub async fn is_state_initialized(&mut self) -> Result<bool, Error> {
        let state = self.get(self.state_initialized_key.clone()).await?;
        Ok(match state {
            Some(state_initialized_bytes) => !state_initialized_bytes.is_empty(),
            None => false,
        })
    }

    pub async fn set_state_initialized(&mut self) -> Result<(), Error> {
        self.put(
            self.state_initialized_key.clone(),
            Vec::from(STATE_INITIALIZED_VALUE),
        )
        .await
    }

    pub async fn accept_block(&mut self, mut block: Block) -> Result<Id, Error> {
        block.status = Status::Accepted;
        let bid = block.init()?.clone();
        log::info!("Accepting block with id: {}", bid);

        self.put_block(block).await?;
        log::info!("Put accepted block into database with id: {}", bid);

        self.set_last_accepted_block_id(&bid).await?;
        log::info!("Setting last accepted block id in database to: {}", bid);

        Ok(bid)
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
