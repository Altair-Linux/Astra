use crate::{RepoConfig, RepoError, RepoIndex};
use astra_crypto::sha256_hex;
use std::path::Path;

/// Client for interacting with Astra repositories.
pub struct RepoClient {
    http: reqwest::Client,
}

impl RepoClient {
    /// Create a new repository client.
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }

    /// Fetch the repository index.
    pub async fn fetch_index(&self, repo: &RepoConfig) -> Result<RepoIndex, RepoError> {
        let url = repo
            .url
            .join("index.json")
            .map_err(|e| RepoError::InvalidIndex(e.to_string()))?;

        let response = self
            .http
            .get(url.as_str())
            .send()
            .await?
            .error_for_status()
            .map_err(|e| RepoError::DownloadFailed(e.to_string()))?;

        let index: RepoIndex = response.json().await?;
        Ok(index)
    }

    /// Download a package file to the specified path.
    pub async fn download_package(
        &self,
        repo: &RepoConfig,
        filename: &str,
        expected_checksum: &str,
        dest: &Path,
    ) -> Result<(), RepoError> {
        let url = repo
            .url
            .join(&format!("packages/{filename}"))
            .map_err(|e| RepoError::DownloadFailed(e.to_string()))?;

        let response = self
            .http
            .get(url.as_str())
            .send()
            .await?
            .error_for_status()
            .map_err(|e| RepoError::DownloadFailed(e.to_string()))?;

        let bytes = response.bytes().await?;

        // Verify checksum before writing to disk
        let actual_checksum = sha256_hex(&bytes);
        if actual_checksum != expected_checksum {
            return Err(RepoError::ChecksumMismatch {
                package: filename.to_string(),
                expected: expected_checksum.to_string(),
                actual: actual_checksum,
            });
        }

        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(dest, &bytes)?;

        Ok(())
    }

    /// Download a package's signature file.
    pub async fn download_signature(
        &self,
        repo: &RepoConfig,
        filename: &str,
        dest: &Path,
    ) -> Result<(), RepoError> {
        let sig_filename = format!("{filename}.sig");
        let url = repo
            .url
            .join(&format!("signatures/{sig_filename}"))
            .map_err(|e| RepoError::DownloadFailed(e.to_string()))?;

        let response = self
            .http
            .get(url.as_str())
            .send()
            .await?
            .error_for_status()
            .map_err(|e| RepoError::DownloadFailed(e.to_string()))?;

        let bytes = response.bytes().await?;
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(dest, &bytes)?;

        Ok(())
    }
}

impl Default for RepoClient {
    fn default() -> Self {
        Self::new()
    }
}
