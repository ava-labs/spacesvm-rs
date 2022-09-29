use std::{
    collections::{HashMap, HashSet},
    io::Result,
};

use radix_fmt::radix;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct Type {
    name: String,
    typ: String,
}


pub type Types = HashMap<String, Vec<Type>>;

pub type TypedDataMessage = HashSet<String>;

// TypedDataDomain represents the domain part of an EIP-712 message.
pub struct TypedDataDomain {
    pub name: String,
    pub magic: String,
}

pub struct TypedDataInterior {
    types: Types,
    primary_type: String,
    domain: TypedDataDomain,
    message: TypedDataMessage,
}

pub fn mini_kvvm_domain(m: u64) -> TypedDataDomain {
    return TypedDataDomain {
        name: "MiniKvvm".to_string(),
        magic: radix(m, 10).to_string(),
    };
}

pub trait TypedData {
    // Generates a keccak256 hash of the encoding of the provided data
    fn hash_struct(&self, primary_type: String, data: TypedDataMessage) -> Result<&'static [u8]>;

    // Returns an array of custom types ordered by their hierarchical reference tree
    fn dependencies(&self, primary_type: String, found: Vec<String>) -> Vec<String>;

    /// Generates the following encoding:
    /// `name ‖ "(" ‖ member₁ ‖ "," ‖ member₂ ‖ "," ‖ … ‖ memberₙ ")"`
    ///
    /// each member is written as `type ‖ " " ‖ name` encodings cascade down and are sorted by name
    fn encode_type(&self, primary_type: String) -> Result<&'static [u8]>;

    /// Creates the keccak256 hash  of the data
    fn type_hash(&self, primary_type: String) -> Result<&'static [u8]>;

    /// Generates the following encoding:
    /// `enc(value₁) ‖ enc(value₂) ‖ … ‖ enc(valueₙ)`
    ///
    /// each encoded member is 32-byte long
    fn encode_data(
        &self,
        primary_type: String,
        data: HashSet<String>,
        depth: usize,
    ) -> Result<&'static [u8]>;

    /// Deals with the primitive values found
    /// while searching through the typed data
    fn encode_primative_value(
        &self,
        enc_type: String,
        encValue: HashSet<String>,
        depth: usize,
    ) -> Result<&'static [u8]>;
}

pub fn digest_hash(td: Box<dyn TypedData>) -> Result<Vec<u8>> {
    let typed_data_hash = td.into();
    //TODO impl me
}
