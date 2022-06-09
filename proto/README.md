
# avalanche-proto

Protobuf generated client and server resources for Avalanche gRPC in rust.
The generated stubs use the upstream [avalanchego/proto](https://github.com/ava-labs/avalanchego-internal/tree/dev/proto) definitions as the
source of truth and versioning will align with avalanchego releases.

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
                format!("failed serve_with_incoming '{}'", e),
            )
        })
```

## note

Support for avalanchego >= 1.7.11 