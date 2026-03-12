use crate::{AstraConfig, AstraError};
use astra_builder::Builder;
use astra_crypto::{KeyPair, KeyRing};
use astra_db::{Database, InstallReason};
use astra_pkg::{Package, PackageReader};
use astra_repo::{RepoClient, RepoConfig, RepoIndex, RepoPackageEntry};
use astra_resolver::{PackageCandidate, Resolver};
use semver::Version;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use url::Url;

/// The main package manager that orchestrates all operations.
pub struct PackageManager {
    config: AstraConfig,
    db: Database,
    keyring: KeyRing,
    repo_client: RepoClient,
    /// Cached repository indices.
    indices: HashMap<String, RepoIndex>,
}

impl PackageManager {
    /// Initialize a new Astra system at the given root.
    pub fn init(config: AstraConfig) -> Result<Self, AstraError> {
        std::fs::create_dir_all(&config.data_dir)?;
        std::fs::create_dir_all(&config.cache_dir)?;

        let db = Database::open(&config.db_path())?;
        let keyring = if config.keyring_path().exists() {
            KeyRing::load_from_file(&config.keyring_path())?
        } else {
            let kr = KeyRing::new();
            kr.save_to_file(&config.keyring_path())?;
            kr
        };

        config.save(&config.config_path())?;

        Ok(Self {
            config,
            db,
            keyring,
            repo_client: RepoClient::new(),
            indices: HashMap::new(),
        })
    }

    /// Open an existing Astra system.
    pub fn open(config: AstraConfig) -> Result<Self, AstraError> {
        if !config.data_dir.exists() {
            return Err(AstraError::NotInitialized);
        }

        let db = Database::open(&config.db_path())?;
        let keyring = if config.keyring_path().exists() {
            KeyRing::load_from_file(&config.keyring_path())?
        } else {
            KeyRing::new()
        };

        Ok(Self {
            config,
            db,
            keyring,
            repo_client: RepoClient::new(),
            indices: HashMap::new(),
        })
    }

    /// Get a reference to the config.
    pub fn config(&self) -> &AstraConfig {
        &self.config
    }

    /// Get a mutable reference to the config.
    pub fn config_mut(&mut self) -> &mut AstraConfig {
        &mut self.config
    }

    /// Get a reference to the database.
    pub fn db(&self) -> &Database {
        &self.db
    }

    /// Get a reference to the keyring.
    pub fn keyring(&self) -> &KeyRing {
        &self.keyring
    }

    // ─── Repository Management ─────────────────────────────────────

    /// Add a new repository.
    pub fn add_repo(&mut self, name: &str, url_str: &str) -> Result<(), AstraError> {
        let url = Url::parse(url_str)
            .map_err(|e| AstraError::Other(format!("invalid URL: {e}")))?;

        // Check for duplicates
        if self.config.repositories.iter().any(|r| r.name == name) {
            return Err(AstraError::Other(format!(
                "repository '{name}' already exists"
            )));
        }

        self.config.repositories.push(RepoConfig {
            name: name.to_string(),
            url,
            enabled: true,
        });

        self.config.save(&self.config.config_path())?;
        Ok(())
    }

    /// Remove a repository.
    pub fn remove_repo(&mut self, name: &str) -> Result<(), AstraError> {
        let before = self.config.repositories.len();
        self.config.repositories.retain(|r| r.name != name);
        if self.config.repositories.len() == before {
            return Err(AstraError::Other(format!(
                "repository '{name}' not found"
            )));
        }
        self.indices.remove(name);
        self.config.save(&self.config.config_path())?;
        Ok(())
    }

    /// Update (fetch) all repository indices.
    pub async fn update(&mut self) -> Result<Vec<String>, AstraError> {
        let mut updated = Vec::new();
        let repos: Vec<RepoConfig> = self.config.repositories.clone();
        for repo in &repos {
            if !repo.enabled {
                continue;
            }
            match self.repo_client.fetch_index(repo).await {
                Ok(index) => {
                    // Cache the index to disk
                    let cache_dir = self.config.repo_cache_dir(&repo.name);
                    std::fs::create_dir_all(&cache_dir)?;
                    let index_path = cache_dir.join("index.json");
                    let json = serde_json::to_string_pretty(&index)
                        .map_err(|e| AstraError::Other(e.to_string()))?;
                    std::fs::write(&index_path, json)?;

                    self.indices.insert(repo.name.clone(), index);
                    updated.push(repo.name.clone());
                }
                Err(e) => {
                    tracing::warn!("Failed to update repository '{}': {}", repo.name, e);
                }
            }
        }
        Ok(updated)
    }

    /// Load cached indices from disk.
    pub fn load_cached_indices(&mut self) -> Result<(), AstraError> {
        for repo in &self.config.repositories {
            let cache_dir = self.config.repo_cache_dir(&repo.name);
            let index_path = cache_dir.join("index.json");
            if index_path.exists() {
                let json = std::fs::read_to_string(&index_path)?;
                let index: RepoIndex = serde_json::from_str(&json)
                    .map_err(|e| AstraError::Other(format!("invalid cached index: {e}")))?;
                self.indices.insert(repo.name.clone(), index);
            }
        }
        Ok(())
    }

    // ─── Package Queries ───────────────────────────────────────────

    /// Search for packages across all repositories.
    pub fn search(&self, query: &str) -> Vec<(&str, &RepoPackageEntry)> {
        let mut results = Vec::new();
        for (repo_name, index) in &self.indices {
            for entry in index.search(query) {
                results.push((repo_name.as_str(), entry));
            }
        }
        results
    }

    /// Get info about a package from repositories.
    pub fn info(&self, name: &str) -> Option<(&str, &RepoPackageEntry)> {
        for (repo_name, index) in &self.indices {
            if let Some(entry) = index.find_package(name) {
                return Some((repo_name.as_str(), entry));
            }
        }
        None
    }

    // ─── Installation ──────────────────────────────────────────────

    /// Install packages by name.
    pub async fn install(&mut self, names: &[String]) -> Result<Vec<String>, AstraError> {
        self.load_cached_indices()?;

        // Build resolver
        let mut resolver = Resolver::new();
        // Add installed packages
        for pkg in self.db.list_packages()? {
            resolver.add_installed(pkg.name.clone(), pkg.version.clone());
        }
        // Add available packages from indices
        for (_, index) in &self.indices {
            for entry in &index.packages {
                resolver.add_candidate(PackageCandidate {
                    name: entry.name.clone(),
                    version: entry.version.clone(),
                    dependencies: entry.dependencies.clone(),
                    optional_dependencies: vec![],
                    conflicts: entry.conflicts.clone(),
                    provides: entry.provides.clone(),
                });
            }
        }

        // Resolve
        let resolution = resolver.resolve(names)?;
        tracing::info!(
            "Resolved {} packages to install: {:?}",
            resolution.install_order.len(),
            resolution.install_order
        );

        let mut installed = Vec::new();
        for pkg_name in &resolution.install_order {
            // Find which repo has this package
            let (repo_config, entry) = self.find_package_in_repos(pkg_name)?;

            // Download
            let cache_dir = self.config.package_cache_dir();
            std::fs::create_dir_all(&cache_dir)?;
            let pkg_path = cache_dir.join(&entry.filename);

            self.repo_client
                .download_package(&repo_config, &entry.filename, &entry.checksum, &pkg_path)
                .await?;

            // Read & verify
            let package = PackageReader::read_from_file(&pkg_path)?;

            // Verify signature against any key in keyring
            let mut verified = false;
            for (_, key) in self.keyring.all_keys() {
                if package.verify(key).is_ok() {
                    verified = true;
                    break;
                }
            }
            if !verified {
                return Err(AstraError::Other(format!(
                    "signature verification failed for '{pkg_name}': no trusted key"
                )));
            }

            // Install files
            let file_paths = self.extract_files(&package)?;

            // Record in database
            let reason = if names.contains(pkg_name) {
                InstallReason::Explicit
            } else {
                InstallReason::Dependency
            };
            self.db
                .install_package(&package.metadata, &file_paths, reason)?;

            installed.push(pkg_name.clone());
        }

        Ok(installed)
    }

    /// Install a local package file.
    pub fn install_local(
        &mut self,
        path: &Path,
        skip_verify: bool,
    ) -> Result<String, AstraError> {
        let package = PackageReader::read_from_file(path)?;

        if !skip_verify {
            let mut verified = false;
            for (_, key) in self.keyring.all_keys() {
                if package.verify(key).is_ok() {
                    verified = true;
                    break;
                }
            }
            if !verified {
                return Err(AstraError::Other(format!(
                    "signature verification failed for '{}': no trusted key",
                    package.metadata.name
                )));
            }
        }

        let file_paths = self.extract_files(&package)?;
        self.db
            .install_package(&package.metadata, &file_paths, InstallReason::Explicit)?;

        Ok(package.metadata.name.clone())
    }

    /// Extract package files to the root filesystem.
    fn extract_files(&self, package: &Package) -> Result<Vec<PathBuf>, AstraError> {
        let mut paths = Vec::new();
        for (rel_path, content) in &package.files {
            let dest = self.config.root.join(rel_path);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&dest, content)?;
            paths.push(rel_path.clone());
        }
        Ok(paths)
    }

    // ─── Removal ───────────────────────────────────────────────────

    /// Remove a package.
    pub fn remove(&mut self, name: &str) -> Result<Vec<PathBuf>, AstraError> {
        if !self.db.is_installed(name)? {
            return Err(AstraError::Other(format!("package '{name}' is not installed")));
        }

        // Check reverse dependencies
        let rdeps = self.db.get_reverse_dependencies(name)?;
        if !rdeps.is_empty() {
            return Err(AstraError::Other(format!(
                "cannot remove '{name}': required by {}",
                rdeps.join(", ")
            )));
        }

        let files = self.db.remove_package(name)?;

        // Remove files from filesystem
        for file_path in &files {
            let full_path = self.config.root.join(file_path);
            if full_path.exists() {
                std::fs::remove_file(&full_path).ok();
            }
        }

        // Clean up empty directories
        for file_path in &files {
            let full_path = self.config.root.join(file_path);
            if let Some(parent) = full_path.parent() {
                Self::remove_empty_dirs(parent, &self.config.root);
            }
        }

        Ok(files)
    }

    fn remove_empty_dirs(dir: &Path, root: &Path) {
        let mut current = dir.to_path_buf();
        while current != *root {
            if current.exists() && std::fs::read_dir(&current).map(|mut d| d.next().is_none()).unwrap_or(false) {
                std::fs::remove_dir(&current).ok();
            } else {
                break;
            }
            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => break,
            }
        }
    }

    // ─── Upgrade ───────────────────────────────────────────────────

    /// Check for available upgrades.
    pub fn check_upgrades(&self) -> Result<Vec<(String, Version, Version)>, AstraError> {
        let mut upgrades = Vec::new();
        let installed = self.db.list_packages()?;

        for pkg in &installed {
            for (_, index) in &self.indices {
                if let Some(entry) = index.find_package(&pkg.name) {
                    if entry.version > pkg.version {
                        upgrades.push((
                            pkg.name.clone(),
                            pkg.version.clone(),
                            entry.version.clone(),
                        ));
                    }
                }
            }
        }

        Ok(upgrades)
    }

    /// Upgrade all packages.
    pub async fn upgrade(&mut self) -> Result<Vec<String>, AstraError> {
        self.load_cached_indices()?;
        let upgrades = self.check_upgrades()?;
        let names: Vec<String> = upgrades.into_iter().map(|(name, _, _)| name).collect();
        if names.is_empty() {
            return Ok(vec![]);
        }
        self.install(&names).await
    }

    // ─── Verification ──────────────────────────────────────────────

    /// Verify an installed package's integrity.
    pub fn verify_installed(&self, name: &str) -> Result<Vec<String>, AstraError> {
        let pkg = self.db.get_package(name)?;
        let metadata = self.db.get_metadata(name)?;
        let mut issues = Vec::new();

        for file_path in &pkg.files {
            let full_path = self.config.root.join(file_path);
            if !full_path.exists() {
                issues.push(format!("missing: {}", file_path.display()));
                continue;
            }

            // Check checksum if available
            let key = file_path.to_string_lossy().to_string();
            if let Some(checksum) = metadata.checksums.get(&key) {
                let content = std::fs::read(&full_path)?;
                let actual = astra_crypto::sha256_hex(&content);
                if actual != checksum.sha256 {
                    issues.push(format!("modified: {}", file_path.display()));
                }
            }
        }

        Ok(issues)
    }

    // ─── Key Management ────────────────────────────────────────────

    /// Import a public key into the keyring.
    pub fn import_key(&mut self, name: &str, key: astra_crypto::PublicKey) -> Result<(), AstraError> {
        self.keyring.add(name.to_string(), key);
        self.keyring.save_to_file(&self.config.keyring_path())?;
        Ok(())
    }

    /// Export the signing key's public key.
    pub fn export_public_key(&self) -> Result<astra_crypto::PublicKey, AstraError> {
        let key_path = self.config.signing_key_path();
        if !key_path.exists() {
            return Err(AstraError::Other("no signing key found".into()));
        }
        let keypair = KeyPair::load_from_file(&key_path)?;
        Ok(keypair.public_key())
    }

    /// Generate a new signing key pair.
    pub fn generate_keypair(&self) -> Result<KeyPair, AstraError> {
        let keypair = KeyPair::generate();
        keypair.save_to_file(&self.config.signing_key_path())?;
        Ok(keypair)
    }

    /// Load the signing key pair.
    pub fn load_keypair(&self) -> Result<KeyPair, AstraError> {
        let key_path = self.config.signing_key_path();
        if !key_path.exists() {
            return Err(AstraError::Other(
                "no signing key found; generate one with 'astra key generate'".into(),
            ));
        }
        Ok(KeyPair::load_from_file(&key_path)?)
    }

    // ─── Building ──────────────────────────────────────────────────

    /// Build a package from a directory.
    pub fn build(&self, pkg_dir: &Path, output_dir: &Path) -> Result<PathBuf, AstraError> {
        let keypair = self.load_keypair()?;
        Ok(Builder::build(pkg_dir, &keypair, output_dir)?)
    }

    // ─── Helpers ───────────────────────────────────────────────────

    fn find_package_in_repos(
        &self,
        name: &str,
    ) -> Result<(RepoConfig, RepoPackageEntry), AstraError> {
        for repo in &self.config.repositories {
            if let Some(index) = self.indices.get(&repo.name) {
                if let Some(entry) = index.find_package(name) {
                    return Ok((repo.clone(), entry.clone()));
                }
            }
        }
        Err(AstraError::Repository(
            astra_repo::RepoError::PackageNotFound(name.to_string()),
        ))
    }

    /// Save current config.
    pub fn save_config(&self) -> Result<(), AstraError> {
        self.config.save(&self.config.config_path())?;
        Ok(())
    }
}
