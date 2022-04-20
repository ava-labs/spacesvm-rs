use avalanche_types::ids;
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicU64;

#[derive(Serialize, Deserialize)]
pub struct Block {
    parent_id: ids::Id,
    height: u64,
    timestamp: AtomicU64,

    id: ids::Id,
    bytes: [u8; 32],
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

    pub fn timestamp(&self) -> &AtomicU64 {
        &self.timestamp
    }
}
