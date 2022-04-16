pub mod engine;
pub mod genesis;
pub mod http;
pub mod kvvm;
pub mod plugin;
pub mod util;

pub mod vmpb {
    tonic::include_proto!("vm");
}

pub mod httppb {
    tonic::include_proto!("http");
}

pub mod metrics {
    tonic::include_proto!("metrics");
}
