use jsonrpc_derive::rpc;
use jsonrpc_http_server::jsonrpc_core::{BoxFuture, IoHandler, Result};
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::engine::*;

/// IssueTxArgs are the arguments for IssueTx.
#[derive(Serialize, Deserialize)]
pub struct IssueTxArgs {
    /// key is a hex string
    key: String,
    /// value is a hex string
    value: String,
}

/// IssueTxReply is the reply from IssueTx.
pub struct IssueTxReply {
    success: bool,
}

// GetBlockArgs are the arguments to GetBlock
pub struct GetBlockArgs {
    // id of the block we're getting.
    // If left blank, gets the latest block
    id: String,
}

/// GetBlockReply is the reply from GetBlock
pub struct GetBlockReply {
    api_block: APIBlock,
    key_values: Vec<KeyValue>,
}

/// KeyValue is the GetBlock return type
#[derive(Serialize, Deserialize)]
pub struct KeyValue {
    key: String,
    value: String,
}

/// APIBlock is the API representation of a block
#[derive(Serialize, Deserialize)]
pub struct APIBlock {
    /// Timestamp json.Uint64 `json:"timestamp"` // Timestamp of most recent block
    timestamp: AtomicU64,
    /// Data in the most recent block. Base 58 repr. of 5 bytes.
    data: String,
    /// String repr. of ID of the most recent block
    id: String,
    // String repr. of ID of the most recent block's parent
    parent_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct GetValueReply {
    value: String,
}

#[rpc(server)]
pub trait ServiceApi {
    #[rpc(name = "issueTx", alias("kvvm.issueTx"))]
    fn issue_tx(&self) -> BoxFuture<Result<IssueTxReply>>;

    #[rpc(name = "getBlock", alias("kvvm.getBlock"))]
    fn get_block(&self, args: GetBlockArgs) -> BoxFuture<Result<GetBlockReply>>;

    #[rpc(name = "getValue", alias("kvvm.getValue"))]
    fn get_value(&self) -> BoxFuture<Result<GetValueReply>>;
}

pub struct ServiceApiImpl {
    vm: Arc<RwLock<dyn ChainVM>>,
}

pub fn new(vm: Arc<RwLock<dyn ChainVM>>) -> IoHandler {
    let mut io_handler = IoHandler::new();
    let handlers = ServiceApiImpl { vm };
    io_handler.extend_with(handlers.to_delegate());
    io_handler
}

impl ServiceApi for ServiceApiImpl {
    /// ProposeBlock is an API method to propose a new block whose data is [args].Data.
    /// [args].Data must be a string repr. of a 32 byte array
    /// [WARN] issue an invalid tx is a no-op, will not be included in block, but still show as a success for this method
    /// TODO parse the string to different transaction object
    fn issue_tx(&self) -> BoxFuture<Result<IssueTxReply>> {
        Box::pin(async move {
            log::info!("IssueTx");
            Ok(IssueTxReply { success: true })
        })
    }

    /// GetBlock gets the block whose ID is [args.ID]
    /// If [args.ID] is empty, get the latest block
    fn get_block(&self, args: GetBlockArgs) -> BoxFuture<Result<GetBlockReply>> {
        Box::pin(async move {
            log::info!("GetBlock");
            Ok(GetBlockReply {})
        })
    }

    /// GetValue gets the value of the key based on arg
    fn get_value(&self) -> BoxFuture<Result<GetValueReply>> {
        Box::pin(async move {
            log::info!("GetValue");
            Ok(GetValueReply {})
        })
    }
}
