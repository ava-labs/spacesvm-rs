pub mod service;

use avalanche_types::ids;
use jsonrpc_core::{BoxFuture, Result};
use jsonrpc_derive::rpc;
use serde::{Deserialize, Serialize};

#[rpc(server)]
pub trait Service {
    #[rpc(name = "ping")]
    fn ping(&self) -> BoxFuture<Result<PingResponse>>;

    #[rpc(name = "issue_raw_tx")]
    fn issue_raw_tx(&self, params: IssueRawTxArgs) -> BoxFuture<Result<IssueRawTxResponse>>;

    #[rpc(name = "build_block")]
    fn build_block(&self, params: BuildBlockArgs) -> BoxFuture<Result<BuildBlockResponse>>;

    #[rpc(name = "get_block")]
    fn get_block(&self, params: GetBlockArgs) -> BoxFuture<Result<GetBlockResponse>>;

    #[rpc(name = "last_accepted")]
    fn last_accepted(&self) -> BoxFuture<Result<LastAcceptedResponse>>;

    #[rpc(name = "parse_block")]
    fn parse_block(&self, params: ParseBlockArgs) -> BoxFuture<Result<ParseBlockResponse>>;

    #[rpc(name = "put_block")]
    fn put_block(&self, params: PutBlockArgs) -> BoxFuture<Result<PutBlockResponse>>;
}

#[derive(Serialize)]
pub struct PingResponse {
    pub success: bool,
}

#[derive(Deserialize)]
pub struct IssueRawTxArgs {
    pub tx: Vec<u8>,
}

#[derive(Serialize)]
pub struct IssueRawTxResponse {
    pub tx_id: ids::Id,
}

#[derive(Deserialize)]
pub struct BuildBlockArgs {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

#[derive(Serialize)]
pub struct BuildBlockResponse {
    pub block: Vec<u8>,
}

#[derive(Deserialize)]
pub struct GetBlockArgs {
    #[serde(deserialize_with = "ids::must_deserialize_id")]
    pub id: ids::Id,
}

#[derive(Serialize)]
pub struct GetBlockResponse {
    pub block: Vec<u8>,
}

#[derive(Serialize)]
pub struct LastAcceptedResponse {
    pub id: ids::Id,
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
pub struct PutBlockArgs {
    pub bytes: Vec<u8>,
}

#[derive(Serialize)]
pub struct PutBlockResponse {
    pub id: ids::Id,
}
