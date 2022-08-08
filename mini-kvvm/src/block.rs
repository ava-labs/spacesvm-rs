use std::{
    cmp::Ordering,
    io::{Error, ErrorKind, Result},
};

use avalanche_types::{
    choices::status::Status,
    ids::{must_deserialize_id, Id},
    rpcchainvm,
};
use avalanche_utils::rfc3339;
use chrono::{DateTime, NaiveDateTime, Utc};
use hmac_sha256::Hash;
use serde::{Deserialize, Serialize};

use crate::kvvm::ChainVm;

pub const DATA_LEN: usize = 32;

impl Block {
    pub fn new(
        parent: Id,
        height: u64,
        data: Vec<u8>,
        timestamp: DateTime<Utc>,
        status: Status,
    ) -> Self {
        Self {
            parent,
            height,
            timestamp,
            data,
            status,
            id: Id::empty(),
            bytes: Vec::default(),
            vm: None,
        }
    }
}

pub trait MiniKvvmBlock: rpcchainvm::concensus::snowman::Block + Serialize {
    fn data(&self) -> &[u8];
    fn initialize(&mut self, vm: ChainVm) -> Result<Id>;
    fn set_status(&mut self, status: Status);
}

// TODO remove
// Default is only used as a placeholder for unimplemented block logic
impl Default for Block {
    fn default() -> Self {
        Self {
            id: Id::empty(),
            parent: Id::empty(),
            timestamp: DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc),
            bytes: Vec::default(),
            height: 0,
            status: Status::Unknown("".to_string()),
            data: Vec::default(),
            vm: None,
        }
    }
}

/// snow/consensus/snowman/Block
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/consensus/snowman#Block
#[derive(Serialize, Clone, Deserialize)]
pub struct Block {
    #[serde(deserialize_with = "must_deserialize_id")]
    pub parent: Id,
    pub status: Status,
    height: u64,
    #[serde(with = "rfc3339::serde_format")]
    timestamp: DateTime<Utc>,
    data: Vec<u8>,

    // generated not serialized
    #[serde(skip)]
    id: Id,
    // generated not serialized
    #[serde(skip)]
    bytes: Vec<u8>,
    #[serde(skip)]
    vm: Option<ChainVm>,
}

#[tonic::async_trait]
impl rpcchainvm::concensus::snowman::Decidable for Block {
    /// id returns the ID of this block
    async fn id(&self) -> Id {
        self.id
    }

    /// status returns the status of this block
    async fn status(&self) -> Status {
        self.status.clone()
    }

    /// Accepts this element.
    async fn accept(&mut self) -> Result<()> {
        let vm = self.vm.clone();
        let vm = vm.ok_or(Error::new(ErrorKind::Other, "no vm associated with block"))?;
        let mut inner = vm.inner.write().await;

        self.status = Status::Accepted;

        // add newly accepted block to state
        inner
            .state
            .put_block(self.clone(), vm.clone())
            .await
            .map_err(|e| Error::new(ErrorKind::Other, format!("failed to put block: {:?}", e)))?;

        // set last accepted block to this block id
        inner
            .state
            .set_last_accepted_block_id(&self.id)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, format!("failed to put block: {:?}", e)))?;

        // remove from verified blocks
        inner.verified_blocks.remove(&self.id);
        Ok(())
    }

    /// Rejects this element.
    async fn reject(&mut self) -> Result<()> {
        let vm = self.vm.clone();
        let vm = vm.ok_or(Error::new(ErrorKind::Other, "no vm associated with block"))?;
        let mut inner = vm.inner.write().await;

        self.status = Status::Rejected;

        // add newly rejected block to state
        inner
            .state
            .put_block(self.clone(), vm.clone())
            .await
            .map_err(|e| Error::new(ErrorKind::Other, format!("failed to put block: {:?}", e)))?;

        // remove from verified, as it is rejected
        inner.verified_blocks.remove(&self.id);
        Ok(())
    }
}

#[tonic::async_trait]
impl rpcchainvm::concensus::snowman::Block for Block {
    /// bytes returns the binary representation of this block
    async fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// height returns this block's height. The genesis block has height 0.
    async fn height(&self) -> u64 {
        self.height
    }

    async fn timestamp(&self) -> u64 {
        self.timestamp.timestamp() as u64
    }

    async fn parent(&self) -> Id {
        self.parent
    }

    /// verify ensures that the state of the block is expected.
    async fn verify(&self) -> Result<()> {
        let vm = self
            .vm
            .clone()
            .ok_or(Error::new(ErrorKind::Other, "no reference to vm"))?;

        let vm = vm.inner.read().await;

        match vm.state.get_block(self.parent).await? {
            Some(parent_block) => {
                // Ensure block height comes right after its parent's height
                if parent_block.height().await + 1 != self.height {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "failed to verify block invalid height",
                    ));
                }
                // Ensure block timestamp is after its parent's timestamp.
                if self.timestamp().await.cmp(&parent_block.timestamp().await) == Ordering::Less {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!(
                            "block timestamp: {} is after parents: {}",
                            self.timestamp().await,
                            parent_block.timestamp().await
                        ),
                    ));
                }
                Ok(())
            }
            None => Err(Error::new(
                ErrorKind::NotFound,
                "failed to verify block parent not found",
            )),
        }
    }
}

impl MiniKvvmBlock for Block {
    /// data returns the block payload.
    fn data(&self) -> &[u8] {
        &self.data
    }

    fn set_status(&mut self, status: Status) {
        self.status = status;
    }

    /// initialize populates the generated fields (id, bytes) of the the block and
    /// returns the generated id.
    fn initialize(&mut self, vm: ChainVm) -> Result<Id> {
        if self.id.is_empty() {
            match serde_json::to_vec(&self) {
                // Populate generated fields
                Ok(block_bytes) => {
                    let block_data = block_bytes.as_slice();
                    let block_id = to_block_id(&block_data);
                    self.id = block_id;
                    self.bytes = block_bytes;
                    self.vm = Some(vm);
                    return Ok(self.id);
                }
                Err(error) => {
                    return Err(Error::new(ErrorKind::NotFound, error));
                }
            }
        }
        Ok(self.id)
    }
}

fn to_block_id(bytes: &[u8]) -> Id {
    new_id(Hash::hash(bytes))
}

fn new_id(bytes: [u8; DATA_LEN]) -> Id {
    Id::from_slice(&bytes)
}

#[tokio::test]
async fn test_serialization_round_trip() {
    use rpcchainvm::concensus::snowman::Block as _; //Bring the block trait into scope for [.parent()]
    let block = Block::default();
    let writer = serde_json::to_vec(&block).unwrap();
    let value: Block = serde_json::from_slice(&writer).unwrap();
    assert_eq!(block.parent().await, value.parent().await);
}
