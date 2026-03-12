//! # Astra Repository Client
//!
//! Handles communication with remote Astra package repositories.
//! Downloads package indices, fetches packages, and verifies checksums.

mod client;
mod error;
mod index;

pub use client::RepoClient;
pub use error::RepoError;
pub use index::{RepoConfig, RepoIndex, RepoPackageEntry};
