pub mod genesis;
pub mod kvvm;
pub mod plugin;

pub mod vm {
    tonic::include_proto!("vm");
}

pub mod metrics {
    tonic::include_proto!("metrics");
}
