use std::{
    fmt,
    io::{Error, ErrorKind, Result, Write},
    time::Instant,
};

use crate::chain::storage::set_balance;
use ethereum_types::Address;
use log::debug;
use serde::{Deserialize, Serialize};

pub const MIN_BLOCK_COST: usize = 0;
pub const DEFAULT_LOOKBACK_WINDOW: i64 = 60; // Seconds
pub const DEFAULT_VALUE_UNIT_SIZE: u64 = 1 * 1024; //Kib

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct Genesis {
    pub magic: u64,

    /// Tx params
    pub base_tx_units: u64,

    /// SetTx params
    pub value_unit_size: u64,
    pub max_value_size: u64,

    /// Fee Mechanism Params
    pub min_price: u64,
    pub lookback_window: i64,
    pub target_block_rate: i64,
    pub target_block_size: u64,
    pub max_block_size: u64,
    pub block_cost_enabled: bool,

    /// Allocations
    pub custom_allocations: Option<Vec<CustomAllocation>>,
    pub airdrop_hash: String,
    pub airdrop_units: u64,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
struct CustomAllocation {
    pub address: Address,
    pub balance: u64,
}

impl Default for Genesis {
    fn default() -> Self {
        Self::default()
    }
}

impl Genesis {
    pub fn default() -> Self {
        Self {
            magic: 0,

            /// Tx params
            base_tx_units: 1,

            /// SetTx params
            value_unit_size: DEFAULT_VALUE_UNIT_SIZE,
            max_value_size: 200 * 1024, // 200 Kib

            /// Fee Mechanism Params
            min_price: 1,
            lookback_window: DEFAULT_LOOKBACK_WINDOW, // ^0 Seconds
            target_block_rate: 1,                     // 1 Block per Second
            target_block_size: 225,                   // ~225 Kib
            max_block_size: 246,                      // 246 Kib
            block_cost_enabled: true,

            /// Allocations
            custom_allocations: None,
            airdrop_hash: "".to_string(),
            airdrop_units: 0,
        }
    }

    pub fn from_json<S>(d: S) -> Result<Self>
    where
        S: AsRef<[u8]>,
    {
        let resp: Self = match serde_json::from_slice(d.as_ref()) {
            Ok(p) => p,
            Err(e) => {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("failed to decode {}", e),
                ));
            }
        };
        Ok(resp)
    }

    pub fn verify(&self) -> std::io::Result<()> {
        if self.magic == 0 {
            return Err(Error::new(ErrorKind::InvalidData, format!("invalid magic")));
        }
        if self.target_block_rate == 0 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("invalid block rate"),
            ));
        }
        Ok(())
    }

    pub async fn load(
        &self,
        db: Box<dyn avalanche_types::rpcchainvm::database::Database + Send + Sync>,
        air_drop: &[u8],
    ) -> std::io::Result<()> {
        let start = Instant::now();
        defer!(debug!("loaded genesis allocations: {:?}", start.elapsed()));

        // TODO: add airdrop support.
        // TODO: add support for versiondb

        // Do custom allocation last in case an address shows up in standard
        // allocation
        for alloc in self.custom_allocations.iter() {
            set_balance(db, alloc.address, self.airdrop_units)
                .await
                .map_err(|e| {
                    Error::new(ErrorKind::Other, format!("failed to set balance: {:?}", e))
                })?;
        }

        Ok(())
    }
}

impl fmt::Display for Genesis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = serde_yaml::to_string(&self).unwrap();
        write!(f, "{}", s)
    }
}
