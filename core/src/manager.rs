use crate::{AstraConfig, AstraError};
use astra_builder::Builder;
use astra_crypto::{sha256_hex, KeyPair, KeyRing};
use astra_db::{Database, InstallReason};
use astra_pkg::{Package, PackageReader, ScriptType};
use astra_repo::{RepoClient, RepoConfig, RepoIndex, RepoPackageEntry};
use astra_resolver::{PackageCandidate, Resolver};
use serde::{Deserialize, Serialize};
use semver::Version;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Component;
#[cfg(unix)]
use std::process::{Command, Stdio};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use url::Url;

/// the main package manager that coordinates everything.
pub struct PackageManager {
    config: AstraConfig,
    db: Database,
    keyring: KeyRing,
    repo_client: RepoClient,
    /// cached repo indices.
    indices: HashMap<String, RepoIndex>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum TxOperation {
    Install,
    Remove,
    Upgrade,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TxJournal {
    id: String,
    operation: TxOperation,
    package: String,
    created_files: Vec<PathBuf>,
    removed_files: Vec<PathBuf>,
}

impl PackageManager {
    /// sets up a fresh astra system at the given root.
    pub fn init(config: AstraConfig) -> Result<Self, AstraError> {
        std::fs::create_dir_all(&config.data_dir)?;
        std::fs::create_dir_all(&config.cache_dir)?;
        std::fs::create_dir_all(config.transactions_dir())?;
        std::fs::create_dir_all(config.trusted_keys_dir())?;

        let db = Database::open(&config.db_path())?;
        let keyring = if config.keyring_path().exists() {
            KeyRing::load_from_file(&config.keyring_path())?
        } else {
            let kr = KeyRing::new();
            kr.save_to_file(&config.keyring_path())?;
            kr
        };

        config.save(&config.config_path())?;

        let manager = Self {
            config,
            db,
            keyring,
            repo_client: RepoClient::new(),
            indices: HashMap::new(),
        };

        manager.recover_transactions()?;
        Ok(manager)
    }

    /// opens an existing astra system.
    pub fn open(config: AstraConfig) -> Result<Self, AstraError> {
        if !config.data_dir.exists() {
            return Err(AstraError::NotInitialized);
        }

        std::fs::create_dir_all(config.transactions_dir())?;
        std::fs::create_dir_all(config.trusted_keys_dir())?;

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

    /// returns a reference to the config.
    pub fn config(&self) -> &AstraConfig {
        &self.config
    }

    /// returns a mutable reference to the config.
    pub fn config_mut(&mut self) -> &mut AstraConfig {
        &mut self.config
    }

    /// returns a reference to the database.
    pub fn db(&self) -> &Database {
        &self.db
    }

    /// returns a reference to the keyring.
    pub fn keyring(&self) -> &KeyRing {
        &self.keyring
    }

    // ─── repository management ─────────────────────────────────────

    /// adds a new repository.
    pub fn add_repo(&mut self, name: &str, url_str: &str) -> Result<(), AstraError> {
        let url =
            Url::parse(url_str).map_err(|e| AstraError::Other(format!("invalid URL: {e}")))?;

        // check for duplicates
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

    /// removes a repository.
    pub fn remove_repo(&mut self, name: &str) -> Result<(), AstraError> {
        let before = self.config.repositories.len();
        self.config.repositories.retain(|r| r.name != name);
        if self.config.repositories.len() == before {
            return Err(AstraError::Other(format!("repository '{name}' not found")));
        }
        self.indices.remove(name);
        self.config.save(&self.config.config_path())?;
        Ok(())
    }

    /// fetches all repo indices from remote.
    pub async fn update(&mut self) -> Result<Vec<String>, AstraError> {
        let mut updated = Vec::new();
        let repos: Vec<RepoConfig> = self.config.repositories.clone();
        for repo in &repos {
            if !repo.enabled {
                continue;
            }
            match self.repo_client.fetch_index(repo).await {
                Ok(index) => {
                    // cache the index to disk
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

    /// loads cached indices from disk.
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

    // ─── package queries ───────────────────────────────────────────

    /// searches for packages across all repos.
    pub fn search(&self, query: &str) -> Vec<(&str, &RepoPackageEntry)> {
        let mut results = Vec::new();
        for (repo_name, index) in &self.indices {
            for entry in index.search(query) {
                results.push((repo_name.as_str(), entry));
            }
        }
        results
    }

    /// gets info about a package from repos.
    pub fn info(&self, name: &str) -> Option<(&str, &RepoPackageEntry)> {
        for (repo_name, index) in &self.indices {
            if let Some(entry) = index.find_package(name) {
                return Some((repo_name.as_str(), entry));
            }
        }
        None
    }

    // ─── installation ──────────────────────────────────────────────

    /// installs packages by name from remote repos.
    pub async fn install(&mut self, names: &[String]) -> Result<Vec<String>, AstraError> {
        self.load_cached_indices()?;

        // build resolver
        let mut resolver = Resolver::new();
        // add installed packages
        for pkg in self.db.list_packages()? {
            resolver.add_installed(pkg.name.clone(), pkg.version.clone());
        }
        // add available packages from indices
        for index in self.indices.values() {
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

        // resolve
        let resolution = resolver.resolve(names)?;
        tracing::info!(
            "Resolved {} packages to install: {:?}",
            resolution.install_order.len(),
            resolution.install_order
        );

        let mut installed = Vec::new();
        for pkg_name in &resolution.install_order {
            // find which repo has this package
            let (repo_config, entry) = self.find_package_in_repos(pkg_name)?;

            // download
            let cache_dir = self.config.package_cache_dir();
            std::fs::create_dir_all(&cache_dir)?;
            let pkg_path = cache_dir.join(&entry.filename);

            self.repo_client
                .download_package(&repo_config, &entry.filename, &entry.checksum, &pkg_path)
                .await?;

            // read & verify
            let package = PackageReader::read_from_file(&pkg_path)?;
            self.verify_package_file_checksums(&package)?;

            // verify signature against any key in keyring
            self.verify_with_keyring(&package, pkg_name)?;

            let tx_id = self.begin_transaction(TxOperation::Install, &package.metadata.name)?;
            let result: Result<Vec<PathBuf>, AstraError> = (|| {
                self.run_script_if_present(&package, ScriptType::PreInstall)?;
                let file_paths = self.extract_files(&package)?;
                self.update_transaction_created_files(&tx_id, file_paths.clone())?;

                let reason = if names.contains(pkg_name) {
                    InstallReason::Explicit
                } else {
                    InstallReason::Dependency
                };
                self.db
                    .install_package(&package.metadata, &file_paths, reason)?;

                self.run_script_if_present(&package, ScriptType::PostInstall)?;
                Ok(file_paths)
            })();

            match result {
                Ok(_paths) => {
                    self.commit_transaction(&tx_id)?;
                }
                Err(err) => {
                    self.rollback_transaction(&tx_id)?;
                    return Err(err);
                }
            }

            installed.push(pkg_name.clone());
            self.log_event(&format!("installed package '{}'", pkg_name));
        }

        Ok(installed)
    }

    /// installs a local .astpkg file.
    pub fn install_local(&mut self, path: &Path, skip_verify: bool) -> Result<String, AstraError> {
        let package = PackageReader::read_from_file(path)?;
        self.verify_package_file_checksums(&package)?;

        if !skip_verify {
            self.verify_with_keyring(&package, &package.metadata.name)?;
        }

        let tx_id = self.begin_transaction(TxOperation::Install, &package.metadata.name)?;
        let result: Result<(), AstraError> = (|| {
            self.run_script_if_present(&package, ScriptType::PreInstall)?;
            let file_paths = self.extract_files(&package)?;
            self.update_transaction_created_files(&tx_id, file_paths.clone())?;
            self.db
                .install_package(&package.metadata, &file_paths, InstallReason::Explicit)?;
            self.run_script_if_present(&package, ScriptType::PostInstall)?;
            Ok(())
        })();

        match result {
            Ok(()) => self.commit_transaction(&tx_id)?,
            Err(err) => {
                self.rollback_transaction(&tx_id)?;
                return Err(err);
            }
        }

        self.log_event(&format!(
            "installed package '{}' from local file {}",
            package.metadata.name,
            path.display()
        ));

        Ok(package.metadata.name.clone())
    }

    /// extracts package files to the root filesystem.
    fn extract_files(&self, package: &Package) -> Result<Vec<PathBuf>, AstraError> {
        let mut paths = Vec::new();
        for (rel_path, content) in &package.files {
            Self::validate_relative_path(rel_path)?;
            let dest = self.safe_root_join(rel_path)?;
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&dest, content)?;
            paths.push(rel_path.clone());
        }
        Ok(paths)
    }

    // ─── removal ───────────────────────────────────────────────────

    /// removes a package and its files.
    pub fn remove(&mut self, name: &str) -> Result<Vec<PathBuf>, AstraError> {
        if !self.db.is_installed(name)? {
            return Err(AstraError::Other(format!(
                "package '{name}' is not installed"
            )));
        }

        // check reverse dependencies
        let rdeps = self.db.get_reverse_dependencies(name)?;
        if !rdeps.is_empty() {
            return Err(AstraError::Other(format!(
                "cannot remove '{name}': required by {}",
                rdeps.join(", ")
            )));
        }

        let files = self.db.remove_package(name)?;

        // remove files from filesystem
        for file_path in &files {
            let full_path = self.config.root.join(file_path);
            if full_path.exists() {
                std::fs::remove_file(&full_path).ok();
            }
        }

        // clean up empty directories
        for file_path in &files {
            let full_path = self.config.root.join(file_path);
            if let Some(parent) = full_path.parent() {
                Self::remove_empty_dirs(parent, &self.config.root);
            }
        }

        self.log_event(&format!("removed package '{}'", name));

        Ok(files)
    }

    fn remove_empty_dirs(dir: &Path, root: &Path) {
        let mut current = dir.to_path_buf();
        while current != *root {
            if current.exists()
                && std::fs::read_dir(&current)
                    .map(|mut d| d.next().is_none())
                    .unwrap_or(false)
            {
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

    // ─── upgrade ───────────────────────────────────────────────────

    /// checks what packages have newer versions available.
    pub fn check_upgrades(&self) -> Result<Vec<(String, Version, Version)>, AstraError> {
        let mut upgrades = Vec::new();
        let installed = self.db.list_packages()?;

        for pkg in &installed {
            for index in self.indices.values() {
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

    /// upgrades all packages that have newer versions.
    pub async fn upgrade(&mut self) -> Result<Vec<String>, AstraError> {
        self.load_cached_indices()?;
        let upgrades = self.check_upgrades()?;
        let names: Vec<String> = upgrades.into_iter().map(|(name, _, _)| name).collect();
        if names.is_empty() {
            return Ok(vec![]);
        }
        let upgraded = self.install(&names).await?;
        for name in &upgraded {
            self.log_event(&format!("upgraded package '{}'", name));
        }
        Ok(upgraded)
    }

    // ─── verification ──────────────────────────────────────────────

    /// verifies an installed package's file integrity.
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

            // check checksum if available
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

    // ─── key management ────────────────────────────────────────────

    /// imports a public key into the keyring.
    pub fn import_key(
        &mut self,
        name: &str,
        key: astra_crypto::PublicKey,
    ) -> Result<(), AstraError> {
        std::fs::create_dir_all(self.config.trusted_keys_dir())?;
        let key_file_path = self
            .config
            .trusted_keys_dir()
            .join(format!("{}.pub", name.replace('/', "_")));
        key.save_to_file(&key_file_path)?;

        self.keyring.add(name.to_string(), key);
        self.keyring.save_to_file(&self.config.keyring_path())?;
        self.log_event(&format!("added trusted key '{}'", name));
        Ok(())
    }

    /// removes a public key from the keyring.
    pub fn remove_key(&mut self, name: &str) -> Result<(), AstraError> {
        if self.keyring.remove(name).is_none() {
            return Err(AstraError::Other(format!("key '{}' not found", name)));
        }
        self.keyring.save_to_file(&self.config.keyring_path())?;

        let key_file_path = self
            .config
            .trusted_keys_dir()
            .join(format!("{}.pub", name.replace('/', "_")));
        if key_file_path.exists() {
            let _ = std::fs::remove_file(key_file_path);
        }

        self.log_event(&format!("removed trusted key '{}'", name));
        Ok(())
    }

    /// exports the signing key's public key.
    pub fn export_public_key(&self) -> Result<astra_crypto::PublicKey, AstraError> {
        let key_path = self.config.signing_key_path();
        if !key_path.exists() {
            return Err(AstraError::Other("no signing key found".into()));
        }
        let keypair = KeyPair::load_from_file(&key_path)?;
        Ok(keypair.public_key())
    }

    /// generates a new signing keypair.
    pub fn generate_keypair(&self) -> Result<KeyPair, AstraError> {
        let keypair = KeyPair::generate();
        keypair.save_to_file(&self.config.signing_key_path())?;
        Ok(keypair)
    }

    /// loads the signing keypair from disk.
    pub fn load_keypair(&self) -> Result<KeyPair, AstraError> {
        let key_path = self.config.signing_key_path();
        if !key_path.exists() {
            return Err(AstraError::Other(
                "no signing key found; generate one with 'astra key generate'".into(),
            ));
        }
        Ok(KeyPair::load_from_file(&key_path)?)
    }

    // ─── building ──────────────────────────────────────────────────

    /// builds a package from a directory.
    pub fn build(
        &self,
        pkg_dir: &Path,
        output_dir: &Path,
        sandbox: bool,
    ) -> Result<PathBuf, AstraError> {
        let keypair = self.load_keypair()?;
        Ok(Builder::build(pkg_dir, &keypair, output_dir, sandbox)?)
    }

    // ─── helpers ───────────────────────────────────────────────────

    fn verify_with_keyring(&self, package: &Package, pkg_name: &str) -> Result<(), AstraError> {
        let mut verified = false;
        for key in self.keyring.all_keys().values() {
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
        Ok(())
    }

    fn verify_package_file_checksums(&self, package: &Package) -> Result<(), AstraError> {
        for (path, content) in &package.files {
            let key = path.to_string_lossy().replace('\\', "/");
            if let Some(expected) = package.metadata.checksums.get(&key) {
                let actual = sha256_hex(content);
                if actual != expected.sha256 {
                    return Err(AstraError::Other(format!(
                        "checksum mismatch in package '{}' for file '{}'",
                        package.metadata.name, key
                    )));
                }
                if expected.size != content.len() as u64 {
                    return Err(AstraError::Other(format!(
                        "size mismatch in package '{}' for file '{}'",
                        package.metadata.name, key
                    )));
                }
            }
        }
        Ok(())
    }

    fn validate_relative_path(path: &Path) -> Result<(), AstraError> {
        if path.is_absolute() {
            return Err(AstraError::Other(format!(
                "invalid package path '{}': absolute paths are forbidden",
                path.display()
            )));
        }
        for component in path.components() {
            if matches!(component, Component::ParentDir) {
                return Err(AstraError::Other(format!(
                    "invalid package path '{}': parent traversal is forbidden",
                    path.display()
                )));
            }
        }
        Ok(())
    }

    fn safe_root_join(&self, rel_path: &Path) -> Result<PathBuf, AstraError> {
        let root = self
            .config
            .root
            .canonicalize()
            .unwrap_or_else(|_| self.config.root.clone());
        let dest = root.join(rel_path);
        if !dest.starts_with(&root) {
            return Err(AstraError::Other(format!(
                "refusing to write outside root: '{}'",
                rel_path.display()
            )));
        }
        Ok(dest)
    }

    fn begin_transaction(&self, operation: TxOperation, package: &str) -> Result<String, AstraError> {
        std::fs::create_dir_all(self.config.transactions_dir())?;
        let id = format!(
            "{}-{}",
            chrono::Utc::now().timestamp_millis(),
            package.replace('/', "_")
        );
        let journal = TxJournal {
            id: id.clone(),
            operation,
            package: package.to_string(),
            created_files: Vec::new(),
            removed_files: Vec::new(),
        };
        let tx_file = self.config.transactions_dir().join(format!("{}.json", &id));
        let json = serde_json::to_string_pretty(&journal)
            .map_err(|e| AstraError::Other(e.to_string()))?;
        std::fs::write(tx_file, json)?;
        Ok(id)
    }

    fn update_transaction_created_files(
        &self,
        tx_id: &str,
        created_files: Vec<PathBuf>,
    ) -> Result<(), AstraError> {
        let tx_file = self.config.transactions_dir().join(format!("{}.json", tx_id));
        let content = std::fs::read_to_string(&tx_file)?;
        let mut journal: TxJournal =
            serde_json::from_str(&content).map_err(|e| AstraError::Other(e.to_string()))?;
        journal.created_files = created_files;
        let json = serde_json::to_string_pretty(&journal)
            .map_err(|e| AstraError::Other(e.to_string()))?;
        std::fs::write(tx_file, json)?;
        Ok(())
    }

    fn commit_transaction(&self, tx_id: &str) -> Result<(), AstraError> {
        let tx_file = self.config.transactions_dir().join(format!("{}.json", tx_id));
        if tx_file.exists() {
            std::fs::remove_file(tx_file)?;
        }
        Ok(())
    }

    fn rollback_transaction(&self, tx_id: &str) -> Result<(), AstraError> {
        let tx_file = self.config.transactions_dir().join(format!("{}.json", tx_id));
        if !tx_file.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&tx_file)?;
        let journal: TxJournal =
            serde_json::from_str(&content).map_err(|e| AstraError::Other(e.to_string()))?;

        for rel_path in &journal.created_files {
            if let Ok(full_path) = self.safe_root_join(rel_path) {
                if full_path.exists() {
                    let _ = std::fs::remove_file(&full_path);
                }
                if let Some(parent) = full_path.parent() {
                    Self::remove_empty_dirs(parent, &self.config.root);
                }
            }
        }

        let _ = self.db.remove_package(&journal.package);
        let _ = std::fs::remove_file(&tx_file);
        self.log_event(&format!(
            "rolled back transaction {} for package '{}'",
            tx_id, journal.package
        ));
        Ok(())
    }

    fn recover_transactions(&self) -> Result<(), AstraError> {
        let tx_dir = self.config.transactions_dir();
        if !tx_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(tx_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() || path.extension().map(|x| x != "json").unwrap_or(true) {
                continue;
            }

            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                let _ = self.rollback_transaction(stem);
            }
        }
        Ok(())
    }

    fn run_script_if_present(&self, package: &Package, script: ScriptType) -> Result<(), AstraError> {
        let Some(script_content) = package.scripts.get(&script) else {
            return Ok(());
        };

        #[cfg(not(unix))]
        {
            let _ = script_content;
            tracing::warn!("Skipping lifecycle script on non-Unix host");
            return Ok(());
        }

        #[cfg(unix)]
        {
            let status = Command::new("sh")
                .arg("-c")
                .arg(script_content)
                .env("ASTRA_ROOT", &self.config.root)
                .stdin(Stdio::null())
                .status()?;
            if !status.success() {
                return Err(AstraError::Other(format!(
                    "lifecycle script '{}' failed for package '{}'",
                    script.filename(), package.metadata.name
                )));
            }
            Ok(())
        }
    }

    fn log_event(&self, message: &str) {
        let log_path = self.config.log_path();
        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
            let _ = writeln!(
                file,
                "{} {}",
                chrono::Utc::now().to_rfc3339(),
                message
            );
        }
    }

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

    /// saves the current config to disk.
    pub fn save_config(&self) -> Result<(), AstraError> {
        self.config.save(&self.config.config_path())?;
        Ok(())
    }
}
