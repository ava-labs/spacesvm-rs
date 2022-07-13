use avalanche_types::ids::Id;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct Activity {
    timestamp: u64,
    tx_id: Id,
    typ: String,
    sender: String,
    key: String,
    to: String, // common.Address will be 0x000 when not populated
    units: u64,
}
