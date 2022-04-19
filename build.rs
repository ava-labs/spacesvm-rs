/// ref. https://github.com/hyperium/tonic/tree/master/tonic-build
fn main() {
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile(
            &[
                "proto/appsender.proto",
                "proto/metrics.proto",
                "proto/vm.proto",
                "proto/http.proto",
            ],
            &["proto"],
        )
        .unwrap();
}
