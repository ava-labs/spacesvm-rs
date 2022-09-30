pub mod service;

use avalanche_types::ids;
use jsonrpc_core::{BoxFuture, Error, ErrorCode, Result};
use jsonrpc_derive::rpc;
use serde::{Deserialize, Serialize};

use crate::chain::{storage::ValueMeta, tx, tx::decoder::TypedData};

#[rpc]
pub trait Service {
    #[rpc(name = "ping")]
    fn ping(&self) -> BoxFuture<Result<PingResponse>>;

    #[rpc(name = "issue_raw_tx")]
    fn issue_raw_tx(&self, params: IssueRawTxArgs) -> BoxFuture<Result<IssueRawTxResponse>>;

    #[rpc(name = "issue_tx")]
    fn issue_tx(&self, params: IssueTxArgs) -> BoxFuture<Result<IssueTxResponse>>;

    #[rpc(name = "decode_tx")]
    fn decode_tx(&self, params: DecodeTxArgs) -> BoxFuture<Result<DecodeTxResponse>>;

    #[rpc(name = "resolve")]
    fn resolve(&self, params: ResolveArgs) -> BoxFuture<Result<ResolveResponse>>;

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

#[derive(Deserialize, Serialize, Debug)]
pub struct PingResponse {
    pub success: bool,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct IssueRawTxArgs {
    pub tx: Vec<u8>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct IssueRawTxResponse {
    #[serde(deserialize_with = "ids::must_deserialize_id")]
    pub tx_id: ids::Id,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct IssueTxArgs {
    pub typed_data: TypedData,
    pub signature: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct IssueTxResponse {
    #[serde(deserialize_with = "ids::must_deserialize_id")]
    pub tx_id: ids::Id,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DecodeTxArgs {
    pub tx_data: tx::unsigned::TransactionData,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DecodeTxResponse {
    pub typed_data: TypedData,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ResolveArgs {
    pub bucket: Vec<u8>,
    pub key: Vec<u8>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct ResolveResponse {
    pub exists: bool,
    pub value: Vec<u8>,
    pub meta: ValueMeta,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BuildBlockArgs {}

#[derive(Deserialize, Serialize, Debug)]
pub struct BuildBlockResponse {
    pub block: Vec<u8>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetBlockArgs {
    #[serde(deserialize_with = "ids::must_deserialize_id")]
    pub id: ids::Id,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetBlockResponse {
    pub block: Vec<u8>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct LastAcceptedResponse {
    pub id: ids::Id,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ParseBlockArgs {
    pub bytes: Vec<u8>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ParseBlockResponse {
    pub block: Vec<u8>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PutBlockArgs {
    pub bytes: Vec<u8>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PutBlockResponse {
    #[serde(deserialize_with = "ids::must_deserialize_id")]
    pub id: ids::Id,
}

pub fn create_jsonrpc_error(e: std::io::Error) -> Error {
    let mut error = Error::new(ErrorCode::InternalError);
    error.message = format!("{}", e);
    error
}
