pub mod engine;
pub mod genesis;
pub mod kvvm;
pub mod plugin;

pub mod vmpb {
    tonic::include_proto!("vm");
}

pub mod metrics {
    tonic::include_proto!("metrics");
}
