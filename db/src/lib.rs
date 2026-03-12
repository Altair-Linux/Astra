//! # astra database
//!
//! local package database backed by sqlite.
//! keeps track of installed packages, their files, deps, and timestamps.
//! uses sqlite transactions for atomic writes and crash safety.

mod database;
mod error;

pub use database::{Database, InstallReason, InstalledPackage};
pub use error::DbError;
