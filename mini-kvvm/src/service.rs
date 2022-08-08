use crate::{block::Block, kvvm::ChainVm};

use avalanche_types::{
    choices::status::Status,
    ids::Id,
    rpcchainvm::snowman::block::{ChainVm as ChainVmTrait, Getter, Parser},
};
use chrono::{DateTime, NaiveDateTime, Utc};
use jsonrpc_core::{BoxFuture, Error as JsonRPCError, ErrorCode as JRPCErrorCode, Result};
use jsonrpc_derive::rpc;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub const SERVICE_PUBLICENDPOINT: &str = "/kvvm-rs"; //used for this service's endpoint

#[derive(Serialize)]
pub struct BuildBlockResponse {
    pub block: Vec<u8>,
}

#[derive(Deserialize)]
pub struct GetBlockArgs {
    pub id: String,
}

#[derive(Serialize)]
pub struct GetBlockResponse {
    pub block: Vec<u8>,
}

#[derive(Serialize)]
pub struct LastAcceptedResponse {
    pub id: Id,
}

#[derive(Deserialize)]
pub struct ParseBlockArgs {
    pub bytes: Vec<u8>,
}

#[derive(Serialize)]
pub struct ParseBlockResponse {
    pub block: Vec<u8>,
}

#[derive(Deserialize)]
pub struct AddBlockArgs {
    pub bytes: Vec<u8>,
}

#[derive(Serialize)]
pub struct AddBlockResponse {
    pub id: Id,
}

#[rpc(server)]
pub trait Service {
    #[rpc(name = "build_block")]
    fn build_block(&self) -> BoxFuture<Result<BuildBlockResponse>>;

    #[rpc(name = "get_block")]
    fn get_block(&self, params: GetBlockArgs) -> BoxFuture<Result<GetBlockResponse>>;

    #[rpc(name = "last_accepted")]
    fn last_accepted(&self) -> BoxFuture<Result<LastAcceptedResponse>>;

    #[rpc(name = "parse_block")]
    fn parse_block(&self, params: ParseBlockArgs) -> BoxFuture<Result<ParseBlockResponse>>;

    #[rpc(name = "add_block")]
    fn add_block(&self, params: AddBlockArgs) -> BoxFuture<Result<AddBlockResponse>>;
}
/// Implementation of handlers
pub struct ServiceImpl {
    pub vm: ChainVm,
}

// TODO: Edit to pass error messages through jsonrpc error
fn create_jsonrpc_error(e: std::io::Error) -> JsonRPCError {
    let mut error = JsonRPCError::new(JRPCErrorCode::InternalError);
    error.message = format!("{}", e);
    error
}

impl Service for ServiceImpl {
    fn build_block(&self) -> BoxFuture<Result<BuildBlockResponse>> {
        log::info!("build block method called");
        let vm = self.vm.clone();

        Box::pin(async move {
            let result = vm.build_block().await.map_err(create_jsonrpc_error)?;
            let bytes = result.bytes().await.to_vec();
            Ok(BuildBlockResponse { block: bytes })
        })
    }

    fn add_block(&self, params: AddBlockArgs) -> BoxFuture<Result<AddBlockResponse>> {
        use crate::block::MiniKvvmBlock;
        log::info!("add block method called");
        let vm = self.vm.clone();

        let mut block = Block::new(
            Id::empty(),
            0,
            params.bytes,
            DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc),
            Status::Processing,
        );

        Box::pin(async move {
            let block_id = block.initialize(vm.clone()).map_err(create_jsonrpc_error)?;
            let mut inner = vm.inner.write().await;
            let accepted_block_id = inner
                .state
                .accept_block(block, vm.clone())
                .await
                .map_err(create_jsonrpc_error)?;
            inner.verified_blocks.remove(&accepted_block_id);
            Ok(AddBlockResponse { id: block_id })
        })
    }

    fn get_block(&self, params: GetBlockArgs) -> BoxFuture<Result<GetBlockResponse>> {
        log::info!("get block method called");
        let vm = self.vm.clone();

        Box::pin(async move {
            let id = Id::from_str(params.id.as_str()).map_err(create_jsonrpc_error)?;
            let result = vm.get_block(id).await.map_err(create_jsonrpc_error)?;
            let bytes = result.bytes().await.to_vec();
            Ok(GetBlockResponse { block: bytes })
        })
    }

    fn last_accepted(&self) -> BoxFuture<Result<LastAcceptedResponse>> {
        log::info!("last accepted method called");
        let vm = self.vm.clone();

        Box::pin(async move {
            let result = vm.last_accepted().await.map_err(create_jsonrpc_error)?;
            Ok(LastAcceptedResponse { id: result })
        })
    }

    fn parse_block(&self, params: ParseBlockArgs) -> BoxFuture<Result<ParseBlockResponse>> {
        log::info!("parse block method called");
        let vm = self.vm.clone();

        Box::pin(async move {
            let result = vm
                .parse_block(params.bytes.as_ref())
                .await
                .map_err(create_jsonrpc_error)?;
            let bytes = result.bytes().await.to_vec();
            Ok(ParseBlockResponse { block: bytes })
        })
    }
}
