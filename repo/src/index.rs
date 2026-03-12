use astra_pkg::Dependency;
use semver::Version;
use serde::{Deserialize, Serialize};
use url::Url;

/// Configuration for a repository source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    /// Human-readable name.
    pub name: String,
    /// Base URL of the repository.
    pub url: Url,
    /// Whether this repository is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Repository package index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoIndex {
    /// Repository name.
    pub name: String,
    /// Repository description.
    #[serde(default)]
    pub description: String,
    /// Last update timestamp.
    #[serde(default)]
    pub last_updated: String,
    /// Available packages.
    pub packages: Vec<RepoPackageEntry>,
}

impl RepoIndex {
    /// Find a package entry by name.
    pub fn find_package(&self, name: &str) -> Option<&RepoPackageEntry> {
        self.packages.iter().find(|p| p.name == name)
    }

    /// Find all versions of a package.
    pub fn find_all_versions(&self, name: &str) -> Vec<&RepoPackageEntry> {
        self.packages.iter().filter(|p| p.name == name).collect()
    }

    /// Search packages by name or description.
    pub fn search(&self, query: &str) -> Vec<&RepoPackageEntry> {
        let query_lower = query.to_lowercase();
        self.packages
            .iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&query_lower)
                    || p.description.to_lowercase().contains(&query_lower)
            })
            .collect()
    }
}

/// An entry in the repository index for a single package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoPackageEntry {
    /// Package name.
    pub name: String,
    /// Package version.
    pub version: Version,
    /// Target architecture.
    pub architecture: String,
    /// Description.
    pub description: String,
    /// Dependencies.
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    /// Conflicts.
    #[serde(default)]
    pub conflicts: Vec<String>,
    /// Provides.
    #[serde(default)]
    pub provides: Vec<String>,
    /// SHA-256 checksum of the package file.
    pub checksum: String,
    /// Download filename (relative to packages/).
    pub filename: String,
    /// Package size in bytes.
    pub size: u64,
    /// License.
    #[serde(default)]
    pub license: String,
    /// Maintainer.
    #[serde(default)]
    pub maintainer: String,
}
