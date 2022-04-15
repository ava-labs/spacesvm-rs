pub mod engine;
pub mod error;
pub mod genesis;
pub mod kvvm;
pub mod plugin;

pub(crate) mod vm {
    tonic::include_proto!("vm");
}

pub mod metrics {
    tonic::include_proto!("metrics");
}
