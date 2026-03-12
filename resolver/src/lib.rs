//! # Astra Dependency Resolver
//!
//! Deterministic dependency resolution with conflict detection,
//! circular dependency detection, and optional dependency support.
//! Designed to allow future SAT solver replacement.

mod error;
mod resolver;

pub use error::ResolverError;
pub use resolver::{PackageCandidate, ResolutionResult, Resolver};
