use avalanche_types::ids::Id;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct Activity {
    timestamp: u64,
    pub tx_id: Id,
    typ: String,
    pub sender: String,
    key: String,
    to: String, // common.Address will be 0x000 when not populated
    units: u64,
}
