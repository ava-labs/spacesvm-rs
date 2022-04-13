pub mod genesis;
pub mod plugin;

pub mod vm {
    tonic::include_proto!("vm");
}
#[allow(unused_imports)]
use vm::vm_server::Vm;

pub mod metrics {
    tonic::include_proto!("metrics");
}

#[derive(Debug)]
pub struct KvVm {}

// impl Vm for KvVm {
// TODO
// }
