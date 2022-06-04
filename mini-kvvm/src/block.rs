use std::{
    cmp::Ordering,
    io::{Error, ErrorKind, Result},
    sync::Arc,
};

use avalanche_types::{
    choices::status::Status,
    ids::{must_deserialize_id, Id},
};
use avalanche_utils::rfc3339;
use bytes::BufMut;
use chrono::{DateTime, NaiveDateTime, Utc};
use hmac_sha256::Hash;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::kvvm::ChainVMInterior;

pub const DATA_LEN: usize = 32;

// TODO remove
// Default is only used as a placeholder for unimplemented block logic
impl Default for Block {
    fn default() -> Self {
        Self {
            id: Some(Id::empty()),
            parent: Id::empty(),
            timestamp: DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc),
            bytes: Vec::default(),
            height: 0,
            status: Status::Unknown("".to_string()),
            data: Vec::default(),
        }
    }
}

/// snow/consensus/snowman/Block
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/consensus/snowman#Block
#[derive(Serialize, Debug, Clone, Deserialize)]
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
    id: Option<Id>,
    // generated not serialized
    #[serde(skip)]
    bytes: Vec<u8>,
}

impl Block {
    pub fn new(
        parent: Id,
        height: u64,
        data: Vec<u8>,
        timestamp: DateTime<Utc>,
        status: Status,
    ) -> Result<Self> {
        Ok(Self {
            parent,
            height,
            timestamp,
            data,
            status,
            id: None,
            bytes: Vec::default(),
        })
    }

    pub fn parent(&self) -> Id {
        self.parent
    }

    /// id returns the ID of this block
    pub fn id(&self) -> Option<Id> {
        self.id
    }

    pub fn timestamp(&self) -> &DateTime<Utc> {
        &self.timestamp
    }

    /// verify ensures that the state of the block is expected.
    pub async fn verify(&self, inner: &Arc<RwLock<ChainVMInterior>>) -> Result<()> {
        let mut vm = inner.write().await;

        match vm.state.get_block(self.parent).await? {
            Some(parent_block) => {
                // Ensure block height comes right after its parent's height
                if parent_block.height() + 1 != self.height() {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "failed to verify block invalid height",
                    ));
                }
                // Ensure block timestamp is after its parent's timestamp.
                if self.timestamp().cmp(parent_block.timestamp()) == Ordering::Less {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!(
                            "block timestamp: {} is after parents: {}",
                            self.timestamp(),
                            parent_block.timestamp()
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

    /// data returns the block payload.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// bytes returns the binary representation of this block
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// height returns this block's height. The genesis block has height 0.
    pub fn height(&self) -> u64 {
        self.height
    }

    /// status returns the status of this block
    pub fn status(&self) -> Status {
        self.status.clone()
    }

    /// initialize populates the generated fields (id, bytes) of the the block and
    /// returns the generated id.
    pub fn initialize(&mut self) -> Result<Id> {
        if self.id.is_none() {
            let mut writer = Vec::new().writer();
            serde_json::to_writer(&mut writer, &self.parent())?;
            serde_json::to_writer(&mut writer, &self.height())?;
            serde_json::to_writer(&mut writer, &self.timestamp().to_string())?;
            serde_json::to_writer(&mut writer, &self.data())?;

            let block_bytes = serde_json::to_vec(&self).unwrap();

            let block_data = writer.into_inner();
            let block_id = Self::generate(&block_data);
            self.id = Some(block_id);
            self.bytes = block_bytes;
        }

        Ok(self.id.expect("this is some"))
    }

    pub fn new_id(bytes: [u8; DATA_LEN]) -> Id {
        Id::from_slice(&bytes)
    }

    pub fn generate(bytes: &[u8]) -> Id {
        Self::new_id(Hash::hash(bytes))
    }
}

#[test]
fn test_serialization_round_trip() {
    let block = Block::default();
    let writer = serde_json::to_vec(&block).unwrap();
    let value: Block = serde_json::from_slice(&writer).unwrap();
    assert_eq!(block.parent(), value.parent());
}
