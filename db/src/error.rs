use thiserror::Error;

/// Errors that can occur during database operations.
#[derive(Debug, Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("package not found: {0}")]
    PackageNotFound(String),

    #[error("package already installed: {0}")]
    AlreadyInstalled(String),

    #[error("database locked")]
    Locked,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
