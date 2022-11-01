use avalanche_types::jsonrpc::avm::IssueTxRequest;
use clap::{Parser, Subcommand};
use jsonrpc_client_transports::{transports, RpcError, RpcChannel};
use jsonrpc_core::{futures, serde_json, MethodCall, serde};
use log::debug;
use mini_kvvm::api::{ServiceClient as Client, IssueTxResponse, DecodeTxResponse, PingResponse};
use mini_kvvm::api::{DecodeTxArgs, IssueTxArgs};
use mini_kvvm::chain::crypto;
use mini_kvvm::chain::tx::decoder::TypedData;
use mini_kvvm::chain::tx::{decoder, tx::TransactionType, unsigned::TransactionData};
use secp256k1::{rand, SecretKey};
use serde::{Serialize, Deserialize};
use std::error;
use std::fs::File;
use std::io::{Result, Write, Error, ErrorKind};
use std::path::Path;
use http_manager;

const API_URL_PATH: &str = "/ext/vm/qBnAKUQ2mxiB1JdqsPPU7Ufuj1XmPLpnPTRvZEpkYZBmK6UjE/public";

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
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct Data<T>{
    pub jsonrpc: String,
    pub id: u32,
    pub method: String,
    pub params: T,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
struct Response<T>{
    pub jsonrpc: String,
    pub id: u32,
    pub result:T,
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn error::Error>> {
    // ref. https://github.com/env-logger-rs/env_logger/issues/47
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug"),
    );
    let cli = Cli::parse();

    let secret_key = get_or_create_pk(&cli.private_key_file)?;
    log::debug!("ping...");
    ping(&cli.endpoint).await?;
    log::debug!("ping succeeded");

    let tx = command_to_tx(cli.command);
    log::debug!("decoding");
    let decoded_tx = decode_tx(&cli.endpoint, tx).await?;
    log::debug!("decoded");
    let typed_data = &decoded_tx.typed_data;
    let signature = sign_td_data(&secret_key, &decoded_tx).await?;

    let resp = issue_tx(&cli.endpoint, signature, typed_data).await?;

    log::debug!("resp: {:?}",resp);

    Ok(())
 
}

fn command_to_tx(command: Command) -> TransactionData {
    match command {
        Command::Bucket { bucket } => bucket_tx(bucket),
        Command::Set { bucket, key, value } => set_tx(bucket, key, value.as_bytes().to_vec()),
        Command::Delete { bucket, key } => delete_tx(bucket, key),
    }
}

fn get_or_create_pk(path: &str) -> Result<SecretKey> {
    if !Path::new(path).try_exists()? {
        let secret_key = SecretKey::new(&mut rand::thread_rng());
        let mut f = File::create(path)?;
        let hex = hex::encode(&secret_key.secret_bytes());
        f.write_all(hex.as_bytes())?;
        return Ok(secret_key);
    }
    let contents = std::fs::read_to_string(path)?;
    let parsed = hex::decode(contents)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    Ok(SecretKey::from_slice(&parsed)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?)
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

async fn ping(http_rpc: &str) -> Result<PingResponse> {
    let joined = http_manager::join_uri(http_rpc, API_URL_PATH)?;
    log::debug!("post request url path: {}", joined.as_str());

    // TODO: use builder pattern.
    let data:Data<()> = Data{
        jsonrpc: "2.0".to_owned(),
        id: 1 ,
        method: "ping".to_owned(),
        params: (),
    };

    let payload = serde_json::to_string(&data)?;
    log::debug!("request payload: {}", payload);

    let rb = http_manager::post_non_tls(http_rpc, API_URL_PATH, &payload).await?;

    let resp: Response<PingResponse> = serde_json::from_slice(&rb)
    .map_err(|e| Error::new(ErrorKind::InvalidData, format!("failed to decode: {}", e)))?;

    Ok(resp.result)
}

pub async fn decode_tx(http_rpc: &str, tx: TransactionData) -> Result<DecodeTxResponse> {
    let joined = http_manager::join_uri(http_rpc, API_URL_PATH)?;
    log::info!("decode_tx: post request url path: {}", joined.as_str());

    let param = DecodeTxArgs{ tx_data: tx };

    let mut params = Vec::with_capacity(1);
    params.push(param);

    // TODO: use builder pattern.
    let data:Data<Vec<DecodeTxArgs>> = Data{
        jsonrpc: "2.0".to_owned(),
        id: 1 ,
        method: "decode_tx".to_owned(),
        params,
    };

    let payload = serde_json::to_string(&data)?;
    log::debug!("decode_tx: request payload: {}", payload);

    let rb = http_manager::post_non_tls(http_rpc, API_URL_PATH, &payload).await?;

    let string_with_escapes = std::str::from_utf8(&rb).unwrap();

    log::debug!("string {}",string_with_escapes);

    let resp: Response<DecodeTxResponse> = serde_json::from_slice(&rb)
    .map_err(|e| Error::new(ErrorKind::InvalidData, format!("failed to decode tx: {}", e)))?;

    Ok(resp.result)
}

pub async fn sign_td_data(pk: &SecretKey, decoded_tx: &DecodeTxResponse) -> Result<String> {
    let dh = decoder::hash_structured_data(&decoded_tx.typed_data)?;
    let signature = crypto::sign(&dh.as_bytes(), &pk)?;

    Ok(hex::encode(signature))
}

pub async fn issue_tx(http_rpc: &str, signature: String, typed_data: &TypedData) -> Result<IssueTxResponse> {
    let joined = http_manager::join_uri(http_rpc, API_URL_PATH)?;
    log::info!("issue_tx: post request url path: {}", joined.as_str());

    let param = IssueTxArgs{ typed_data: typed_data.to_owned(), signature };

    let mut params = Vec::with_capacity(1);
    params.push(param);

    // TODO: use builder pattern.
    let data:Data<Vec<IssueTxArgs>> = Data{
        jsonrpc: "2.0".to_owned(),
        id: 1 ,
        method: "issue_tx".to_owned(),
        params,
    };

    let payload = serde_json::to_string(&data)?;
    log::debug!("request payload: {}", payload);

    let rb = http_manager::post_non_tls(http_rpc, API_URL_PATH, &payload).await?;

     let string_with_escapes = std::str::from_utf8(&rb).unwrap();

    log::debug!("string {}",string_with_escapes);

    let resp: Response<IssueTxResponse> = serde_json::from_slice(&rb)
    .map_err(|e| Error::new(ErrorKind::InvalidData, format!("failed to decode issue_tx: {}", e)))?;

    Ok(resp.result)
}

#[tokio::test]
async fn test_stuff() {
    use avalanche_types::ids;
    use mini_kvvm::api;

    let tx_data = mini_kvvm::chain::tx::unsigned::TransactionData {
        typ: TransactionType::Bucket,
        bucket: "kvs".to_string(),
        key: "".to_string(),
        value: vec![],
    };

    let resp = tx_data.decode();
    assert!(resp.is_ok());

    let mut utx = resp.unwrap();

    utx.set_block_id(ids::Id::empty()).await;
    let typed_data = utx.typed_data().await;

    let string = serde_json::to_string(&api::DecodeTxResponse { typed_data }).unwrap();

    let decoded_tx: api::DecodeTxResponse = serde_json::from_str(&string).unwrap();

    let typed_data = &decoded_tx.typed_data;

    let secret_key = get_or_create_pk(".mini-kvvm-cli-pk");
    assert!(secret_key.is_ok());

    let signature = sign_td_data(&secret_key.unwrap(), &decoded_tx).await;
    assert!(signature.is_ok());

    let param = IssueTxArgs{ typed_data: typed_data.to_owned(), signature: signature.unwrap() };

    let mut params = Vec::with_capacity(1);
    params.push(param); 

    let data:Data<Vec<IssueTxArgs>> = Data{
        jsonrpc: "2.0".to_owned(),
        id: 1 ,
        method: "issue_tx".to_owned(),
        params: params,
    };

    let payload = serde_json::to_string(&data);
    assert!(payload.is_ok());

    let payload = payload.unwrap();
    println!("call: {}", payload);

    let data: Data<Vec<IssueTxArgs>> = serde_json::from_str(&payload).unwrap();

    // if let Some(params) = data.params{
    //     let out = params.first().unwrap();
    // } else {
    //     panic!("shit")
    // }

    

}

fn unescape(s: &str) -> serde_json::Result<String> {
    serde_json::from_str(&format!("\"{}\"", s))
}