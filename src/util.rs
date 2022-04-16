use tokio::net::TcpListener;
use tonic::transport::Channel;

pub struct Grpc;

impl Grpc {
    async fn new_listener() -> TcpListener {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        listener
    }

    async fn dial(addr: &'static str) -> Channel {
        let channel = Channel::from_static(addr).connect().await.unwrap();
        channel
    }
}
