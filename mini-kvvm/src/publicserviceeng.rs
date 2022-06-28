use jsonrpc_core::*;
use serde::{Serialize, Deserialize};
use avalanche_types::ids::Id;
use crate::block::Block;
use prost::bytes::Bytes;
use avalanche_proto::google::protobuf::Timestamp;

pub const PUBLICENDPOINT: String = String::from("/kvvm-rs");

#[derive(Deserialize)]
pub struct SetStateArgs {
    pub state: u32,
}

#[derive(Serialize, Deserialize)]
pub struct VersionedDbServer {
    pub version: String,
    pub server_addr: String,
}

#[derive(Deserialize)]
pub struct InitializeArgs {
    pub network_id: u32,
    pub subnet_id: Vec<u8>,
    pub chain_id: Vec<u8>,
    pub node_id: Vec<u8>,
    pub x_chain_id: Vec<u8>,
    pub avax_asset_id: Vec<u8>,
    pub genesis_bytes: Vec<u8>,
    pub upgrade_bytes: Vec<u8>,
    pub config_bytes: Vec<u8>,
    pub db_servers: ::prost::alloc::vec::Vec<avalanche_proto::vm::VersionedDbServer>,
    pub server_addr: String
}

#[derive(Serialize)]
pub struct SetStateResponseEng {
    pub last_accepted_id: Vec<u8>,
    pub last_accepted_parent_id: Vec<u8>,
    pub height: u64,
    pub bytes: Vec<u8>
}

#[derive(Deserialize)]
pub struct GetBlockArgs {
    pub id: Vec<u8>
}

#[derive(Serialize)]
pub struct GetBlockResponseEng {
    pub parent_id: Vec<u8>,
    pub bytes: Vec<u8>,
    pub status: u32,
    pub height: u64,
    pub err: u32,
}

#[derive(Deserialize)]
pub struct ParseBlockArgs {
    pub bytes: Vec<u8>
}

#[derive(Serialize)]
pub struct ParseBlockResponseEng {
    pub id: Vec<u8>,
    pub parent_id: Vec<u8>,
    pub status: u32,
    pub height: u64,
}

#[derive(Serialize)]
pub struct BuildBlockResponseEng {
    pub id: Vec<u8>,
    pub parent_id: Vec<u8>,
    pub bytes: Vec<u8>,
    pub height: u64,
}

#[derive(Deserialize)]
pub struct SetPreferenceArgs {
    pub id: Vec<u8>
}

#[derive(Serialize)]
pub struct SetPreferenceResponseEng {}

#[derive(Serialize)]
pub struct LastStateSummaryResponseEng {
    pub id: Vec<u8>,
    pub height: u64,
    pub bytes: Vec<u8>,
    pub err: u32,
}
