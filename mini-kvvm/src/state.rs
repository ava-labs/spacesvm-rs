use std::io::{Error, ErrorKind, Result};

use avalanche_types::{
    choices::status::Status,
    ids::Id,
    rpcchainvm::{self, database},
};
pub use bytes::*;

use crate::block::{Block, MiniKvvmBlock};
use crate::kvvm::ChainVm;

const LAST_ACCEPTED_BLOCK_ID_KEY: &[u8] = b"last_accepted";
const STATE_INITIALIZED_KEY: &[u8] = b"state_initialized";
const STATE_INITIALIZED_VALUE: &[u8] = b"state_has_infact_been_initialized";
const SINGLETON_STATE_PREFIX: &[u8] = b"singleton";

pub const BLOCK_DATA_LEN: usize = 32;
pub const BLOCK_STATE_PREFIX: &[u8] = b"blockStatePrefix";

pub struct State {
    client: Option<database::manager::versioned_database::VersionedDatabase>,
    last_accepted_block_id_key: Vec<u8>,
    state_initialized_key: Vec<u8>,
}

impl State {
    pub fn new(client: Option<database::manager::versioned_database::VersionedDatabase>) -> Self {
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
        let client = self.client.clone().ok_or(Error::new(
            ErrorKind::Other,
            "no database associated with this client",
        ))?;

        let client = client.inner.write().await;
        let resp = client.get(key.as_slice()).await;

        log::info!("state get response: {:?}", resp);

        let err = match &resp {
            Ok(_) => database::DatabaseError::None as u32,
            Err(e) => rpcchainvm::database::rpcdb::error_to_error_code(&e.to_string()).unwrap(),
        };
        let err = num_traits::FromPrimitive::from_u32(err);

        match err {
            Some(database::DatabaseError::Closed) => Err(Error::new(
                ErrorKind::Other,
                format!("failed to get: {:?}", err),
            )),
            Some(database::DatabaseError::NotFound) => Ok(None),
            _ => {
                let resp = resp.map_err(|e| Error::new(ErrorKind::Other, format!("{:?}", e)))?; //this should never panic, but this handles errors in case it should happen
                Ok(Some(resp))
            }
        }
    }

    pub async fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        let client = self.client.clone().ok_or(Error::new(
            ErrorKind::Other,
            "no database associated with this client",
        ))?;
        let mut client = client.inner.write().await;
        let resp = client.put(key.as_slice(), value.as_slice()).await;

        let err = match &resp {
            Ok(_) => database::DatabaseError::None as u32,
            Err(e) => rpcchainvm::database::rpcdb::error_to_error_code(&e.to_string()).unwrap(),
        };

        let err = num_traits::FromPrimitive::from_u32(err);

        match err {
            Some(database::DatabaseError::None) => Ok(()),
            Some(database::DatabaseError::Closed) => Err(Error::new(
                ErrorKind::Other,
                format!("failed to put: {:?}", err),
            )),
            Some(database::DatabaseError::NotFound) => Err(Error::new(
                ErrorKind::NotFound,
                format!("failed to put: {:?}", err),
            )),
            _ => {
                resp.map_err(|e| Error::new(ErrorKind::Other, format!("{:?}", e)))?; //this should never panic, but this handles errors in case it should happen
                Ok(())
            }
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
        //log::info!("state get_block value: {:?}", block); //TODO implement debug block trait

        Ok(block)
    }

    pub async fn put_block(&mut self, mut block: impl MiniKvvmBlock, vm: ChainVm) -> Result<()> {
        let value = serde_json::to_vec(&block)?;
        let key = Self::prefix(BLOCK_STATE_PREFIX, block.initialize(vm)?.as_ref());
        self.put(key, value).await
    }

    pub async fn has_last_accepted_block(&self) -> Result<bool> {
        let last = self.get_last_accepted_block_id().await?;
        if last.is_some() {
            return Ok(true);
        }
        Ok(false)
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

    pub async fn accept_block(&mut self, mut block: impl MiniKvvmBlock, vm: ChainVm) -> Result<Id> {
        block.set_status(Status::Accepted);
        let block_id = block
            .initialize(vm.clone())
            .map_err(|e| Error::new(ErrorKind::Other, format!("failed to init block: {:?}", e)))?;
        log::info!("accepting block with id: {}", block_id);
        self.put_block(block, vm).await?;
        self.set_last_accepted_block_id(&block_id).await?;

        Ok(block_id)
    }
}
