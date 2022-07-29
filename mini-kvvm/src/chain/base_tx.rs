use std::io::{Error, ErrorKind, Result};

use avalanche_types::ids;
use serde::{Deserialize, Serialize};

use super::genesis::Genesis;

#[derive(Serialize, Deserialize)]
struct BaseTx {
    /// ID of a block in the [lookbackWindow].
    block_id: ids::Id,

    // Value defined in genesis to protect against replay attacks on
    // different VMs.
    magic: u64,

    // Value per unit to spend on this transaction.
    price: u64,
}

impl BaseTx {
    fn get_block_id(&self) -> ids::Id {
        self.block_id
    }

    fn set_block_id(&self, id: ids::Id) {
        self.block_id = id;
    }

    fn get_magic(&self) -> u64 {
        self.magic
    }

    fn set_magic(&self, magic: u64) {
        self.magic = magic;
    }

    fn get_price(&self) -> u64 {
        self.price
    }

    fn set_price(&self, price: u64) {
        self.price = price;
    }

    fn execute_base(&self, genesis: Genesis) -> Result<()> {
        if self.block_id == ids::Id::empty() {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("invalid block id"),
            ));
        }
        if self.magic != genesis.magic {
            return Err(Error::new(ErrorKind::InvalidData, format!("invalid magic")));
        }

        if self.price < genesis.min_price {
            return Err(Error::new(ErrorKind::InvalidData, format!("invalid price")));
        }

        Ok(())
    }

    fn fee_units(&self, genesis: Genesis) -> u64 {
        return genesis.base_tx_units;
    }

    fn load_units(&self, genesis: Genesis) -> u64 {
        return self.fee_units(genesis);
    }

    fn copy(&self) -> BaseTx {
        let block_id = ids::Id::from_slice(self.block_id.as_ref());
        BaseTx {
            block_id,
            magic: self.magic,
            price: self.price,
        }
    }
}
