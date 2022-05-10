use avalanche_types::ids;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utils::rfc3339;

pub const DATA_LEN: usize = 32;

impl Default for Block {
    fn default() -> Self {
        let now = chrono::offset::Utc::now();
        Self {
            id: ids::Id::default(),
            parent_id: ids::Id::default(),
            timestamp: now,
            bytes: [0; DATA_LEN],
            height: 0,
            status: Status::Unknown,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub parent_id: ids::Id,
    height: u64,
    #[serde(with = "rfc3339::serde_format")]
    timestamp: DateTime<Utc>,
    id: ids::Id,
    bytes: [u8; DATA_LEN],
    status: Status,
}

/// snow/consensus/snowman/Block
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/consensus/snowman#Block
impl Block {
    // TODO: add
    // ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/choices#Decidable

    pub fn parent(&self) -> &ids::Id {
        &self.parent_id
    }

    pub fn id(&self) -> &ids::Id {
        &self.id
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
