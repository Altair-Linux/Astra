use crate::{Checksum, Metadata, PackageError, ScriptType};
use astra_crypto::{sha256_hex, PublicKey};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

const DETERMINISTIC_MTIME: u64 = 0;

/// in-memory representation of an astra package.
#[derive(Debug, Clone)]
pub struct Package {
    /// package metadata.
    pub metadata: Metadata,
    /// files included in the package (relative path -> content).
    pub files: HashMap<PathBuf, Vec<u8>>,
    /// install/remove scripts.
    pub scripts: HashMap<ScriptType, String>,
    /// ed25519 signature over the package content.
    pub signature: Option<Vec<u8>>,
}

impl Package {
    /// creates a new package with the given metadata.
    pub fn new(metadata: Metadata) -> Self {
        Self {
            metadata,
            files: HashMap::new(),
            scripts: HashMap::new(),
            signature: None,
        }
    }

    /// adds a file to the package.
    pub fn add_file(&mut self, path: impl Into<PathBuf>, content: Vec<u8>) {
        self.files.insert(path.into(), content);
    }

    /// adds an install script.
    pub fn add_script(&mut self, script_type: ScriptType, content: String) {
        self.scripts.insert(script_type, content);
    }

    /// computes sha-256 checksums for all files and updates metadata.
    pub fn compute_checksums(&mut self) {
        self.metadata.checksums.clear();
        let mut total_size = 0u64;
        for (path, content) in &self.files {
            let hash = sha256_hex(content);
            let size = content.len() as u64;
            total_size += size;
            self.metadata.checksums.insert(
                path.to_string_lossy().replace('\\', "/"),
                Checksum { sha256: hash, size },
            );
        }
        self.metadata.installed_size = total_size;
    }

    /// builds the signable content: a hash of metadata + file contents + scripts.
    pub fn signable_content(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();

        let metadata_bytes = serde_json::to_vec(&self.metadata).unwrap_or_default();
        hasher.update((metadata_bytes.len() as u64).to_le_bytes());
        hasher.update(&metadata_bytes);

        // hash all file contents in sorted order for consistency
        let mut paths: Vec<_> = self.files.keys().collect();
        paths.sort();
        for path in paths {
            let content = &self.files[path];
            let normalized = path.to_string_lossy().replace('\\', "/");
            hasher.update((normalized.len() as u64).to_le_bytes());
            hasher.update(normalized.as_bytes());
            hasher.update((content.len() as u64).to_le_bytes());
            hasher.update(content);
        }
        // hash scripts in sorted order
        let mut script_types: Vec<_> = self.scripts.keys().collect();
        script_types.sort_by_key(|s| s.filename());
        for st in script_types {
            let script_name = st.filename();
            let script_content = self.scripts[st].as_bytes();
            hasher.update((script_name.len() as u64).to_le_bytes());
            hasher.update(script_name.as_bytes());
            hasher.update((script_content.len() as u64).to_le_bytes());
            hasher.update(script_content);
        }
        hasher.finalize().to_vec()
    }

    /// signs this package with a keypair.
    pub fn sign(&mut self, keypair: &astra_crypto::KeyPair) {
        self.compute_checksums();
        let content = self.signable_content();
        self.signature = Some(astra_crypto::sign_data(&content, keypair));
    }

    /// verifies the package signature against a public key.
    pub fn verify(&self, public_key: &PublicKey) -> Result<(), PackageError> {
        let sig = self
            .signature
            .as_ref()
            .ok_or(PackageError::MissingSignature)?;
        let content = self.signable_content();
        astra_crypto::verify_signature(&content, sig, public_key)?;
        Ok(())
    }
}

/// writes a `Package` to the `.astpkg` format (tar + zstd).
pub struct PackageWriter;

impl PackageWriter {
    /// writes a package to a file.
    pub fn write_to_file(package: &Package, path: &Path) -> Result<(), PackageError> {
        let file = std::fs::File::create(path)?;
        Self::write(package, file)
    }

    /// writes a package to any writer.
    pub fn write<W: Write>(package: &Package, writer: W) -> Result<(), PackageError> {
        let encoder = zstd::Encoder::new(writer, 3)?;
        let encoder = encoder.auto_finish();
        let mut archive = tar::Builder::new(encoder);

        // write metadata.json
        let meta_bytes = serde_json::to_vec_pretty(&package.metadata)?;
        let mut header = tar::Header::new_gnu();
        header.set_path("metadata.json")?;
        header.set_size(meta_bytes.len() as u64);
        header.set_mode(0o644);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(DETERMINISTIC_MTIME);
        header.set_cksum();
        archive.append(&header, &meta_bytes[..])?;

        // write files/ entries in sorted order
        let mut paths: Vec<_> = package.files.keys().collect();
        paths.sort();
        for file_path in paths {
            let content = &package.files[file_path];
            // use forward slashes for tar compatibility
            let archive_path_str =
                format!("files/{}", file_path.to_string_lossy().replace('\\', "/"));
            let mut header = tar::Header::new_gnu();
            header.set_path(&archive_path_str)?;
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_uid(0);
            header.set_gid(0);
            header.set_mtime(DETERMINISTIC_MTIME);
            header.set_cksum();
            archive.append(&header, &content[..])?;
        }

        // write scripts/
        let mut script_types: Vec<_> = package.scripts.keys().collect();
        script_types.sort_by_key(|s| s.filename());
        for script_type in script_types {
            let content = &package.scripts[script_type];
            let archive_path = Path::new("scripts").join(script_type.filename());
            let mut header = tar::Header::new_gnu();
            header.set_path(&archive_path)?;
            header.set_size(content.len() as u64);
            header.set_mode(0o755);
            header.set_uid(0);
            header.set_gid(0);
            header.set_mtime(DETERMINISTIC_MTIME);
            header.set_cksum();
            archive.append(&header, content.as_bytes())?;
        }

        // write signature
        if let Some(ref sig) = package.signature {
            let mut header = tar::Header::new_gnu();
            header.set_path("signature")?;
            header.set_size(sig.len() as u64);
            header.set_mode(0o644);
            header.set_uid(0);
            header.set_gid(0);
            header.set_mtime(DETERMINISTIC_MTIME);
            header.set_cksum();
            archive.append(&header, &sig[..])?;
        }

        archive.finish()?;
        Ok(())
    }
}

/// reads a `Package` from the `.astpkg` format (tar + zstd).
pub struct PackageReader;

impl PackageReader {
    /// reads a package from a file.
    pub fn read_from_file(path: &Path) -> Result<Package, PackageError> {
        let file = std::fs::File::open(path)?;
        Self::read(file)
    }

    /// reads a package from any reader.
    pub fn read<R: Read>(reader: R) -> Result<Package, PackageError> {
        let decoder = zstd::Decoder::new(reader)?;
        let mut archive = tar::Archive::new(decoder);

        let mut metadata: Option<Metadata> = None;
        let mut files: HashMap<PathBuf, Vec<u8>> = HashMap::new();
        let mut scripts: HashMap<ScriptType, String> = HashMap::new();
        let mut signature: Option<Vec<u8>> = None;

        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.to_path_buf();
            let path_str = path.to_string_lossy().replace('\\', "/");

            let mut content = Vec::new();
            entry.read_to_end(&mut content)?;

            if path_str == "metadata.json" {
                metadata = Some(serde_json::from_slice(&content)?);
            } else if path_str == "signature" {
                signature = Some(content);
            } else if let Some(file_path) = path_str.strip_prefix("files/") {
                if !file_path.is_empty() {
                    files.insert(PathBuf::from(file_path), content);
                }
            } else if let Some(script_name) = path_str.strip_prefix("scripts/") {
                let script_content = String::from_utf8(content).map_err(|e| {
                    PackageError::InvalidFormat(format!("script is not valid UTF-8: {e}"))
                })?;
                let script_type = match script_name {
                    "pre_install.sh" => ScriptType::PreInstall,
                    "post_install.sh" => ScriptType::PostInstall,
                    "pre_remove.sh" => ScriptType::PreRemove,
                    "post_remove.sh" => ScriptType::PostRemove,
                    "pre_upgrade.sh" => ScriptType::PreUpgrade,
                    "post_upgrade.sh" => ScriptType::PostUpgrade,
                    _ => continue,
                };
                scripts.insert(script_type, script_content);
            }
        }

        let metadata = metadata.ok_or(PackageError::MissingMetadata)?;
        metadata.validate()?;

        Ok(Package {
            metadata,
            files,
            scripts,
            signature,
        })
    }

    /// reads just the metadata from a package file without extracting everything.
    pub fn read_metadata(path: &Path) -> Result<Metadata, PackageError> {
        let file = std::fs::File::open(path)?;
        let decoder = zstd::Decoder::new(file)?;
        let mut archive = tar::Archive::new(decoder);

        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.to_path_buf();
            if path.to_string_lossy() == "metadata.json" {
                let mut content = Vec::new();
                entry.read_to_end(&mut content)?;
                let metadata: Metadata = serde_json::from_slice(&content)?;
                metadata.validate()?;
                return Ok(metadata);
            }
        }

        Err(PackageError::MissingMetadata)
    }

    /// computes the sha-256 hash of a package file.
    pub fn file_checksum(path: &Path) -> Result<String, PackageError> {
        let mut file = std::fs::File::open(path)?;
        let mut hasher = Sha256::new();
        io::copy(&mut file, &mut hasher)?;
        Ok(hex::encode(hasher.finalize()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use semver::Version;
    use std::collections::BTreeMap;

    fn sample_metadata() -> Metadata {
        Metadata {
            name: "test-pkg".into(),
            version: Version::new(1, 0, 0),
            architecture: "x86_64".into(),
            description: "A test package".into(),
            dependencies: vec![],
            optional_dependencies: vec![],
            conflicts: vec![],
            provides: vec![],
            maintainer: "Test <test@example.com>".into(),
            license: "ZPL-2.0".into(),
            build_date: Utc::now(),
            checksums: BTreeMap::new(),
            installed_size: 0,
        }
    }

    #[test]
    fn test_package_roundtrip() {
        let mut pkg = Package::new(sample_metadata());
        pkg.add_file("usr/bin/hello", b"#!/bin/sh\necho hello\n".to_vec());
        pkg.add_script(ScriptType::PostInstall, "echo installed".into());
        pkg.compute_checksums();

        let mut buf = Vec::new();
        PackageWriter::write(&pkg, &mut buf).unwrap();
        let pkg2 = PackageReader::read(&buf[..]).unwrap();

        assert_eq!(pkg2.metadata.name, "test-pkg");
        assert_eq!(pkg2.files.len(), 1);
        assert_eq!(pkg2.scripts.len(), 1);
    }

    #[test]
    fn test_package_sign_verify() {
        let keypair = astra_crypto::KeyPair::generate();
        let mut pkg = Package::new(sample_metadata());
        pkg.add_file("usr/bin/hello", b"#!/bin/sh\necho hello\n".to_vec());
        pkg.sign(&keypair);

        let mut buf = Vec::new();
        PackageWriter::write(&pkg, &mut buf).unwrap();
        let pkg2 = PackageReader::read(&buf[..]).unwrap();
        assert!(pkg2.verify(&keypair.public_key()).is_ok());
    }
}
