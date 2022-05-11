use avalanche_proto::rpcdb::{database_client::*, GetRequest, PutRequest};
use avalanche_types::ids;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::io::{Error, ErrorKind};
use tonic::transport::Channel;

use crate::block::Block;

pub type Database = DatabaseClient<Channel>;

const LAST_ACCEPTED_BLOCK_ID_KEY: &[u8] = b"last_accepted_block_id";
const STATE_INITIALIZED_KEY: &[u8] = b"state_initialized";
const STATE_INITIALIZED_VALUE: &[u8] = b"state_has_infact_been_initialized";
const BLOCK_STATE_PREFIX: &[u8] = b"blockStatePrefix";
const SINGLETON_STATE_PREFIX: &[u8] = b"singleton";

pub const BLOCK_DATA_LEN: usize = 32;

/// snow.engine.common.AppHandler
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/vms/components/state#State
#[tonic::async_trait]
pub trait State<'a> {
    async fn get(&'a self, key: ids::Id) -> Result<Option<Vec<u8>>, Error>;
    async fn put(&'a mut self, key: ids::Id, value: Vec<u8>) -> Result<(), Error>;
    async fn get_block(&'a mut self, id: ids::Id) -> Result<Option<Block>, Error>;
}

#[derive(Debug)]
pub struct Interior<'a> {
    client: &'a Database,

    last_accepted_block_id_key: Vec<u8>,
    state_initialized_key: Vec<u8>,
}

impl<'a> Interior<'a> {
    pub fn new(client: &'a Database) -> Self {
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
}

#[tonic::async_trait]
impl<'a> State<'a> for Interior<'a> {
    async fn get(&'a self, key: ids::Id) -> Result<Option<Vec<u8>>, Error> {
        let key = prost::bytes::Bytes::from(Vec::from(key.as_ref()));
        let mut client = self.client.clone();
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

    async fn put(&'a mut self, key: ids::Id, value: Vec<u8>) -> Result<(), Error> {
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

    async fn get_block(&'a mut self, id: ids::Id) -> Result<Option<Block>, Error> {
        let key = Self::prefix(BLOCK_STATE_PREFIX, id.as_ref());
        let value = self.get(ids::Id::from_slice(&key)).await?;

        Ok(match value {
            Some(v) => serde_json::from_slice(&v)?,
            None => None,
        })
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
