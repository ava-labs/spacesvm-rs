use super::common;

pub fn value_hash(v: &[u8]) -> common::Hash {
    return common::Hash::hash(v);
}
