use astra_pkg::Dependency;
use semver::Version;
use serde::{Deserialize, Serialize};
use url::Url;

/// config for a repository source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    /// human-readable name.
    pub name: String,
    /// base url of the repository.
    pub url: Url,
    /// whether this repo is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// the package index for a repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoIndex {
    /// repo name.
    pub name: String,
    /// repo description.
    #[serde(default)]
    pub description: String,
    /// when the index was last updated.
    #[serde(default)]
    pub last_updated: String,
    /// packages available in this repo.
    pub packages: Vec<RepoPackageEntry>,
}

impl RepoIndex {
    /// finds a package entry by name.
    pub fn find_package(&self, name: &str) -> Option<&RepoPackageEntry> {
        self.packages.iter().find(|p| p.name == name)
    }

    /// finds all versions of a package.
    pub fn find_all_versions(&self, name: &str) -> Vec<&RepoPackageEntry> {
        self.packages.iter().filter(|p| p.name == name).collect()
    }

    /// searches packages by name or description.
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

/// a single package entry in the repo index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoPackageEntry {
    /// package name.
    pub name: String,
    /// package version.
    pub version: Version,
    /// target architecture.
    pub architecture: String,
    /// description.
    pub description: String,
    /// dependencies.
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    /// conflicts.
    #[serde(default)]
    pub conflicts: Vec<String>,
    /// provides.
    #[serde(default)]
    pub provides: Vec<String>,
    /// sha-256 checksum of the package file.
    pub checksum: String,
    /// download filename (relative to packages/).
    pub filename: String,
    /// package size in bytes.
    pub size: u64,
    /// license.
    #[serde(default)]
    pub license: String,
    /// maintainer.
    #[serde(default)]
    pub maintainer: String,
}
