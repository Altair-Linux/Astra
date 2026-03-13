use astra_repo::RepoConfig;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// global astra configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstraConfig {
    /// root directory (typically /).
    #[serde(default = "default_root")]
    pub root: PathBuf,
    /// data directory for astra state.
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,
    /// cache directory for downloads.
    #[serde(default = "default_cache_dir")]
    pub cache_dir: PathBuf,
    /// configured repositories.
    #[serde(default)]
    pub repositories: Vec<RepoConfig>,
}

fn default_root() -> PathBuf {
    PathBuf::from("/")
}

fn default_data_dir() -> PathBuf {
    PathBuf::from("/var/lib/astra")
}

fn default_cache_dir() -> PathBuf {
    PathBuf::from("/var/cache/astra")
}

impl AstraConfig {
    /// loads config from a file.
    pub fn load(path: &Path) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        Ok(config)
    }

    /// saves config to a file.
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content =
            serde_json::to_string_pretty(self).map_err(|e| std::io::Error::other(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// path to the database file.
    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("astra.db")
    }

    /// path to the keyring file.
    pub fn keyring_path(&self) -> PathBuf {
        self.data_dir.join("keyring.json")
    }

    /// path to the signing key.
    pub fn signing_key_path(&self) -> PathBuf {
        self.data_dir.join("signing.key")
    }

    /// path to the config file.
    pub fn config_path(&self) -> PathBuf {
        self.data_dir.join("config.json")
    }

    /// cache path for a specific repo.
    pub fn repo_cache_dir(&self, repo_name: &str) -> PathBuf {
        self.cache_dir.join("repos").join(repo_name)
    }

    /// where downloaded packages are cached.
    pub fn package_cache_dir(&self) -> PathBuf {
        self.cache_dir.join("packages")
    }
}

impl Default for AstraConfig {
    fn default() -> Self {
        Self {
            root: default_root(),
            data_dir: default_data_dir(),
            cache_dir: default_cache_dir(),
            repositories: Vec::new(),
        }
    }
}
