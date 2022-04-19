pub mod engine;
pub mod genesis;
pub mod http;
pub mod kvvm;
pub mod plugin;
pub mod service;
pub mod util;

pub mod aliasreaderpb {
    tonic::include_proto!("aliasreader");
}

pub mod appsenderpb {
    tonic::include_proto!("appsender");
}

pub mod httppb {
    tonic::include_proto!("http");
}

pub mod keystorepb {
    tonic::include_proto!("keystore");
}

pub mod messengerpb {
    tonic::include_proto!("messenger");
}

pub mod metrics {
    tonic::include_proto!("metrics");
}

pub mod rpcdbpb {
    tonic::include_proto!("rpcdb");
}

pub mod sharedmemorypb {
    tonic::include_proto!("sharedmemory");
}

pub mod subnetlookuppb {
    tonic::include_proto!("subnetlookup");
}

pub mod vmpb {
    tonic::include_proto!("vm");
}
