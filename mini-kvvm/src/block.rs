use std::io::{Error, ErrorKind};

use avalanche_types::ids::Id;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};
use utils::rfc3339;

pub const DATA_LEN: usize = 32;

impl Default for Block {
    fn default() -> Self {
        let now = chrono::offset::Utc::now();
        Self {
            id: Some(Id::default()),
            parent_id: Id::default(),
            timestamp: now,
            bytes: [0; DATA_LEN],
            height: 0,
            status: Status::Unknown,
        }
    }
}

#[derive(Serialize, Debug, Copy, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub parent_id: Id,
    height: u64,
    #[serde(with = "rfc3339::serde_format")]
    timestamp: DateTime<Utc>,
    id: Option<Id>,
    bytes: [u8; DATA_LEN],
    status: Status,
}

/// snow/consensus/snowman/Block
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/consensus/snowman#Block
impl Block {
    // TODO: add
    // ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/choices#Decidable

    pub fn new(
        parent_id: Id,
        height: u64,
        bytes: [u8; DATA_LEN],
        timestamp: DateTime<Utc>,
        status: Status,
    ) -> Result<Self, Error> {
        Ok(Self {
            parent_id,
            height,
            timestamp: timestamp,
            bytes,
            id: None,
            status,
        })
    }

    pub fn parent(&self) -> Id {
        self.parent_id
    }

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

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn height(&self) -> u64 {
        self.height
    }

    pub fn status(&self) -> Status {
        self.status
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
