/// ref. https://github.com/hyperium/tonic/tree/master/tonic-build
fn main() {
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile(
            &[
                // e.g.,
                // git submodule add https://github.com/prometheus/client_model
                // git submodule update --remote
                "client_model/io/prometheus/client/metrics.proto",
                "proto/vm.proto",
            ],
            &["client_model", "proto"],
        )
        .unwrap();
}
