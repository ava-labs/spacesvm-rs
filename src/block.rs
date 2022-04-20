use avalanche_types::ids;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicU64;
use utils::rfc3339;

pub const DATA_LEN: usize = 32;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    parent_id: ids::Id,
    height: u64,
    #[serde(with = "rfc3339::serde_format")]
    timestamp: DateTime<Utc>,

    id: ids::Id,
    bytes: [u8; DATA_LEN],
}

/// snow/consensus/snowman/Block
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/consensus/snowman#Block
impl Block {
    // TODO: add
    // ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/choices#Decidable

    pub fn parent(&self) -> &ids::Id {
        &self.parent_id
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

    pub fn timestamp(&self) -> &DateTime<Utc> {
        &self.timestamp
    }
}
