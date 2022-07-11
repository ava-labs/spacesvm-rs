use std::sync::Arc;
use tokio::sync::RwLock;
use std::io::Result as IoResult;
use crate::publicservicevm::{Block, Id};
use avalanche_types::vm::state::State as VmState;

pub struct ChainVmInterior {
    pub placehold_id: String
}

impl ChainVmInterior {
    pub fn new() -> ChainVmInterior {
        ChainVmInterior { placehold_id: String::from("TK421")}
    }

    pub async fn build_block(inner: &Arc<RwLock<ChainVmInterior>>) -> IoResult<Block> {
        let vm = inner.read().await;
        Ok(Block {
            placehold: format!("Vm of id {} has built a block", vm.placehold_id)
        })
    }

    pub async fn get_block(inner: &Arc<RwLock<ChainVmInterior>>, block_id: Id) -> IoResult<Block> {
        let vm = inner.read().await;
        Ok(Block {
            placehold: format!("Vm of id {} has returned a block of id {}", vm.placehold_id, block_id.placehold)
        })
    }

    pub async fn last_accepted(inner: &Arc<RwLock<ChainVmInterior>>) -> IoResult<Id> {
        let vm = inner.read().await;
        Ok(Id {
            placehold: format!("Vm of id {} has found the last accepted block", vm.placehold_id)
        })
    }

    pub async fn parse_block(inner: &Arc<RwLock<ChainVmInterior>>, bytes: &[u8]) -> IoResult<Block> {
        let vm = inner.read().await;
        Ok(Block {
            placehold: format!("Vm of id {} has parsed a block with bytes {}...", vm.placehold_id, bytes[0])
        })
    }

    pub async fn set_state(inner: &Arc<RwLock<ChainVmInterior>>, _: VmState) -> IoResult<()> {
        let _ = inner.read().await;
        Ok(())
    }

    pub async fn set_preference(inner: &Arc<RwLock<ChainVmInterior>>, _: Id) -> IoResult<()> {
        let _ = inner.read().await;
        Ok(())
    }

}