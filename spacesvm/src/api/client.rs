use std::{
    fs::File,
    io::{Error, ErrorKind, Result, Write},
    path::Path,
};

use crate::{
    api::{
        DecodeTxArgs, DecodeTxResponse, IssueTxArgs, IssueTxResponse, PingResponse, ResolveArgs,
        ResolveResponse,
    },
    chain::tx::{
        decoder::{self, TypedData},
        tx::TransactionType,
        unsigned::TransactionData,
    },
};
use avalanche_types::key::{
    self,
    secp256k1::{private_key::Key, signature::Sig},
};
use http::{Method, Request};
use hyper::{body, client::HttpConnector, Body, Client as HyperClient};
use jsonrpc_core::{Call, Id, MethodCall, Params, Value, Version};
use serde::de;

pub use http::Uri;

/// HTTP client for interacting with the API, assumes single threaded use.
pub struct Client<C> {
    id: u64,
    client: HyperClient<C>,
    endpoint: Uri,
    private_key: Option<Key>,
}

impl Client<HttpConnector> {
    pub fn new(endpoint: Uri) -> Self {
        let client = HyperClient::new();
        Self {
            id: 0,
            client,
            endpoint,
            private_key: None,
        }
    }
}

impl Client<HttpConnector> {
    fn next_id(&mut self) -> Id {
        let id = self.id;
        self.id = id + 1;
        Id::Num(id)
    }

    pub fn set_endpoint(mut self, endpoint: Uri) -> Self {
        self.endpoint = endpoint;
        self
    }

    pub fn set_private_key(mut self, private_key: Key) -> Self {
        self.private_key = Some(private_key);
        self
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

    /// Returns a recoverable signature from bytes.
    pub fn sign_digest(&self, dh: &[u8]) -> Result<Sig> {
        if let Some(pk) = &self.private_key {
            pk.sign_digest(dh)?;
        }
        Err(Error::new(ErrorKind::Other, "private key not set"))
    }

    /// Returns a PingResponse from client request.
    pub async fn ping(&mut self) -> Result<PingResponse> {
        let (_id, json_request) = self.raw_request("ping", &Params::None);
        let resp = self.post_de::<PingResponse>(&json_request).await?;

        Ok(resp)
    }

    /// Returns a DecodeTxResponse from client request.
    pub async fn decode_tx(&mut self, tx_data: TransactionData) -> Result<DecodeTxResponse> {
        let arg_value = serde_json::to_value(&DecodeTxArgs { tx_data })?;
        let (_id, json_request) = self.raw_request("decodeTx", &Params::Array(vec![arg_value]));
        let resp = self.post_de::<DecodeTxResponse>(&json_request).await?;

        Ok(resp)
    }

    /// Returns a IssueTxResponse from client request.
    pub async fn issue_tx(&mut self, typed_data: &TypedData) -> Result<IssueTxResponse> {
        let dh = decoder::hash_structured_data(typed_data)?;
        let sig = self.sign_digest(&dh.as_bytes())?.to_bytes().to_vec();
        log::debug!("signature: {:?}", sig);

        let arg_value = serde_json::to_value(&IssueTxArgs {
            typed_data: typed_data.to_owned(),
            signature: sig,
        })?;
        let (_id, json_request) = self.raw_request("issueTx", &Params::Array(vec![arg_value]));
        let resp = self.post_de::<IssueTxResponse>(&json_request).await?;

        Ok(resp)
    }

    /// Returns a ResolveResponse from client request.
    pub async fn resolve(&mut self, space: &str, key: &str) -> Result<ResolveResponse> {
        let arg_value = serde_json::to_value(&ResolveArgs {
            space: space.as_bytes().to_vec(),
            key: key.as_bytes().to_vec(),
        })?;
        let (_id, json_request) = self.raw_request("issueTx", &Params::Array(vec![arg_value]));
        let resp = self.post_de::<ResolveResponse>(&json_request).await?;

        Ok(resp)
    }

    /// Returns a deserialized response from client request.
    pub async fn post_de<T: de::DeserializeOwned>(&self, json: &str) -> Result<T> {
        println!("json: {}", json);
        let req = Request::builder()
            .method(Method::POST)
            .uri(self.endpoint.to_string())
            .header("content-type", "application/json-rpc")
            .body(Body::from(json.to_owned()))
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("failed to create client request: {}", e),
                )
            })?;

        let mut resp = self.client.request(req).await.map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("client post request failed: {}", e),
            )
        })?;

        let bytes = body::to_bytes(resp.body_mut())
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        // deserialize bytes to value
        let v: Value = serde_json::from_slice(&bytes).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("failed to deserialize response to value: {}", e),
            )
        })?;

        // deserialize result to T
        let resp = serde_json::from_value(v["result"].to_owned()).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("failed to deserialize response: {}", e),
            )
        })?;

        Ok(resp)
    }
}

pub fn claim_tx(space: &str) -> TransactionData {
    TransactionData {
        typ: TransactionType::Claim,
        space: space.to_owned(),
        key: String::new(),
        value: vec![],
    }
}

pub fn set_tx(space: &str, key: &str, value: &str) -> TransactionData {
    TransactionData {
        typ: TransactionType::Set,
        space: space.to_owned(),
        key: key.to_owned(),
        value: value.as_bytes().to_vec(),
    }
}

pub fn delete_tx(space: &str, key: &str) -> TransactionData {
    TransactionData {
        typ: TransactionType::Delete,
        space: space.to_owned(),
        key: key.to_owned(),
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
    let (id, _) = cli.raw_request("ping", &Params::None);
    assert_eq!(id, jsonrpc_core::Id::Num(0));
    let (id, req) = cli.raw_request("ping", &Params::None);
    assert_eq!(id, jsonrpc_core::Id::Num(1));
    assert_eq!(
        req,
        r#"{"jsonrpc":"2.0","method":"ping","params":null,"id":1}"#
    );
}
