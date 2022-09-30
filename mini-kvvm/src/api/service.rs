use crate::{
    api::*,
    chain::{self, tx::Transaction, vm::Vm},
    vm,
};

use avalanche_types::{ids, rpcchainvm::snowman::block::ChainVm};

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
        log::debug!("ping called");

        Box::pin(async move { Ok(PingResponse { success: true }) })
    }

    /// Takes a raw tx as a byte slice and returns the tx id.
    fn issue_raw_tx(&self, _params: IssueRawTxArgs) -> BoxFuture<Result<IssueRawTxResponse>> {
        log::debug!("issue raw tx method called");
        let _vm = self.vm.clone();

        Box::pin(async move {
            Ok(IssueRawTxResponse {
                tx_id: ids::Id::empty(),
            })
        })
    }

    /// Takes tx args and returns the tx id.
    fn issue_tx(&self, params: IssueTxArgs) -> BoxFuture<Result<IssueTxResponse>> {
        log::debug!("issue tx called");
        let vm = self.vm.clone();

        Box::pin(async move {
            let unsigned_tx = params
                .typed_data
                .parse_typed_data()
                .map_err(create_jsonrpc_error)?;

            let mut tx = chain::tx::tx::Transaction::new(unsigned_tx, params.signature);
            tx.init().await.map_err(create_jsonrpc_error)?;
            let tx_id = tx.id().await;

            let mut txs = Vec::with_capacity(1);
            txs.push(tx);
            vm.submit(txs).await.map_err(create_jsonrpc_error)?;
            Ok(IssueTxResponse { tx_id })
        })
    }

    fn decode_tx(&self, params: DecodeTxArgs) -> BoxFuture<Result<DecodeTxResponse>> {
        log::debug!("decode input called");
        let vm = self.vm.clone();

        Box::pin(async move {
            let mut utx = params.tx_data.decode().map_err(create_jsonrpc_error)?;
            let inner = vm.inner.read().await;
            let last_accepted = &inner.last_accepted;
            utx.set_block_id(last_accepted.id).await;
            let typed_data = utx.typed_data().await;
            Ok(DecodeTxResponse { typed_data })
        })
    }

    fn resolve(&self, params: ResolveArgs) -> BoxFuture<Result<ResolveResponse>> {
        log::debug!("resolve called");
        let vm = self.vm.clone();

        Box::pin(async move {
            let value = chain::storage::get_value(&vm.db, &params.bucket, &params.key)
                .await
                .map_err(create_jsonrpc_error)?;
            if value.is_none() {
                return Ok(ResolveResponse::default());
            }

            let meta = chain::storage::get_value_meta(&vm.db, &params.bucket, &params.key)
                .await
                .map_err(create_jsonrpc_error)?;
            if meta.is_none() {
                return Ok(ResolveResponse::default());
            }

            Ok(ResolveResponse {
                exists: true,
                value: value.unwrap(),
                meta: meta.unwrap(),
            })
        })
    }

    /// Calls build_block on the vm level.
    fn build_block(&self, _params: BuildBlockArgs) -> BoxFuture<Result<BuildBlockResponse>> {
        log::debug!("build block method called");
        let vm = self.vm.clone();

        Box::pin(async move {
            let block = vm.build_block().await.map_err(create_jsonrpc_error)?;
            let bytes = block.bytes().await;
            Ok(BuildBlockResponse {
                block: bytes.to_vec(),
            })
        })
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
