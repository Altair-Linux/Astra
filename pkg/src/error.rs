use thiserror::Error;

/// things that can go wrong with package operations.
#[derive(Debug, Error)]
pub enum PackageError {
    #[error("invalid package format: {0}")]
    InvalidFormat(String),

    #[error("missing metadata")]
    MissingMetadata,

    #[error("missing signature")]
    MissingSignature,

    #[error("checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("invalid metadata: {0}")]
    InvalidMetadata(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("crypto error: {0}")]
    Crypto(#[from] astra_crypto::CryptoError),
}
