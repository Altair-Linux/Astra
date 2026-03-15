use astra_pkg::Dependency;
use astra_pkg::PackageReader;
use chrono::Utc;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::Path;
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

/// scans a repository directory and builds index metadata.
pub fn generate_repo_index(
    repo_root: &Path,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<RepoIndex, crate::RepoError> {
    let packages_dir = repo_root.join("packages");
    std::fs::create_dir_all(&packages_dir)?;

    let mut packages = Vec::new();

    for entry in std::fs::read_dir(&packages_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || path.extension().map(|ext| ext != "astpkg").unwrap_or(true) {
            continue;
        }

        let metadata = PackageReader::read_metadata(&path)?;
        let checksum = PackageReader::file_checksum(&path)?;
        let size = std::fs::metadata(&path)?.len();
        let filename = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();

        packages.push(RepoPackageEntry {
            name: metadata.name,
            version: metadata.version,
            architecture: metadata.architecture,
            description: metadata.description,
            dependencies: metadata.dependencies,
            conflicts: metadata.conflicts,
            provides: metadata.provides,
            checksum,
            filename,
            size,
            license: metadata.license,
            maintainer: metadata.maintainer,
        });
    }

    packages.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| b.version.cmp(&a.version))
            .then_with(|| a.architecture.cmp(&b.architecture))
    });

    let default_name = repo_root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "altair-repo".to_string());

    Ok(RepoIndex {
        name: name.unwrap_or(&default_name).to_string(),
        description: description
            .unwrap_or("Altair Linux package repository")
            .to_string(),
        last_updated: Utc::now().to_rfc3339(),
        packages,
    })
}
