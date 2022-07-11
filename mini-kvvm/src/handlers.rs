//Implementation of publicservicevm

use avalanche_types::vm::{state::State as VmState, engine::common::HttpHandler};

use jsonrpc_core::{Error as JsonRPCError, ErrorCode as JRPCErrorCode, Result, IoHandler, BoxFuture};
use jsonrpc_derive::rpc;
use std::{sync::Arc, collections::HashMap};
use tokio::sync::RwLock;

use super::engine::{ChainVm, Getter, Parser, Vm}; //need to import these so chainvm can call methods
use super::publicservicevm::*;
use super::kvvm::ChainVmInterior;

/// Defines the kinds of methods the kvvm api can handle
#[rpc(server)]
pub trait HandlersService {
    
    #[rpc(name = "build_block")]
    fn build_block(&self) -> BoxFuture<Result<BuildBlockResponse>>;

    #[rpc(name = "get_block")]
    fn get_block(&self, params: GetBlockArgs) -> BoxFuture<Result<GetBlockResponse>>;

    #[rpc(name = "last_accepted")]
    fn last_accepted(&self) -> BoxFuture<Result<LastAcceptedResponse>>;

    #[rpc(name = "parse_block")]
    fn parse_block(&self, params: ParseBlockArgs) -> BoxFuture<Result<ParseBlockResponse>>;

    #[rpc(name = "set_state")]
    fn set_state(&self, params: SetStateArgs) -> BoxFuture<Result<()>>;

    #[rpc(name = "set_preference")]
    fn set_preference(&self, params: SetPreferenceArgs) -> BoxFuture<Result<()>>;
}

/// Implementation of handlers
pub struct HandlersServiceImpl {
    vm: Arc<RwLock<ChainVmInterior>>
}

// TODO: Edit to pass error messages through jsonrpc error
fn create_jsonrpc_error(_: std::io::Error) -> JsonRPCError{
    JsonRPCError::new(JRPCErrorCode::InternalError)
}

impl HandlersService for HandlersServiceImpl {

    fn build_block(&self) -> BoxFuture<Result<BuildBlockResponse>> {
        println!("build block called");
        let vm = self.vm.clone();
        Box::pin(async move {
            let result = ChainVmInterior::build_block(&vm)
                .await
                .map_err(create_jsonrpc_error)?;

            Ok(BuildBlockResponse{ 
                block: result
            })
        })
    }

    fn get_block(&self, params: GetBlockArgs) -> BoxFuture<Result<GetBlockResponse>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let result = ChainVmInterior::get_block(&vm, params.id)
                .await
                .map_err(create_jsonrpc_error)?;

            Ok(GetBlockResponse{ 
                block: result
            })
        })
    }

    fn last_accepted(&self) -> BoxFuture<Result<LastAcceptedResponse>> {
        println!("last accepted called");
        let vm = self.vm.clone();
        Box::pin(async move {
            let result = ChainVmInterior::last_accepted(&vm)
                .await
                .map_err(create_jsonrpc_error)?;

            Ok(LastAcceptedResponse{ 
                id: result
            })
        })
    }

    fn parse_block(&self, params: ParseBlockArgs) -> BoxFuture<Result<ParseBlockResponse>> {
        println!("parse block called");
        let vm = self.vm.clone();
        Box::pin(async move {
            let result = ChainVmInterior::parse_block(&vm, params.bytes.as_ref())
                .await
                .map_err(create_jsonrpc_error)?;

            Ok(ParseBlockResponse{ 
                block: result
            })
        })
    }

    fn set_state(&self, params: SetStateArgs) -> BoxFuture<Result<()>> {
        println!("set state called");
        let vm = self.vm.clone();
        Box::pin(async move {
            let vmstate = VmState::try_from(params.state)
            .map_err(|_| {
                JsonRPCError::internal_error()
            })?;

            let _ = ChainVmInterior::set_state(&vm, vmstate)
                .await
                .map_err(create_jsonrpc_error)?;

            Ok(())
        })
    }

    fn set_preference(&self, params: SetPreferenceArgs) -> BoxFuture<Result<()>> {
        println!("set preference called");
        let vm = self.vm.clone();
        Box::pin(async move {
            let _ = ChainVmInterior::set_preference(&vm, params.id)
                .await
                .map_err(create_jsonrpc_error)?;

            Ok(())
        })
    }
}
