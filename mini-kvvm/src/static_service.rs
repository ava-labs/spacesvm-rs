use crate::kvvm::ChainVm;

use avalanche_types::rpcchainvm::common::vm::Vm;
use jsonrpc_core::{BoxFuture, Error as JsonRPCError, ErrorCode as JRPCErrorCode, Result};
use jsonrpc_derive::rpc;
use serde::Deserialize;

pub const STATICSERVICE_PUBLICENDPOINT: &str = "/static-kvvm-rs"; //used for this service's endpoint

#[derive(Deserialize)]
pub struct SetStateArgs {
    pub id: u32,
}

#[rpc(server)]
pub trait StaticService {
    #[rpc(name = "set_state")]
    fn set_state(&self, params: SetStateArgs) -> BoxFuture<Result<()>>;
}
/// Implementation of handlers
pub struct StaticServiceImpl {
    pub vm: ChainVm,
}

// TODO: Edit to pass error messages through jsonrpc error
fn create_jsonrpc_error(_: std::io::Error) -> JsonRPCError {
    JsonRPCError::new(JRPCErrorCode::InternalError)
}

impl StaticService for StaticServiceImpl {
    fn set_state(&self, params: SetStateArgs) -> BoxFuture<Result<()>> {
        log::info!("set state method called");
        let vm = self.vm.clone();

        Box::pin(async move {
            let state = avalanche_types::rpcchainvm::state::State::try_from(params.id)
                .map_err(|_| JsonRPCError::new(JRPCErrorCode::InternalError))?;

            vm.set_state(state).await.map_err(create_jsonrpc_error)?;

            Ok(())
        })
    }
}
