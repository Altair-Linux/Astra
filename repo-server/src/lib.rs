//! # astra repository server
//!
//! a simple http server for hosting astra package repos.
//! serves the index, packages, and signatures as static files.

mod server;

pub use server::serve_repository;
