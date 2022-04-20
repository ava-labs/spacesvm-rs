/// ref. https://github.com/hyperium/tonic/tree/master/tonic-build
fn main() {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(
            &[
                "proto/aliasreader.proto",
                "proto/appsender.proto",
                "proto/keystore.proto",
                "proto/messenger.proto",
                "proto/metrics.proto",
                "proto/rpcdb.proto",
                "proto/sharedmemory.proto",
                "proto/subnetlookup.proto",
                "proto/vm.proto",
                "proto/http.proto",
            ],
            &["proto"],
        )
        .unwrap();
}
