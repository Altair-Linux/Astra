use thiserror::Error;

/// things that can go wrong during crypto operations.
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("signature verification failed")]
    VerificationFailed,

    #[error("invalid key format: {0}")]
    InvalidKey(String),

    #[error("invalid signature format: {0}")]
    InvalidSignature(String),

    #[error("key not found: {0}")]
    KeyNotFound(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
