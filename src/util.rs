use tokio::net::TcpListener;
use tonic::transport::Channel;

pub async fn new_listener() -> TcpListener {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener
}

pub async fn dial(addr: &'static str) -> Channel {
    let channel = Channel::from_static(addr).connect().await.unwrap();
    channel
}
