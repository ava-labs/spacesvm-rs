use std::{
    collections::BTreeMap,
    io::{Error, ErrorKind, Result},
    str::FromStr,
};

use avalanche_types::ids;
use ethers_core::types::transaction::eip712::{
    EIP712Domain, Eip712DomainType as Type, TypedData, Types,
};

use super::{base, claim, delete, set, tx::TransactionType, unsigned};

pub const TD_STRING: &str = "string";
pub const TD_U64: &str = "u64";
pub const TD_BYTES: &str = "bytes";
pub const TD_BLOCK_ID: &str = "blockId";
pub const TD_SPACE: &str = "space";
pub const TD_KEY: &str = "key";
pub const TD_VALUE: &str = "value";

pub type TypedDataMessage = BTreeMap<String, jsonrpc_core::Value>;

pub fn create_typed_data(
    tx_type: TransactionType,
    tx_fields: Vec<Type>,
    message: TypedDataMessage,
) -> TypedData {
    let mut types = Types::new();
    types.insert(tx_type.to_string(), tx_fields);
    types.insert(
        "EIP712Domain".to_owned(),
        vec![
            Type {
                name: "name".to_owned(),
                r#type: "string".to_owned(),
            },
            Type {
                name: "magic".to_owned(),
                r#type: "uint64".to_owned(),
            },
        ],
    );

    let domain = EIP712Domain {
        name: Some("SpacesVm".to_owned()),
        version: None,
        chain_id: None,
        verifying_contract: None,
        salt: None,
    };
    return TypedData {
        types,
        message,
        domain,
        primary_type: tx_type.to_string(),
    };
}

// Attempts to return the base tx from typed data.
pub fn parse_base_tx(typed_data: &TypedData) -> Result<base::Tx> {
    if let Some(r_block_id) = typed_data.message.get(TD_BLOCK_ID) {
        if let Some(id) = r_block_id.as_str() {
            let block_id = ids::Id::from_str(id).map_err(|e| {
                Error::new(
                    ErrorKind::InvalidData,
                    format!("failed to parse id from string: {}: {}", id, e),
                )
            })?;
            return Ok(base::Tx { block_id });
        }
    }

    Err(Error::new(
        ErrorKind::InvalidData,
        format!("invalid typed data: {}", TD_BLOCK_ID),
    ))
}

// Attempts to return and unsigned transaction from typed data.
pub fn parse_typed_data(
    typed_data: &TypedData,
) -> Result<Box<dyn unsigned::Transaction + Send + Sync>> {
    let base_tx = parse_base_tx(&typed_data).map_err(|e| {
        Error::new(
            ErrorKind::InvalidData,
            format!("failed to parse base tx: {:?}", e),
        )
    })?;

    // each tx has space and key
    let space = get_message_value(&typed_data.message, TD_SPACE.to_owned())
        .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;

    let key = get_message_value(&typed_data.message, TD_SPACE.to_owned())
        .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;

    match typed_data.primary_type.as_str() {
        // claim tx
        tx if tx == TransactionType::Claim.as_str() => Ok(Box::new(claim::Tx {
            base_tx,
            space: space.to_string(),
        })),

        // set tx
        tx if tx == TransactionType::Set.as_str() => {
            let value = get_message_value(&typed_data.message, TD_VALUE.to_owned())
                .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
            if let Some(value_str) = value.as_str() {
                return Ok(Box::new(set::Tx {
                    base_tx,
                    space: space.to_string(),
                    key: key.to_string(),
                    value: Vec::from(value_str),
                }));
            }
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("invalid typed data: {}", TD_VALUE),
            ));
        }

        // delete tx
        tx if tx == TransactionType::Delete.as_str() => Ok(Box::new(delete::Tx {
            base_tx,
            space: space.to_string(),
            key: key.to_string(),
        })),

        _ => Err(Error::new(
            ErrorKind::Other,
            "transaction type Unknown is not valid",
        )),
    }
}

/// Attempts to check for a key in the message map. If it exists return the value.
pub fn get_message_value(message: &TypedDataMessage, key: String) -> Result<serde_json::Value> {
    match message.get(&key) {
        Some(value) => Ok(value.to_owned()),
        None => Err(Error::new(
            ErrorKind::NotFound,
            format!("typed data key missing: {:?}", key),
        )),
    }
}

#[tokio::test]
async fn signature_recovers() {
    use avalanche_types::key;
    use ethers_core::types::transaction::eip712::Eip712;

    let secret_key = key::secp256k1::private_key::Key::generate().unwrap();
    let public_key = secret_key.to_public_key();

    let tx_data = crate::chain::tx::unsigned::TransactionData {
        typ: TransactionType::Claim,
        space: "kvs".to_string(),
        key: String::new(),
        value: vec![],
    };
    let resp = tx_data.decode();
    assert!(resp.is_ok());
    let utx = resp.unwrap();
    // let hash = hash_structured_data(&utx.typed_data().await).unwrap();
    let typed_data = utx.typed_data().await.unwrap();
    let hash = typed_data.struct_hash().unwrap();

    let sig = secret_key.sign_digest(&hash).unwrap();
    let sender = key::secp256k1::public_key::Key::from_signature(&hash, &sig.to_bytes()).unwrap();
    assert_eq!(public_key.to_string(), sender.to_string());
    assert_eq!(public_key, sender,);

    let tx_data = crate::chain::tx::unsigned::TransactionData {
        typ: TransactionType::Set,
        space: "kvs".to_string(),
        key: "foo".to_string(),
        value: "bar".as_bytes().to_vec(),
    };
    let resp = tx_data.decode();
    assert!(resp.is_ok());
    let mut utx = resp.unwrap();
    utx.set_block_id(avalanche_types::ids::Id::from_slice("duuuu".as_bytes()))
        .await;
    let typed_data = utx.typed_data().await.unwrap();
    let hash = typed_data.struct_hash().unwrap();

    let sig = secret_key.sign_digest(&hash).unwrap();
    let sender = key::secp256k1::public_key::Key::from_signature(&hash, &sig.to_bytes()).unwrap();
    assert_eq!(public_key.to_string(), sender.to_string());
    assert_eq!(public_key, sender,);
}
