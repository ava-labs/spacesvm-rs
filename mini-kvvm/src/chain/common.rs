use hex::{FromHex, ToHex};
use once_cell::sync::OnceCell;
use primitive_types::H256;
pub use primitive_types::U256;
use serde::{
    de::{self, Deserialize, Deserializer, Visitor},
    Serialize, Serializer,
};
use sha3::Digest;

use std::fmt;
use std::str::FromStr;

// This entire file is lifted from qevm, thanks Ted!

#[derive(Clone)]
pub struct Bytes(Vec<u8>);

impl From<Vec<u8>> for Bytes {
    fn from(v: Vec<u8>) -> Self {
        Self(v)
    }
}

struct BytesRef<'a>(&'a [u8]);

impl<'a> fmt::LowerHex for BytesRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.encode_hex::<String>())
    }
}

impl<'a> Serialize for BytesRef<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("0x{:x}", &self))
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Default, Debug)]
pub struct Hash(H256);

impl Hash {
    #[inline(always)]
    pub fn hash(slice: &[u8]) -> Self {
        Self::from_slice(sha3::Keccak256::digest(slice).as_slice())
    }

    #[inline(always)]
    pub fn empty_bytes_hash() -> &'static Self {
        static V: OnceCell<Hash> = OnceCell::new();
        V.get_or_init(|| {
            let hasher = sha3::Keccak256::new();
            Self::from_slice(hasher.finalize().as_slice())
        })
    }

    #[inline]
    pub fn zero() -> &'static Self {
        static V: OnceCell<Hash> = OnceCell::new();
        V.get_or_init(|| Self(H256::zero()))
    }

    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    #[inline(always)]
    pub fn from_slice(s: &[u8]) -> Self {
        Self(H256::from_slice(s))
    }
}

impl From<U256> for Hash {
    fn from(u: U256) -> Self {
        let mut bytes: [u8; 32] = Default::default();
        u.to_big_endian(&mut bytes);
        Self::from_slice(&bytes)
    }
}

impl FromStr for Hash {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        Ok(Self(H256::from_str(s).map_err(|_| ())?))
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Serialize for Hash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        BytesRef::serialize(&BytesRef(self.as_bytes()), serializer)
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        let bytes = deserializer.deserialize_identifier(BytesVisitor)?.0;
        if bytes.len() != 32 {
            return Err(D::Error::invalid_length(bytes.len(), &"length of 32 bytes"));
        }
        Ok(Hash::from_slice(&bytes))
    }
}

pub struct BytesVisitor;
impl<'de> Visitor<'de> for BytesVisitor {
    type Value = Bytes;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("starts with `0x` and has even number of hex digits")
    }

    fn visit_str<E>(self, value: &str) -> Result<Bytes, E>
    where
        E: de::Error,
    {
        if value.len() < 2 {
            return Err(de::Error::invalid_length(value.len(), &self));
        }
        let bytes = value.as_bytes();
        if bytes[0] == '0' as u8 && (bytes[1] == 'x' as u8 || bytes[1] == 'X' as u8) {
            match Vec::from_hex(&value[2..]) {
                Ok(v) => Ok(v.into()),
                Err(_) => Err(de::Error::invalid_value(de::Unexpected::Str(value), &self)),
            }
        } else {
            Err(de::Error::invalid_value(de::Unexpected::Str(value), &self))
        }
    }
}
