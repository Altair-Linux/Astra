use thiserror::Error;

/// things that can go wrong during dependency resolution.
#[derive(Debug, Error)]
pub enum ResolverError {
    #[error("package not found: {0}")]
    PackageNotFound(String),

    #[error("no satisfying version for {package}: requires {requirement}")]
    NoSatisfyingVersion {
        package: String,
        requirement: String,
    },

    #[error("conflict: {package_a} conflicts with {package_b}")]
    Conflict {
        package_a: String,
        package_b: String,
    },

    #[error("circular dependency detected: {}", cycle.join(" -> "))]
    CircularDependency { cycle: Vec<String> },

    #[error("resolution failed: {0}")]
    ResolutionFailed(String),
}
