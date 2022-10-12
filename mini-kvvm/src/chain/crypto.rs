use std::io::{Error, ErrorKind, Result};

use ethereum_types::Address;
use secp256k1::{
    self,
    ecdsa::{RecoverableSignature, RecoveryId},
    Message, PublicKey, Secp256k1, SecretKey,
};
use sha3::{Digest, Keccak256};

const LEGACY_SIG_ADJ: usize = 27;
pub const MESSAGE_SIZE: usize = 32;
const ECDSA_RECOVERABLE_SIG_LEN: usize  = 65;

pub fn sign(dh: &[u8], secret: &SecretKey) -> Result<Vec<u8>> {
    let secp = Secp256k1::signing_only();
    let sig = secp.sign_ecdsa_recoverable(&Message::from_slice(dh).unwrap(), secret);
    let (recovery_id, sig_bytes) = sig.serialize_compact();
    let mut sig_vec = sig_bytes.to_vec();
    sig_vec.push(recovery_id.to_i32() as u8 + LEGACY_SIG_ADJ as u8);
    Ok(sig_vec)
}

pub fn derive_sender(dh: &[u8], sig: &[u8]) -> Result<PublicKey> {
    if sig.len() != ECDSA_RECOVERABLE_SIG_LEN {
        return Err(Error::new(ErrorKind::Other, format!("invalid signature: {}", sig.len())));
    }

    // Avoid modifying the signature in place in case it is used elsewhere
    let error_handling = |e: secp256k1::Error| Error::new(ErrorKind::Other, e.to_string());
    let mut sig_copy = Vec::new();
    sig_copy.extend_from_slice(sig);
    let mut recovery_id = sig_copy.pop().unwrap();

    // Support signers that don't apply offset (ex: ledger)
    if recovery_id >= LEGACY_SIG_ADJ as u8 {
        recovery_id -= LEGACY_SIG_ADJ as u8
    }
    let recovery_id = RecoveryId::from_i32(recovery_id as i32).map_err(error_handling)?;

    let recovery_sig =
        RecoverableSignature::from_compact(&sig_copy, recovery_id).map_err(error_handling)?;
    let message = secp256k1::Message::from_slice(dh).map_err(error_handling)?;
    let vrfy = Secp256k1::verification_only();
    let public_key = vrfy
        .recover_ecdsa(&message, &recovery_sig)
        .map_err(error_handling)?;

    Ok(public_key)
}

pub fn public_to_address(public_key: &PublicKey) -> Address {
    let raw_key = public_key.serialize_uncompressed();
    assert_eq!(raw_key[0], 0x04);

    let hash = &Keccak256::digest(&raw_key[1..]);

    return Address::from_slice(&hash[12..]);
}
