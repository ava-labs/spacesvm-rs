
# avalanche-proto

Protobuf generated client and server resources for Avalanche gRPC in rust.
The generated stubs use the upstream [avalanchego/proto](https://github.com/ava-labs/avalanchego-internal/tree/dev/proto)
definitions as the source of truth and versioning will align with avalanchego releases.

## versions

Support for avalanchego protocol version 15+

The release version will align with the [protocol version](https://github.com/ava-labs/avalanchego/blob/v1.7.13/vms/rpcchainvm/vm.go#L21)
for avalanchego. In our example linked above avalanchego is currently on protocol version 15.
This aligns with the minor version of the avalanche-proto release. Patches to minor releases
could include improvements and even features for protos not releated to avalanchego.

```bash
avalanche-types = { version = "0.15", features = [] } // supports avalanchego protocol version 15
```

## usage

```rust
use avalanche_proto::{
    http::{
        http_server::Http,
        HttpRequest, HandleSimpleHttpResponse, HandleSimpleHttpRequest
    },
    google::protobuf::Empty,
};
```

```rust
    use avalanche_proto::grpcutil;

    grpcutil::default_server()
        .add_service(VmServer::new(vm))
        .serve_with_incoming(TcpListenerStream::new(listener))
        .await
        .map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to serve vm service: {}", e),
            )
        })
```
