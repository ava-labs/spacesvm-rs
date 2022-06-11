pub mod grpcutil;

include!("gen/mod.rs");

/// ref. https://github.com/ava-labs/avalanchego/blob/v1.7.13/vms/rpcchainvm/vm.go
pub const PROTOCOL_VERSION: &str = env!("CARGO_PKG_VERSION_MINOR");
