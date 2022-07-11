use jsonrpc_core::*;
use serde::{Deserialize, Serialize};

pub const PUBLICENDPOINT: &str = "/kvvm-rs";

//This is a placeholder meant to represent actual block data
#[derive(Serialize, Deserialize)]
pub struct Block {
    pub placehold: String
}

//This is a placeholder meant to represent actual IDs
#[derive(Serialize, Deserialize)]
pub struct Id {
    pub placehold: String
}

#[derive(Serialize, Deserialize)]
pub struct BuildBlockResponse {
    pub block: Block,
}

#[derive(Serialize, Deserialize)]
pub struct GetBlockArgs {
    pub id: Id,
}

#[derive(Serialize, Deserialize)]
pub struct GetBlockResponse {
    pub block: Block,
}

#[derive(Serialize, Deserialize)]
pub struct LastAcceptedResponse {
    pub id: Id,
}

#[derive(Serialize, Deserialize)]
pub struct ParseBlockArgs {
    pub bytes: Box<[u8]>,
}

#[derive(Serialize, Deserialize)]
pub struct ParseBlockResponse {
    pub block: Block,
}

#[derive(Serialize, Deserialize)]
pub struct SetStateArgs {
    pub state: u32,
}

#[derive(Serialize, Deserialize)]
pub struct SetStateResponse {
    pub accepted: bool,
}

#[derive(Serialize, Deserialize)]
pub struct SetPreferenceArgs {
    pub id: Id,
}









