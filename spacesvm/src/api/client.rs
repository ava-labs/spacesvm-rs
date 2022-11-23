use std::{
    fs::File,
    io::{Result, Write},
    path::Path,
};

use crate::{
    api::{
        DecodeTxArgs, DecodeTxResponse, IssueTxArgs, IssueTxResponse, PingResponse, ResolveArgs,
        ResolveResponse,
    },
    chain::tx::{tx::TransactionType, unsigned::TransactionData},
};
use avalanche_types::key;
use http::{Method, Request};
use hyper::{body, client::HttpConnector, Body, Client as HyperClient};
use jsonrpc_core::{Call, Id, MethodCall, Params, Version};
use serde::de;

pub use http::Uri;

pub struct Client<C> {
    id: u64,
    client: HyperClient<C>,
    pub uri: Uri,
}

impl Client<HttpConnector> {
    pub fn new(uri: Uri) -> Self {
        let client = HyperClient::new();
        Self { id: 0, client, uri }
    }
}

impl Client<HttpConnector> {
    fn next_id(&mut self) -> Id {
        let id = self.id;
        self.id = id + 1;
        Id::Num(id)
    }

    /// Returns a serialized json request as string and the request id.
    pub fn raw_request(&mut self, method: &str, params: &Params) -> (Id, String) {
        let id = self.next_id();
        let request = jsonrpc_core::Request::Single(Call::MethodCall(MethodCall {
            jsonrpc: Some(Version::V2),
            method: method.to_owned(),
            params: params.to_owned(),
            id: id.clone(),
        }));
        (
            id,
            serde_json::to_string(&request).expect("jsonrpc request should be serializable"),
        )
    }

    /// Returns a PingResponse from client request.
    pub async fn ping(&mut self) -> Result<PingResponse> {
        let params: Params = serde_json::from_str("{}")?;
        let (_id, json_request) = self.raw_request("ping", &params);
        let resp = self.post_de::<PingResponse>(&json_request).await?;

        Ok(resp)
    }

    /// Returns a DecodeTxResponse from client request.
    pub async fn decode_tx(&mut self, args: DecodeTxArgs) -> Result<DecodeTxResponse> {
        let arg_bytes = serde_json::to_vec(&args)?;
        let params: Params = serde_json::from_slice(&arg_bytes)?;
        let (_id, json_request) = self.raw_request("decodeTx", &params);
        let resp = self.post_de::<DecodeTxResponse>(&json_request).await?;

        Ok(resp)
    }

    /// Returns a IssueTxResponse from client request.
    pub async fn issue_tx(&mut self, args: IssueTxArgs) -> Result<IssueTxResponse> {
        let arg_bytes = serde_json::to_vec(&args)?;
        let params: Params = serde_json::from_slice(&arg_bytes)?;
        let (_id, json_request) = self.raw_request("issueTx", &params);
        let resp = self.post_de::<IssueTxResponse>(&json_request).await?;

        Ok(resp)
    }

    /// Returns a ResolveResponse from client request.
    pub async fn resolve(&mut self, args: ResolveArgs) -> Result<ResolveResponse> {
        let arg_bytes = serde_json::to_vec(&args)?;
        let params: Params = serde_json::from_slice(&arg_bytes)?;
        let (_id, json_request) = self.raw_request("resolve", &params);
        let resp = self.post_de::<ResolveResponse>(&json_request).await?;

        Ok(resp)
    }

    /// Returns a deserialized response from client request.
    pub async fn post_de<T: de::DeserializeOwned>(&self, json: &str) -> Result<T> {
        let req = Request::builder()
            .method(Method::POST)
            .uri(self.uri.to_string())
            .header("content-type", "application/json-rpc")
            .body(Body::from(json.to_owned()))
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("failed to create client request: {}", e),
                )
            })?;

        let resp = self.client.request(req).await.map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("client post request failed: {}", e),
            )
        })?;

        let bytes = body::to_bytes(resp.into_body())
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        let resp = serde_json::from_slice(&bytes).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("failed to create client request: {}", e),
            )
        })?;

        Ok(resp)
    }
}

pub fn claim_tx(space: String) -> TransactionData {
    TransactionData {
        typ: TransactionType::Claim,
        space,
        key: String::new(),
        value: vec![],
    }
}

pub fn set_tx(space: String, key: String, value: Vec<u8>) -> TransactionData {
    TransactionData {
        typ: TransactionType::Set,
        space,
        key,
        value,
    }
}

pub fn delete_tx(space: String, key: String) -> TransactionData {
    TransactionData {
        typ: TransactionType::Delete,
        space,
        key,
        value: vec![],
    }
}

/// Returns a private key from a given path or creates new.
pub fn get_or_create_pk(path: &str) -> Result<key::secp256k1::private_key::Key> {
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

#[tokio::test]
async fn test_raw_request() {
    let mut cli = Client::new(Uri::from_static("http://test.url"));
    let params: Params = serde_json::from_str("{}").unwrap();
    let (id, _) = cli.raw_request("ping", &params);
    assert_eq!(id, jsonrpc_core::Id::Num(0));
    let (id, req) = cli.raw_request("ping", &params);
    assert_eq!(id, jsonrpc_core::Id::Num(1));
    assert_eq!(
        req,
        r#"{"jsonrpc":"2.0","method":"ping","params":{},"id":1}"#
    );
}
