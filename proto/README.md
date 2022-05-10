
# avalanche-proto

Protobuf generated client and server resources for Avalanche gRPC in rust.
The generated stubs use the upstream [avalanchego/proto](https://github.com/ava-labs/avalanchego-internal/tree/dev/proto) definitions as the
source of truth and versioning will align with avalanchego releases.

## usage

```
use avalanche_proto::{
    vm_server::{Vm, VmServer},
    appsender::app_sender_client::AppSenderClient,
    messenger::messenger_client::MessengerClient,
};
```

## note

Support for avalanchego >= 1.7.11 