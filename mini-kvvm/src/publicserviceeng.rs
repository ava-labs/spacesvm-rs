use crate::block::Block;
use avalanche_proto::google::protobuf::Timestamp;
use avalanche_proto::vm;
use avalanche_types::ids::Id;
use jsonrpc_core::*;
use prost::bytes::Bytes;
use serde::{Deserialize, Serialize};
use bytes::Bytes as StandardBytes;

use std::{io::Result, convert::{TryFrom, TryInto}};

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

impl TryInto<vm::VersionedDbServer> for VersionedDbServer {
    type Error = ();

    fn try_into(self) -> std::result::Result<vm::VersionedDbServer, ()> {
        Ok(vm::VersionedDbServer {
            version: self.version,
            server_addr: self.server_addr
        })
    }
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
    pub db_servers: ::prost::alloc::vec::Vec<VersionedDbServer>,
    pub server_addr: String,
}

impl TryInto<vm::InitializeRequest> for InitializeArgs {
    type Error = ();

    fn try_into(self) -> std::result::Result<vm::InitializeRequest, ()> {
        let db: Vec<vm::VersionedDbServer> = prost::alloc::vec::Vec::new();
        for item in self.db_servers {
            db.push(item.try_into().unwrap());
        }

        Ok(vm::InitializeRequest {
            network_id: self.network_id,
            subnet_id: Bytes::from_iter(self.subnet_id),
            chain_id: Bytes::from_iter(self.chain_id),
            node_id: Bytes::from_iter(self.node_id),
            x_chain_id: Bytes::from_iter(self.x_chain_id),
            avax_asset_id: Bytes::from_iter(self.avax_asset_id),
            genesis_bytes: Bytes::from_iter(self.genesis_bytes),
            upgrade_bytes: Bytes::from_iter(self.upgrade_bytes),
            config_bytes: Bytes::from_iter(self.config_bytes),
            db_servers: db,
            server_addr: self.server_addr
        })
    }
}

#[derive(Serialize)]
pub struct InitializeResponseEng {
    pub last_accepted_id: Vec<u8>,
    pub last_accepted_parent_id: Vec<u8>,
    pub height: u64,
    pub bytes: Vec<u8>,
}

impl TryFrom<vm::InitializeResponse> for InitializeResponseEng {
    type Error = ();

    fn try_from(resp: vm::InitializeResponse) -> std::result::Result<InitializeResponseEng, ()> {
        Ok(InitializeResponseEng {
            last_accepted_id: resp.last_accepted_id.as_ref().to_vec(),
            last_accepted_parent_id: resp.last_accepted_parent_id.as_ref().to_vec(),
            height: resp.height,
            bytes: resp.bytes.as_ref().to_vec(),
        })
    }
}

#[derive(Serialize)]
pub struct SetStateResponseEng {
    pub last_accepted_id: Vec<u8>,
    pub last_accepted_parent_id: Vec<u8>,
    pub height: u64,
    pub bytes: Vec<u8>,
}

#[derive(Deserialize)]
pub struct GetBlockArgs {
    pub id: Vec<u8>,
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
    pub bytes: Vec<u8>,
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
    pub id: Vec<u8>,
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
