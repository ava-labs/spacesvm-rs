use std::io::{Error, ErrorKind, Result};

use avalanche_types::key::ECDSA_RECOVERABLE_SIG_LEN;
use ethereum_types::Address;
use secp256k1::{self, ecdsa, ecdsa::RecoverableSignature, PublicKey, Secp256k1};
use sha3::{Digest, Keccak256};

const V_OFFSET: usize = 64;
const LEGACY_SIG_ADJ: usize = 27;
pub const MESSAGE_SIZE: usize = 32;

// pub fn derive_sender(dh: &[u8], private: &PrivateKey) -> Result<Vec<u8>> {
pub fn derive_sender(dh: &[u8], sig: &[u8]) -> Result<PublicKey> {
    if sig.len() != ECDSA_RECOVERABLE_SIG_LEN {
        return Err(Error::new(ErrorKind::Other, format!("invalid signature")));
    }

    // Avoid modifying the signature in place in case it is used elsewhere
    let mut sig_copy: [u8; ECDSA_RECOVERABLE_SIG_LEN] = [0; ECDSA_RECOVERABLE_SIG_LEN];
    sig_copy.clone_from_slice(sig);

    if usize::from(sig_copy[V_OFFSET]) >= LEGACY_SIG_ADJ {
        let offset = sig_copy[V_OFFSET];
        sig_copy[V_OFFSET] = offset - LEGACY_SIG_ADJ as u8;
    }

    // TODO what is the proper recovery id in this context?
    let recovery_id = ecdsa::RecoveryId::from_i32(1 as i32)
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

    let recovery_sig = RecoverableSignature::from_compact(&sig_copy, recovery_id)
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

    let message = secp256k1::Message::from_slice(dh)
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
    let vrfy = Secp256k1::verification_only();
    let public_key = vrfy
        .recover_ecdsa(&message, &recovery_sig)
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

    Ok(public_key)
}

pub fn public_to_address(public_key: &PublicKey) -> Address {
    let raw_key = public_key.serialize_uncompressed();
    assert_eq!(raw_key[0], 0x04);

    let hash = &Keccak256::digest(&raw_key[1..]);

    return Address::from_slice(&hash[12..]);
}