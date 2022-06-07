use std::io::{self, Error, ErrorKind};

use avalanche_proto::{
    grpcutil,
    vm::vm_server::{Vm, VmServer},
};
use log::info;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::server::NamedService;
use tonic_health::server::health_reporter;

/// ref. https://github.com/ava-labs/avalanchego/blob/v1.7.10/vms/rpcchainvm/vm.go
pub const PROTOCOL_VERSION: u8 = 14;
pub const MAGIC_COOKIE_KEY: &str = "VM_PLUGIN";
pub const MAGIC_COOKIE_VALUE: &str = "dynamic";

/// ref. https://github.com/ava-labs/avalanchego/blob/v1.7.10/vms/rpcchainvm/vm.go
#[derive(Debug)]
pub struct HandshakeConfig {
    pub protocol_version: u8,
    pub magic_cookie_key: &'static str,
    pub magic_cookie_value: &'static str,
}

impl Default for HandshakeConfig {
    fn default() -> Self {
        Self::default()
    }
}

impl HandshakeConfig {
    pub fn default() -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            magic_cookie_key: MAGIC_COOKIE_KEY,
            magic_cookie_value: MAGIC_COOKIE_VALUE,
        }
    }
}

struct Plugin;

impl NamedService for Plugin {
    const NAME: &'static str = "plugin";
}

pub async fn serve<V>(vm: V, handshake_config: &HandshakeConfig) -> io::Result<()>
where
    V: Vm,
{
    // "go-plugin requires the gRPC Health Checking Service to be registered on your server"
    // ref. https://github.com/hashicorp/go-plugin/blob/master/docs/guide-plugin-write-non-go.md
    // ref. https://github.com/hyperium/tonic/blob/v0.7.1/examples/src/health/server.rs
    let (mut health_reporter, health_svc) = health_reporter();
    health_reporter.set_serving::<Plugin>().await;

    // ref. https://github.com/hyperium/tonic/blob/v0.7.2/examples/src/reflection/server.rs
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(avalanche_proto::rpcdb::FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(avalanche_proto::vm::FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(
            avalanche_proto::google::protobuf::FILE_DESCRIPTOR_SET,
        )
        .register_encoded_file_descriptor_set(
            avalanche_proto::io::prometheus::client::FILE_DESCRIPTOR_SET,
        )
        .register_encoded_file_descriptor_set(
            tonic_health::proto::GRPC_HEALTH_V1_FILE_DESCRIPTOR_SET,
        )
        .build()
        .expect("failed to create gRPC reflection service");

    // TODO: Add support for abstract unix sockets once supported by tonic.
    // ref. https://github.com/hyperium/tonic/issues/966
    // avalanchego currently only supports plugins listening on IP address.
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    info!("plugin listening on address {:?}", addr);

    // ref. https://github.com/hashicorp/go-plugin/blob/master/docs/guide-plugin-write-non-go.md#4-output-handshake-information
    let handshake_msg = format!("1|{}|tcp|{}|grpc|", handshake_config.protocol_version, addr);
    info!("handshake message: {}", handshake_msg);
    println!("{}", handshake_msg);

    grpcutil::default_server()
        .add_service(health_svc)
        .add_service(reflection_service)
        .add_service(VmServer::new(vm))
        .serve_with_incoming(TcpListenerStream::new(listener))
        .await
        .map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed serve_with_incoming '{}'", e),
            )
        })
}
