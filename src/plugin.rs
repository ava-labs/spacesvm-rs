use std::{
    io::{self, Error, ErrorKind},
    path::Path,
};

use crate::vm::vm_server::{Vm, VmServer};
use log::info;
use tokio::{fs::create_dir_all, net::UnixListener};
use tokio_stream::wrappers::UnixListenerStream;
use tonic::transport::{server::NamedService, Server};
use tonic_health::server::health_reporter;

/// ref. https://github.com/ava-labs/avalanchego/blob/v1.7.10/vms/rpcchainvm/vm.go
pub const PROTOCOL_VERSION: u8 = 12;
pub const MAGIC_COOKIE_KEY: &str = "VM_PLUGIN";
pub const MAGIC_COOKIE_VALUE: &str = "dynamic";

pub const UNIX_SOCKET_PATH: &str = "/var/run/mini-kvvm-rs.sock";

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
    create_dir_all(Path::new(UNIX_SOCKET_PATH).parent().unwrap())
        .await
        .map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed tokio::fs::create_dir_all '{}'", e),
            )
        })?;

    // "go-plugin requires the gRPC Health Checking Service to be registered on your server"
    // ref. https://github.com/hashicorp/go-plugin/blob/master/docs/guide-plugin-write-non-go.md
    // ref. https://github.com/hyperium/tonic/blob/v0.7.1/examples/src/health/server.rs
    let (mut health_reporter, health_svc) = health_reporter();
    health_reporter.set_serving::<Plugin>().await;

    // ref. https://github.com/hashicorp/go-plugin/blob/master/docs/guide-plugin-write-non-go.md#4-output-handshake-information
    let handshake_msg = format!(
        "1|{}|unix|{}|grpc|",
        handshake_config.protocol_version, UNIX_SOCKET_PATH,
    );
    info!("handshake message: {}", handshake_msg);
    println!("{}", handshake_msg);

    // ref. https://github.com/hyperium/tonic/blob/v0.7.1/examples/src/uds/server.rs
    let listener = UnixListener::bind(UNIX_SOCKET_PATH)?;

    let socket_addr = listener.local_addr()?;
    info!("plugin listening on socket address {:?}", socket_addr);

    Server::builder()
        .add_service(health_svc)
        .add_service(VmServer::new(vm))
        .serve_with_incoming(UnixListenerStream::new(listener))
        .await
        .map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed serve_with_incoming '{}'", e),
            )
        })
}
