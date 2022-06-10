use avalanche_proto::helloworld::{greeter_client::GreeterClient, HelloRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = GreeterClient::connect("http://[::1]:50051").await?;
    let request = tonic::Request::new(HelloRequest {
        name: "Avalanche Rustaceans!".into(),
    });
    let response = client.say_hello(request).await?;

    println!("response: {:?}", response);
    Ok(())
}
