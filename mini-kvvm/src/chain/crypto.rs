use std::io::{Result, Error, ErrorKind};

use avalanche_types::key::ECDSA_RECOVERABLE_SIG_LEN;

use rand::{RngCore, thread_rng};
use secp256k1::{self, recover_ecdsa, PublicKey, ecdsa::RecoverableSignature};

const V_OFFSET: usize = 64;
const LEGACY_SIG_ADJ: usize = 27;
pub const MESSAGE_SIZE: usize = 32;

/// A (hashed) message input to an ECDSA signature.
pub struct Message([u8; constants::MESSAGE_SIZE]);

// pub fn derive_sender(dh: &[u8], private: &PrivateKey) -> Result<Vec<u8>> {
pub fn derive_sender(dh: &[u8], sig: &[u8]) -> Result<PublicKey> {
     if sig.len() != ECDSA_RECOVERABLE_SIG_LEN {
        return Err(Error::new(ErrorKind::Other, format!("invalid signature")))
     }

    // Avoid modifying the signature in place in case it is used elsewhere
    let mut sig_copy: [u8; ECDSA_RECOVERABLE_SIG_LEN] = [0; ECDSA_RECOVERABLE_SIG_LEN];
    sig_copy.clone_from_slice(sig);
    if sig_copy[V_OFFSET] >= LEGACY_SIG_ADJ {
        sig_copy[V_OFFSET] = LEGACY_SIG_ADJ
    }

    // TODO what is the proper recovery id in this context?
    let recovery_sig = RecoverableSignature::from_compact(&sig_copy, 1);
    if recovery_sig.is_err() {
        return Err(Error::new(ErrorKind::Other, recovery_sig.unwrap_err()))
    }

    let vrfy = Secp256k1::verification_only();
    let public_key = vrfy.recover_ecdsa(dh,recovery_sig.unwrap());
     if public_key.is_err() {
        return Err(Error::new(ErrorKind::Other, public_key.unwrap_err()))
    }

    Ok(public_key.unwrap())
}

#[test]
fn test_recovery() {
    let mut s = Secp256k1::new();

}