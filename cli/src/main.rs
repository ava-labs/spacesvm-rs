use avalanche_types::key;
use clap::{Parser, Subcommand};
use jsonrpc_client_transports::{transports, RpcError};
use jsonrpc_core::futures;
use mini_kvvm::api::ServiceClient as Client;
use mini_kvvm::api::{DecodeTxArgs, IssueTxArgs, ResolveArgs};
use mini_kvvm::chain::tx::{decoder, tx::TransactionType, unsigned::TransactionData};
use std::error;
use std::fs::File;
use std::io::{Result, Write};
use std::path::Path;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Endpoint for RPC calls.
    #[clap(long)]
    endpoint: String,

    /// Private key file.
    #[clap(long, default_value = ".mini-kvvm-cli-pk")]
    private_key_file: String,

    /// Which subcommand to call.
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Bucket {
        bucket: String,
    },
    Set {
        bucket: String,
        key: String,
        value: String,
    },
    Delete {
        bucket: String,
        key: String,
    },
    Get {
        bucket: String,
        key: String,
    },
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn error::Error>> {
    let cli = Cli::parse();

    let secret_key = get_or_create_pk(&cli.private_key_file)?;
    let connection = transports::http::connect::<Client>(&cli.endpoint);
    let client = futures::executor::block_on(connection)?;
    ping(&client).await?;

    if let Command::Get { bucket, key } = &cli.command {
        futures::executor::block_on(client.resolve(ResolveArgs {
            bucket: bucket.as_bytes().to_vec(),
            key: key.as_bytes().to_vec(),
        }))
        .map_err(|e| e.to_string())?;
    }

    let tx = command_to_tx(cli.command)?;

    futures::executor::block_on(sign_and_submit(&client, &secret_key, tx))
        .map_err(|e| e.to_string().into())
}

fn command_to_tx(command: Command) -> Result<TransactionData> {
    match command {
        Command::Bucket { bucket } => Ok(bucket_tx(bucket)),
        Command::Set { bucket, key, value } => Ok(set_tx(bucket, key, value.as_bytes().to_vec())),
        Command::Delete { bucket, key } => Ok(delete_tx(bucket, key)),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "not a supported tx",
        )),
    }
}

fn get_or_create_pk(path: &str) -> Result<key::secp256k1::private_key::Key> {
    if !Path::new(path).try_exists()? {
        let secret_key = key::secp256k1::private_key::Key::generate().unwrap();
        let mut f = File::create(path)?;
        let hex = hex::encode(&secret_key.to_bytes());
        f.write_all(hex.as_bytes())?;
        return Ok(secret_key);
    }
    let contents = std::fs::read_to_string(path)?;
    let parsed = hex::decode(contents)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    key::secp256k1::private_key::Key::from_bytes(&parsed)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
}

fn bucket_tx(bucket: String) -> TransactionData {
    TransactionData {
        typ: TransactionType::Bucket,
        bucket,
        key: "".to_string(),
        value: vec![],
    }
}

fn set_tx(bucket: String, key: String, value: Vec<u8>) -> TransactionData {
    TransactionData {
        typ: TransactionType::Set,
        bucket,
        key,
        value,
    }
}

fn delete_tx(bucket: String, key: String) -> TransactionData {
    TransactionData {
        typ: TransactionType::Delete,
        bucket,
        key,
        value: vec![],
    }
}

async fn ping(client: &Client) -> Result<()> {
    let error_handling =
        |e: RpcError| std::io::Error::new(std::io::ErrorKind::Other, e.to_string());
    let resp = client.ping().await.map_err(error_handling);
    dbg!(resp.is_ok());
    dbg!(&resp);
    Ok(())
}

async fn sign_and_submit(
    client: &Client,
    pk: &key::secp256k1::private_key::Key,
    tx_data: TransactionData,
) -> Result<()> {
    let error_handling =
        |e: RpcError| std::io::Error::new(std::io::ErrorKind::Other, dbg!(e).to_string());
    let resp = client
        .decode_tx(DecodeTxArgs { tx_data })
        .await
        .map_err(error_handling)?;

    let typed_data = &resp.typed_data;

    let dh = decoder::hash_structured_data(typed_data)?;
    let sig = pk.sign_digest(&dh.as_bytes())?;

    let resp = client
        .issue_tx(IssueTxArgs {
            typed_data: resp.typed_data,
            signature: sig.to_bytes().to_vec(),
        })
        .await
        .map_err(error_handling)?;
    println!("response: {:?}", resp);
    Ok(())
}
