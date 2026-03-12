use crate::{CryptoError, PublicKey};
use ed25519_dalek::{Signature, Verifier};

/// checks an ed25519 signature against the data and public key.
pub fn verify_signature(
    data: &[u8],
    signature_bytes: &[u8],
    public_key: &PublicKey,
) -> Result<(), CryptoError> {
    let sig_bytes: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| CryptoError::InvalidSignature("signature must be 64 bytes".into()))?;
    let signature = Signature::from_bytes(&sig_bytes);
    public_key
        .verifying_key()
        .verify(data, &signature)
        .map_err(|_| CryptoError::VerificationFailed)
}
