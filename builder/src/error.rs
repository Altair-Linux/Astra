use thiserror::Error;

/// things that can go wrong during package building.
#[derive(Debug, Error)]
pub enum BuildError {
    #[error("recipe not found: {0}")]
    RecipeNotFound(String),

    #[error("invalid recipe: {0}")]
    InvalidRecipe(String),

    #[error("build failed: {0}")]
    BuildFailed(String),

    #[error("no files found in package directory")]
    NoFiles,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("package error: {0}")]
    Package(#[from] astra_pkg::PackageError),

    #[error("crypto error: {0}")]
    Crypto(#[from] astra_crypto::CryptoError),
}
