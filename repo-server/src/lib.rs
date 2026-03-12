//! # Astra Repository Server
//!
//! A simple HTTP server for hosting Astra package repositories.
//! Serves the index, packages, and signatures as static files.

mod server;

pub use server::serve_repository;
