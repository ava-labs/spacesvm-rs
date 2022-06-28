use jsonrpc_core::*;
use serde::{Serialize, Deserialize};
use avalanche_types::ids::Id;
use crate::block::Block;

pub const PUBLICENDPOINT: String = String::from("/kvvm-rs");

#[derive(Deserialize)]
pub struct SetStateArgs {
    pub state: u32,
}

#[derive(Serialize)]
pub struct SetStateResponse {
    pub accepted: bool
}

#[derive(Deserialize)]
pub struct GetBlockArgs {
    pub id: Id
}

#[derive(Serialize)]
pub struct GetBlockResponse {
    pub block: Block
}

#[derive(Deserialize)]
pub struct ParseBlockArgs {
    pub bytes: Box<[u8]>
}

#[derive(Serialize)]
pub struct ParseBlockResponse {
    pub block: Block
}

#[derive(Serialize)]
pub struct BuildBlockResponse {
    pub block: Block
}

#[derive(Deserialize)]
pub struct SetPreferenceArgs {
    pub id: Id
}

#[derive(Serialize)]
pub struct SetPreferenceResponse {}

#[derive(Serialize)]
pub struct LastAcceptedResponse {
    pub id: Id
}

