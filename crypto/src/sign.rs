use crate::KeyPair;
use ed25519_dalek::Signer;

/// Sign data with an Ed25519 key pair and return the signature bytes.
pub fn sign_data(data: &[u8], keypair: &KeyPair) -> Vec<u8> {
    let signature = keypair.signing_key().sign(data);
    signature.to_bytes().to_vec()
}
