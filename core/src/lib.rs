//! # astra core
//!
//! the main package management logic that ties everything together:
//! database, resolver, repo client, crypto, and builder.

mod config;
mod error;
mod manager;

pub use config::AstraConfig;
pub use error::AstraError;
pub use manager::PackageManager;
