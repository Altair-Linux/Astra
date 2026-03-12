use chrono::{DateTime, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Package metadata stored inside every `.astpkg` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// Package name (e.g., "coreutils").
    pub name: String,
    /// Semantic version.
    pub version: Version,
    /// Target architecture (e.g., "x86_64", "aarch64", "any").
    pub architecture: String,
    /// Human-readable description.
    pub description: String,
    /// Required dependencies.
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    /// Optional dependencies.
    #[serde(default)]
    pub optional_dependencies: Vec<Dependency>,
    /// Conflicting packages.
    #[serde(default)]
    pub conflicts: Vec<String>,
    /// Virtual packages this package provides.
    #[serde(default)]
    pub provides: Vec<String>,
    /// Package maintainer.
    pub maintainer: String,
    /// License identifier.
    pub license: String,
    /// Build timestamp.
    pub build_date: DateTime<Utc>,
    /// Checksums of all included files.
    #[serde(default)]
    pub checksums: HashMap<String, Checksum>,
    /// Package size in bytes (installed).
    #[serde(default)]
    pub installed_size: u64,
}

impl Metadata {
    /// Validate that required fields are present and correct.
    pub fn validate(&self) -> Result<(), crate::PackageError> {
        if self.name.is_empty() {
            return Err(crate::PackageError::InvalidMetadata(
                "name must not be empty".into(),
            ));
        }
        if self.name.contains(char::is_whitespace) {
            return Err(crate::PackageError::InvalidMetadata(
                "name must not contain whitespace".into(),
            ));
        }
        if self.architecture.is_empty() {
            return Err(crate::PackageError::InvalidMetadata(
                "architecture must not be empty".into(),
            ));
        }
        if self.description.is_empty() {
            return Err(crate::PackageError::InvalidMetadata(
                "description must not be empty".into(),
            ));
        }
        if self.maintainer.is_empty() {
            return Err(crate::PackageError::InvalidMetadata(
                "maintainer must not be empty".into(),
            ));
        }
        if self.license.is_empty() {
            return Err(crate::PackageError::InvalidMetadata(
                "license must not be empty".into(),
            ));
        }
        Ok(())
    }

    /// Full package identifier: "name-version".
    pub fn full_name(&self) -> String {
        format!("{}-{}", self.name, self.version)
    }

    /// Package filename: "name-version-arch.astpkg".
    pub fn filename(&self) -> String {
        format!("{}-{}-{}.astpkg", self.name, self.version, self.architecture)
    }
}

/// A package dependency with optional version constraint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dependency {
    /// Package name.
    pub name: String,
    /// Version requirement string (semver range).
    #[serde(default)]
    pub version_req: Option<String>,
}

impl Dependency {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version_req: None,
        }
    }

    pub fn with_version(name: impl Into<String>, req: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version_req: Some(req.into()),
        }
    }
}

impl std::fmt::Display for Dependency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.version_req {
            Some(req) => write!(f, "{} {}", self.name, req),
            None => write!(f, "{}", self.name),
        }
    }
}

/// A checksum entry for a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checksum {
    /// SHA-256 hash as hex string.
    pub sha256: String,
    /// File size in bytes.
    pub size: u64,
}

/// Script types that can be included in a package.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ScriptType {
    PreInstall,
    PostInstall,
    PreRemove,
    PostRemove,
    PreUpgrade,
    PostUpgrade,
}

impl ScriptType {
    /// Get the filename for this script type.
    pub fn filename(&self) -> &'static str {
        match self {
            ScriptType::PreInstall => "pre_install.sh",
            ScriptType::PostInstall => "post_install.sh",
            ScriptType::PreRemove => "pre_remove.sh",
            ScriptType::PostRemove => "post_remove.sh",
            ScriptType::PreUpgrade => "pre_upgrade.sh",
            ScriptType::PostUpgrade => "post_upgrade.sh",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_validate() {
        let meta = Metadata {
            name: "test-pkg".into(),
            version: Version::new(1, 0, 0),
            architecture: "x86_64".into(),
            description: "A test package".into(),
            dependencies: vec![],
            optional_dependencies: vec![],
            conflicts: vec![],
            provides: vec![],
            maintainer: "Test User <test@example.com>".into(),
            license: "ZPL-2.0".into(),
            build_date: Utc::now(),
            checksums: HashMap::new(),
            installed_size: 0,
        };
        assert!(meta.validate().is_ok());
    }

    #[test]
    fn test_metadata_validate_empty_name() {
        let meta = Metadata {
            name: "".into(),
            version: Version::new(1, 0, 0),
            architecture: "x86_64".into(),
            description: "A test package".into(),
            dependencies: vec![],
            optional_dependencies: vec![],
            conflicts: vec![],
            provides: vec![],
            maintainer: "Test User <test@example.com>".into(),
            license: "ZPL-2.0".into(),
            build_date: Utc::now(),
            checksums: HashMap::new(),
            installed_size: 0,
        };
        assert!(meta.validate().is_err());
    }

    #[test]
    fn test_dependency_display() {
        let dep = Dependency::with_version("glibc", ">=2.35");
        assert_eq!(dep.to_string(), "glibc >=2.35");

        let dep = Dependency::new("bash");
        assert_eq!(dep.to_string(), "bash");
    }
}
