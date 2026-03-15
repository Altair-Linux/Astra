//! # astra repository client
//!
//! talks to remote astra package repositories.
//! downloads package indices, fetches packages, and verifies checksums.

mod client;
mod error;
mod index;

pub use client::RepoClient;
pub use error::RepoError;
pub use index::{generate_repo_index, RepoConfig, RepoIndex, RepoPackageEntry};
