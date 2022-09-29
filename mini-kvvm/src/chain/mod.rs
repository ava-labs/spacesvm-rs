pub mod crypto;
pub mod storage;
pub mod tx;
pub mod vm;

#[cfg(test)]
mod tests {
    use super::{crypto::derive_sender, *};
    use secp256k1::{rand, PublicKey, SecretKey};

    #[test]
    fn signature_recovers() {
        let secret_key = SecretKey::new(&mut rand::thread_rng());
        let public_key = PublicKey::from_secret_key_global(&secret_key);

        let hash = keccak_hash::keccak("yolo message".as_bytes());
        let sig = crypto::sign(&hash.as_bytes(), &secret_key).unwrap();
        let sender = derive_sender(&hash.as_bytes(), &sig).unwrap();
        assert_eq!(public_key.to_string(), sender.to_string(),)
    }
}
