use chrono::{DateTime, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// metadata stored inside every `.astpkg` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// package name (e.g., "coreutils").
    pub name: String,
    /// semver version.
    pub version: Version,
    /// target architecture (e.g., "x86_64", "aarch64", "any").
    pub architecture: String,
    /// short description of what this package does.
    pub description: String,
    /// packages this one needs to work.
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    /// nice-to-have deps that aren't required.
    #[serde(default)]
    pub optional_dependencies: Vec<Dependency>,
    /// packages that can't be installed alongside this one.
    #[serde(default)]
    pub conflicts: Vec<String>,
    /// virtual packages this one provides.
    #[serde(default)]
    pub provides: Vec<String>,
    /// who maintains this package.
    pub maintainer: String,
    /// license identifier.
    pub license: String,
    /// when the package was built.
    pub build_date: DateTime<Utc>,
    /// sha-256 checksums for every file in the package.
    #[serde(default)]
    pub checksums: BTreeMap<String, Checksum>,
    /// total size when installed (bytes).
    #[serde(default)]
    pub installed_size: u64,
}

impl Metadata {
    /// makes sure all required fields are present and look right.
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

    /// returns "name-version" as a single string.
    pub fn full_name(&self) -> String {
        format!("{}-{}", self.name, self.version)
    }

    /// returns the expected filename: "name-version-arch.astpkg".
    pub fn filename(&self) -> String {
        format!(
            "{}-{}-{}.astpkg",
            self.name, self.version, self.architecture
        )
    }
}

/// a package dependency with an optional version constraint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dependency {
    /// package name.
    pub name: String,
    /// version requirement string (semver range).
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

/// checksum info for a single file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checksum {
    /// sha-256 hash as hex string.
    pub sha256: String,
    /// file size in bytes.
    pub size: u64,
}

/// script types that can be included in a package.
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
    /// returns the filename for this script type.
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
            checksums: BTreeMap::new(),
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
            checksums: BTreeMap::new(),
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
