use std::{
    fmt,
    fs::{self, File},
    io::{self, Error, ErrorKind, Write},
    path::Path,
};

use avalanche_types::rpcchainvm::database::manager::versioned_database::VersionedDatabase;
use log::info;
use serde::{Deserialize, Serialize};
use ethereum_types::Address;

pub const MIN_BLOCK_COST: usize  = 0;
pub const DEFAULT_LOOKBACK_WINDOW: i64 = 60; // Seconds
pub const DEFAULT_VALUE_UNIT_SIZE: u64 = 1 * 1024; //Kib

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct Genesis {
    pub magic: u64,

   	/// Tx params
    pub base_tx_units: u64,

    /// SetTx params
    pub value_unit_size: u64,
    pub max_value_sized: u64,

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
         max_value_sized: 200 * 1024, // 200 Kib
     
         /// Fee Mechanism Params
         min_price: 1,
         lookback_window: DEFAULT_LOOKBACK_WINDOW, // ^0 Seconds
         target_block_rate: 1, // 1 Block per Second
         target_block_size: 225, // ~225 Kib
         max_block_size: 246, // 246 Kib
         block_cost_enabled: true,
     
         /// Allocations
         custom_allocations: None,
         airdrop_hash: "".to_string(),
         airdrop_units: 0,
        }
    }

    pub fn from_json<S>(d: S) -> io::Result<Self>
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
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("invalid magic"),
            ));
        }
        if self.target_block_rate == 0 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("invalid block rate"),
            ));
        }
        Ok(())
    }

    pub fn load(db: Box<dyn avalanche_types::rpcchainvm::database::Database + Send + Sync>, air_drop: &[u8]) -> std::io::Result<()> {
        let versioned_database = VersionedDatabase::new(db, version)

        // TODO: add airdrop support.

        //TODO: blocked need support for versiondb
    }

    pub fn sync(&self, file_path: &str) -> io::Result<()> {
        info!("syncing genesis to '{}'", file_path);
        let path = Path::new(file_path);
        let parent_dir = path.parent().unwrap();
        fs::create_dir_all(parent_dir)?;

        let ret = serde_json::to_vec(&self);
        let d = match ret {
            Ok(d) => d,
            Err(e) => {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("failed to serialize genesis info to YAML {}", e),
                ));
            }
        };
        let mut f = File::create(&file_path)?;
        f.write_all(&d)?;

        Ok(())
    }
}

impl fmt::Display for Genesis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = serde_yaml::to_string(&self).unwrap();
        write!(f, "{}", s)
    }
}
