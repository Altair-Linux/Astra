//! # Astra Crypto
//!
//! Cryptographic signing and verification for Astra packages.
//! Uses Ed25519 signatures for package integrity and authenticity.

mod error;
mod keyring;
mod sign;
mod verify;

pub use error::CryptoError;
pub use keyring::{KeyPair, KeyRing, PublicKey};
pub use sign::sign_data;
pub use verify::verify_signature;

/// Compute SHA-256 hash of data and return hex string.
pub fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256() {
        let hash = sha256_hex(b"hello world");
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_sign_and_verify() {
        let keypair = KeyPair::generate();
        let data = b"test package data";
        let signature = sign_data(data, &keypair);
        assert!(verify_signature(data, &signature, &keypair.public_key()).is_ok());
    }

    #[test]
    fn test_verify_wrong_key() {
        let keypair1 = KeyPair::generate();
        let keypair2 = KeyPair::generate();
        let data = b"test package data";
        let signature = sign_data(data, &keypair1);
        assert!(verify_signature(data, &signature, &keypair2.public_key()).is_err());
    }

    #[test]
    fn test_verify_tampered_data() {
        let keypair = KeyPair::generate();
        let data = b"test package data";
        let signature = sign_data(data, &keypair);
        assert!(verify_signature(b"tampered data", &signature, &keypair.public_key()).is_err());
    }

    #[test]
    fn test_keypair_serialization() {
        let keypair = KeyPair::generate();
        let exported = keypair.to_bytes();
        let imported = KeyPair::from_bytes(&exported).unwrap();
        assert_eq!(
            keypair.public_key().as_bytes(),
            imported.public_key().as_bytes()
        );
    }
}
