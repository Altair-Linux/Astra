//! # Astra Core
//!
//! Core package management logic that orchestrates all subsystems:
//! database, resolver, repository client, crypto, and builder.

mod config;
mod error;
mod manager;

pub use config::AstraConfig;
pub use error::AstraError;
pub use manager::PackageManager;
