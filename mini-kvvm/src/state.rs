use std::io::{Error, ErrorKind, Result};

use avalanche_proto::rpcdb::{database_client::*, GetRequest, PutRequest};
use avalanche_types::{choices::status::Status, ids::Id};
pub use bytes::*;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use tonic::transport::Channel;

use crate::block::Block;

pub type Database = DatabaseClient<Channel>;

const LAST_ACCEPTED_BLOCK_ID_KEY: &[u8] = b"last_accepted";
const STATE_INITIALIZED_KEY: &[u8] = b"state_initialized";
const STATE_INITIALIZED_VALUE: &[u8] = b"state_has_infact_been_initialized";
const SINGLETON_STATE_PREFIX: &[u8] = b"singleton";

pub const BLOCK_DATA_LEN: usize = 32;
pub const BLOCK_STATE_PREFIX: &[u8] = b"blockStatePrefix";

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
    pub fn prefix(prefix: &[u8], data: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(prefix.len() + data.len());
        result.extend_from_slice(prefix);
        result.extend_from_slice(data);

        result
    }

    pub async fn get(&self, key: Vec<u8>) -> Result<Option<Vec<u8>>> {
        let key = prost::bytes::Bytes::from(key);
        let mut client = self.client.clone().unwrap();

        let resp = client.get(GetRequest { key }).await.unwrap().into_inner();

        log::info!("state get response: {:?}", resp);

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

    pub async fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        let key = key.into();
        let value = value.into();
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

    // Dupe of kvvm this should be removed or moved to block?
    pub async fn get_block(&self, id: Id) -> Result<Option<Block>> {
        log::debug!("state get_block called");
        let key = Self::prefix(BLOCK_STATE_PREFIX, id.as_ref());
        log::debug!("state get_block key {:?}", key);
        let value = match self.get(key).await {
            Ok(Some(v)) => v,
            _ => return Ok(None),
        };

        let block = serde_json::from_slice(&value).map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed deserialize block: {:?}", e),
            )
        })?;
        log::info!("state get_block value: {:?}", block);

        Ok(block)
    }

    pub async fn put_block(&mut self, mut block: Block) -> Result<()> {
        let value = serde_json::to_vec(&block)?;
        let key = Self::prefix(BLOCK_STATE_PREFIX, block.initialize()?.as_ref());
        self.put(key, value).await
    }

    pub async fn has_last_accepted_block(&self) -> Result<bool> {
        let last = self.get_last_accepted_block_id().await?;

        Ok(match last {
            Some(last_accepted_block) => !last_accepted_block.is_empty(),
            None => false,
        })
    }

    pub async fn get_last_accepted_block_id(&self) -> Result<Option<Id>> {
        match self.get(self.last_accepted_block_id_key.clone()).await? {
            Some(block_id_bytes) => Ok(Some(Id::from_slice(&block_id_bytes))),
            None => Ok(None),
        }
    }

    pub async fn set_last_accepted_block_id(&mut self, id: &Id) -> Result<()> {
        self.put(
            self.last_accepted_block_id_key.clone(),
            Vec::from(id.as_ref()),
        )
        .await
    }

    pub async fn is_state_initialized(&mut self) -> Result<bool> {
        let state = self.get(self.state_initialized_key.clone()).await?;
        Ok(match state {
            Some(state_initialized_bytes) => !state_initialized_bytes.is_empty(),
            None => false,
        })
    }

    pub async fn set_state_initialized(&mut self) -> Result<()> {
        self.put(
            self.state_initialized_key.clone(),
            Vec::from(STATE_INITIALIZED_VALUE),
        )
        .await
    }

    pub async fn accept_block(&mut self, mut block: Block) -> Result<Id> {
        block.status = Status::Accepted;
        let block_id = block
            .initialize()
            .map_err(|e| Error::new(ErrorKind::Other, format!("failed to init block: {:?}", e)))?;
        log::info!("accepting block with id: {}", block_id);
        self.put_block(block).await?;
        self.set_last_accepted_block_id(&block_id).await?;

        Ok(block_id)
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


