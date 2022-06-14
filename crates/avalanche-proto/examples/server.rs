use avalanche_proto::{
    grpcutil,
    helloworld::{
        self,
        greeter_server::{Greeter, GreeterServer},
        HelloReply, HelloRequest,
    },
};
use tokio::runtime::Runtime;
use tonic::{Request, Response, Status};

#[derive(Default)]
struct MyGreeter;

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        println!("Got a request from {:?}", request.remote_addr());

        let reply = HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };
        Ok(Response::new(reply))
    }
}

fn main() {
    let addr = "[::1]:50051".parse().unwrap();
    println!("Server listening on {}", addr);

    // ref. https://github.com/hyperium/tonic/blob/v0.7.2/examples/src/reflection/server.rs
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(helloworld::FILE_DESCRIPTOR_SET)
        .build()
        .expect("failed to create gRPC reflection service");
    let greeter = MyGreeter::default();

    let runtime = Runtime::new().expect("failed to create runtime");

    let server = grpcutil::default_server()
        .add_service(reflection_service)
        .add_service(GreeterServer::new(greeter))
        .serve(addr);

    runtime.block_on(server).expect("runtime failed");
}
