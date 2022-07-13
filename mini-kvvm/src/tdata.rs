
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct Type {
    name: String,
    typ: String,
}

pub type TypedDataMessage = HashSet<String>;

pub type Types = HashMap<String, Vec<Type>>;

// TypedDataDomain represents the domain part of an EIP-712 message.
pub struct TypedDataDomain {
	pub name: String,
	pub magic: String, 
}

struct TypedData {
	types:      Types,
	primary_type: String,
	domain:      TypedDataDomain,
	message:     TypedDataMessage,
}