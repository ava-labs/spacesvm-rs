use std::io::Error;

use avalanche_types::ids::Id;
use avalanche_utils::rfc3339;
use bytes::BufMut;
use chrono::{DateTime, Utc};
use hmac_sha256::Hash;
use serde::{Deserialize, Serialize};

pub const DATA_LEN: usize = 32;

impl Default for Block {
    fn default() -> Self {
        let now = chrono::offset::Utc::now();
        Self {
            id: Some(Id::default()),
            parent: Id::default(),
            timestamp: now,
            bytes: Vec::default(),
            height: 0,
            status: Status::Unknown,
        }
    }
}

/// snow/consensus/snowman/Block
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/consensus/snowman#Block
#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct Block {
    pub parent: Id,
    pub status: Status,
    height: u64,
    #[serde(with = "rfc3339::serde_format")]
    timestamp: DateTime<Utc>,
    bytes: Vec<u8>,
    // id is generated not serialized
    #[serde(skip)]
    id: Option<Id>,
}

impl Block {
    pub fn new(
        parent: Id,
        height: u64,
        bytes: Vec<u8>,
        timestamp: DateTime<Utc>,
        status: Status,
    ) -> Result<Self, Error> {
        Ok(Self {
            parent,
            height,
            timestamp: timestamp,
            bytes,
            id: None,
            status,
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

    // TODO:
    // pub fn verify(&self) -> Result<(), Error> {
    //     Err(Error {})
    // }

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
        self.status
    }

    pub fn init(&mut self) -> Result<&Id, Error> {
        if self.id.is_none() {
            let mut writer = Vec::new().writer();
            serde_json::to_writer(&mut writer, &self.parent())?;
            serde_json::to_writer(&mut writer, &self.height())?;
            serde_json::to_writer(&mut writer, &self.timestamp().to_string())?;
            serde_json::to_writer(&mut writer, &self.bytes())?;

            let buf = writer.into_inner();
            let block_id = Self::generate(&buf);
            self.id = Some(block_id);
        }

        Ok(self.id.as_ref().expect("in Block::id, the id was just set to Some(_) above and yet is still None. This is next to impossible."))
    }

    pub fn new_id(bytes: [u8; DATA_LEN]) -> Id {
        Id::from_slice(&bytes)
    }

    pub fn generate(bytes: &[u8]) -> Id {
        Self::new_id(Hash::hash(bytes))
    }
}

/// snow/consensus/snowman/Block
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/choices#Status
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Status {
    Unknown,
    Processing,
    Rejected,
    Accepted,
}

impl Status {
    pub fn fetched(&self) -> bool {
        match self {
            Self::Processing => true,
            _ => self.decided(),
        }
    }

    pub fn decided(&self) -> bool {
        matches!(self, Self::Rejected | Self::Accepted)
    }

    pub fn valid(&self) -> bool {
        !matches!(self, Self::Unknown)
    }
}
