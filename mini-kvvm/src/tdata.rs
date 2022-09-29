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
//pub fn digest_hash(td: Box<dyn TypedData>) -> Result<Vec<u8>> {
//    let typed_data_hash = td.into();
//    //TODO impl me
//}