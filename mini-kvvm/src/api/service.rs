use crate::{api::*, vm};

use avalanche_types::ids;

pub struct Service {
    pub vm: vm::ChainVm,
}

impl Service {
    pub fn new(vm: vm::ChainVm) -> Self {
        Self { vm }
    }
}

impl crate::api::Service for Service {
    /// Returns true if the API is serving requests.
    fn ping(&self) -> BoxFuture<Result<PingResponse>> {
        log::debug!("last accepted method called");
        let _vm = self.vm.clone();

        Box::pin(async move { Ok(PingResponse { success: true }) })
    }

    /// Takes a raw tx as a byte slice and returns the tx id.
    fn issue_raw_tx(&self, _params: IssueRawTxArgs) -> BoxFuture<Result<IssueRawTxResponse>> {
        log::debug!("build block method called");
        let _vm = self.vm.clone();

        Box::pin(async move {
            Ok(IssueRawTxResponse {
                tx_id: ids::Id::empty(),
            })
        })
    }

    /// Calls build_block on the vm level.
    fn build_block(&self, _params: BuildBlockArgs) -> BoxFuture<Result<BuildBlockResponse>> {
        log::debug!("build block method called");
        let _vm = self.vm.clone();

        Box::pin(async move { Ok(BuildBlockResponse { block: vec![] }) })
    }

    /// Takes raw bytes of block and put it into the local cache and persists to database.
    fn put_block(&self, _params: PutBlockArgs) -> BoxFuture<Result<PutBlockResponse>> {
        log::debug!("put block method called");
        let _vm = self.vm.clone();

        Box::pin(async move {
            Ok(PutBlockResponse {
                id: ids::Id::empty(),
            })
        })
    }

    /// Returns a serialized blocks given its Id.
    fn get_block(&self, _params: GetBlockArgs) -> BoxFuture<Result<GetBlockResponse>> {
        log::debug!("get block method called");
        let _vm = self.vm.clone();

        Box::pin(async move { Ok(GetBlockResponse { block: vec![] }) })
    }

    /// Returns the Id of the last accepted block.
    fn last_accepted(&self) -> BoxFuture<Result<LastAcceptedResponse>> {
        log::debug!("last accepted method called");
        let _vm = self.vm.clone();

        Box::pin(async move {
            Ok(LastAcceptedResponse {
                id: ids::Id::empty(),
            })
        })
    }

    /// Given the block bytes return serialized block.
    fn parse_block(&self, _params: ParseBlockArgs) -> BoxFuture<Result<ParseBlockResponse>> {
        log::debug!("parse block method called");
        let _vm = self.vm.clone();

        Box::pin(async move { Ok(ParseBlockResponse { block: vec![] }) })
    }
}
