use thiserror::Error;

/// things that can go wrong during repo operations.
#[derive(Debug, Error)]
pub enum RepoError {
    #[error("repository not found: {0}")]
    NotFound(String),

    #[error("package not found in repository: {0}")]
    PackageNotFound(String),

    #[error("download failed: {0}")]
    DownloadFailed(String),

    #[error("checksum mismatch for {package}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        package: String,
        expected: String,
        actual: String,
    },

    #[error("invalid index: {0}")]
    InvalidIndex(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("package error: {0}")]
    Package(#[from] astra_pkg::PackageError),

    #[error("crypto error: {0}")]
    Crypto(#[from] astra_crypto::CryptoError),
}
