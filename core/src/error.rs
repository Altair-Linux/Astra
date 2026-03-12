use thiserror::Error;

/// Errors that can occur in the Astra package manager.
#[derive(Debug, Error)]
pub enum AstraError {
    #[error("package error: {0}")]
    Package(#[from] astra_pkg::PackageError),

    #[error("database error: {0}")]
    Database(#[from] astra_db::DbError),

    #[error("resolver error: {0}")]
    Resolver(#[from] astra_resolver::ResolverError),

    #[error("repository error: {0}")]
    Repository(#[from] astra_repo::RepoError),

    #[error("crypto error: {0}")]
    Crypto(#[from] astra_crypto::CryptoError),

    #[error("build error: {0}")]
    Build(#[from] astra_builder::BuildError),

    #[error("not initialized: run 'astra init' first")]
    NotInitialized,

    #[error("operation cancelled")]
    Cancelled,

    #[error("{0}")]
    Other(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
