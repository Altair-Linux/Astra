use crate::{BuildError, Recipe};
use astra_crypto::sha256_hex;
use astra_crypto::KeyPair;
use astra_pkg::{Metadata, Package, PackageWriter, ScriptType};
use chrono::{TimeZone, Utc};
use semver::Version;
use std::collections::{BTreeMap, HashMap};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
use walkdir::WalkDir;

/// the package builder.
pub struct Builder;

impl Builder {
    /// builds a package from a directory containing `astra.pkg` or `Astrafile.yaml`.
    pub fn build(
        pkg_dir: &Path,
        keypair: &KeyPair,
        output_dir: &Path,
        sandbox: bool,
    ) -> Result<PathBuf, BuildError> {
        let recipe_path = if pkg_dir.join("astra.pkg").exists() {
            pkg_dir.join("astra.pkg")
        } else {
            pkg_dir.join("Astrafile.yaml")
        };
        if !recipe_path.exists() {
            return Err(BuildError::RecipeNotFound(
                recipe_path.to_string_lossy().to_string(),
            ));
        }

        let recipe = Recipe::load(&recipe_path)?;
        tracing::info!("Building package: {} v{}", recipe.name, recipe.version);

        let temp_workspace = TempDir::new()?;
        let source_dir = temp_workspace.path().join("source");
        let stage_dir = temp_workspace.path().join("stage");
        std::fs::create_dir_all(&source_dir)?;
        std::fs::create_dir_all(&stage_dir)?;

        #[cfg(unix)]
        let mut downloaded_source: Option<PathBuf> = None;
        if let Some(source_url) = &recipe.source_url {
            let destination = source_dir.join("source.archive");
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| BuildError::BuildFailed(format!("tokio runtime error: {e}")))?;
            rt.block_on(async {
                let response = reqwest::get(source_url)
                    .await
                    .map_err(|e| BuildError::BuildFailed(format!("source download failed: {e}")))?;
                let response = response
                    .error_for_status()
                    .map_err(|e| BuildError::BuildFailed(format!("source download failed: {e}")))?;
                let body = response
                    .bytes()
                    .await
                    .map_err(|e| BuildError::BuildFailed(format!("source download failed: {e}")))?;
                std::fs::write(&destination, &body)?;
                Ok::<(), BuildError>(())
            })?;

            if let Some(expected) = &recipe.source_sha256 {
                let bytes = std::fs::read(&destination)?;
                let actual = sha256_hex(&bytes);
                if &actual != expected {
                    return Err(BuildError::BuildFailed(format!(
                        "source checksum mismatch: expected {}, got {}",
                        expected, actual
                    )));
                }
            }

            let extract_attempt = Command::new("tar")
                .arg("-xf")
                .arg(&destination)
                .arg("-C")
                .arg(&source_dir)
                .status();
            match extract_attempt {
                Ok(status) if status.success() => {}
                _ => {
                    tracing::warn!("Unable to auto-extract source archive; keeping raw archive only");
                }
            }

            #[cfg(unix)]
            {
                downloaded_source = Some(destination);
            }
        }

        let patches_dir = pkg_dir.join("patches");
        if patches_dir.exists() {
            let mut patch_files: Vec<PathBuf> = std::fs::read_dir(&patches_dir)?
                .filter_map(|entry| entry.ok().map(|e| e.path()))
                .filter(|path| {
                    path.extension()
                        .and_then(OsStr::to_str)
                        .map(|ext| ext.eq_ignore_ascii_case("patch"))
                        .unwrap_or(false)
                })
                .collect();
            patch_files.sort();

            for patch in patch_files {
                let status = Command::new("patch")
                    .arg("-p1")
                    .arg("-i")
                    .arg(&patch)
                    .current_dir(&source_dir)
                    .status();
                match status {
                    Ok(s) if s.success() => {}
                    _ => {
                        tracing::warn!("Skipping patch application for {:?}; patch tool unavailable or failed", patch);
                    }
                }
            }
        }

        let build_script = pkg_dir.join("build.sh");
        if build_script.exists() {
            #[cfg(not(unix))]
            {
                let _ = sandbox;
                tracing::warn!("build.sh present but host is non-Unix; skipping script execution and using files_dir fallback");
            }

            #[cfg(unix)]
            {
                let mut command = Command::new("sh");
                command.arg(&build_script);
                command.current_dir(pkg_dir);
                command.env("DESTDIR", &stage_dir);
                command.env("ASTRA_SOURCE_DIR", &source_dir);
                if let Some(source_file) = &downloaded_source {
                    command.env("ASTRA_SOURCE_ARCHIVE", source_file);
                }

                if sandbox {
                    command.env_clear();
                    command.env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin");
                    command.env("HOME", temp_workspace.path());
                    command.env("DESTDIR", &stage_dir);
                    command.env("ASTRA_SOURCE_DIR", &source_dir);
                    if let Some(source_file) = &downloaded_source {
                        command.env("ASTRA_SOURCE_ARCHIVE", source_file);
                    }
                }

                let status = command
                    .status()
                    .map_err(|e| BuildError::BuildFailed(format!("failed to run build.sh: {e}")))?;
                if !status.success() {
                    return Err(BuildError::BuildFailed(
                        "build.sh failed with non-zero exit status".into(),
                    ));
                }
            }
        }

        let source_files_dir = if stage_dir.exists() && std::fs::read_dir(&stage_dir)?.next().is_some() {
            stage_dir.clone()
        } else {
            pkg_dir.join(&recipe.files_dir)
        };

        let mut files: HashMap<PathBuf, Vec<u8>> = HashMap::new();
        if source_files_dir.exists() {
            for entry in WalkDir::new(&source_files_dir) {
                let entry =
                    entry.map_err(|e| BuildError::BuildFailed(format!("walkdir error: {e}")))?;
                if entry.file_type().is_file() {
                    let rel_path = entry
                        .path()
                        .strip_prefix(&source_files_dir)
                        .map_err(|e| BuildError::BuildFailed(format!("path error: {e}")))?;
                    let content = std::fs::read(entry.path())?;
                    let normalized = PathBuf::from(rel_path.to_string_lossy().replace('\\', "/"));
                    files.insert(normalized, content);
                }
            }
        }

        if files.is_empty() {
            return Err(BuildError::NoFiles);
        }

        let build_date = std::env::var("SOURCE_DATE_EPOCH")
            .ok()
            .and_then(|s| s.parse::<i64>().ok())
            .and_then(|epoch| Utc.timestamp_opt(epoch, 0).single())
            .unwrap_or_else(|| Utc.timestamp_opt(0, 0).single().expect("unix epoch"));

        // build metadata
        let metadata = Metadata {
            name: recipe.name.clone(),
            version: Version::parse(&recipe.version)
                .map_err(|e| BuildError::InvalidRecipe(format!("bad version: {e}")))?,
            architecture: recipe.architecture.clone(),
            description: recipe.description.clone(),
            dependencies: recipe.dependencies.iter().map(|d| d.into()).collect(),
            optional_dependencies: recipe
                .optional_dependencies
                .iter()
                .map(|d| d.into())
                .collect(),
            conflicts: recipe.conflicts.clone(),
            provides: recipe.provides.clone(),
            maintainer: recipe.maintainer.clone(),
            license: recipe.license.clone(),
            build_date,
            checksums: BTreeMap::new(),
            installed_size: 0,
        };

        // create package
        let mut package = Package::new(metadata);
        for (path, content) in files {
            package.add_file(path, content);
        }

        // add scripts from recipe
        for (script_name, script_content) in &recipe.scripts {
            let script_type = match script_name.as_str() {
                "pre_install" => ScriptType::PreInstall,
                "post_install" => ScriptType::PostInstall,
                "pre_remove" => ScriptType::PreRemove,
                "post_remove" => ScriptType::PostRemove,
                "pre_upgrade" => ScriptType::PreUpgrade,
                "post_upgrade" => ScriptType::PostUpgrade,
                other => {
                    tracing::warn!("Unknown script type: {other}, skipping");
                    continue;
                }
            };
            package.add_script(script_type, script_content.clone());
        }

        // add scripts from scripts/ directory when present
        let scripts_dir = pkg_dir.join("scripts");
        if scripts_dir.exists() {
            let known_scripts = [
                (ScriptType::PreInstall, "pre_install.sh"),
                (ScriptType::PostInstall, "post_install.sh"),
                (ScriptType::PreRemove, "pre_remove.sh"),
                (ScriptType::PostRemove, "post_remove.sh"),
                (ScriptType::PreUpgrade, "pre_upgrade.sh"),
                (ScriptType::PostUpgrade, "post_upgrade.sh"),
            ];

            for (script_type, filename) in known_scripts {
                let script_path = scripts_dir.join(filename);
                if script_path.exists() {
                    let content = std::fs::read_to_string(script_path)?;
                    package.add_script(script_type, content);
                }
            }
        }

        // sign it
        package.sign(keypair);
        tracing::info!("Package signed successfully");

        // write it out
        std::fs::create_dir_all(output_dir)?;
        let output_path = output_dir.join(package.metadata.filename());
        PackageWriter::write_to_file(&package, &output_path)?;

        tracing::info!("Package written to {:?}", output_path);
        Ok(output_path)
    }
}
