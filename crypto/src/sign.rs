use crate::KeyPair;
use ed25519_dalek::Signer;

/// signs data with an ed25519 keypair, returns the signature bytes.
pub fn sign_data(data: &[u8], keypair: &KeyPair) -> Vec<u8> {
    let signature = keypair.signing_key().sign(data);
    signature.to_bytes().to_vec()
}
