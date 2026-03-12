//! # Astra Database
//!
//! Local package database backed by SQLite.
//! Tracks installed packages, their files, dependencies, and timestamps.
//! Provides atomic writes and crash recovery via SQLite transactions.

mod error;
mod database;

pub use error::DbError;
pub use database::{Database, InstalledPackage, InstallReason};
