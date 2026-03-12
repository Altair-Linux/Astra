//! # astra dependency resolver
//!
//! deterministic dependency resolution with conflict detection,
//! cycle detection, and optional dependency support.
//! built to be replaceable with a sat solver later.

mod error;
mod resolver;

pub use error::ResolverError;
pub use resolver::{PackageCandidate, ResolutionResult, Resolver};
