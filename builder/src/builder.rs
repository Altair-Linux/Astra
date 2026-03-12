use crate::{BuildError, Recipe};
use astra_crypto::KeyPair;
use astra_pkg::{Metadata, Package, PackageWriter, ScriptType};
use chrono::Utc;
use semver::Version;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// The package builder.
pub struct Builder;

impl Builder {
    /// Build a package from a directory containing an Astrafile.yaml.
    pub fn build(
        pkg_dir: &Path,
        keypair: &KeyPair,
        output_dir: &Path,
    ) -> Result<PathBuf, BuildError> {
        let recipe_path = pkg_dir.join("Astrafile.yaml");
        if !recipe_path.exists() {
            return Err(BuildError::RecipeNotFound(
                recipe_path.to_string_lossy().to_string(),
            ));
        }

        let recipe = Recipe::load(&recipe_path)?;
        tracing::info!("Building package: {} v{}", recipe.name, recipe.version);

        // Collect files
        let files_dir = pkg_dir.join(&recipe.files_dir);
        let mut files: HashMap<PathBuf, Vec<u8>> = HashMap::new();

        if files_dir.exists() {
            for entry in WalkDir::new(&files_dir) {
                let entry =
                    entry.map_err(|e| BuildError::BuildFailed(format!("walkdir error: {e}")))?;
                if entry.file_type().is_file() {
                    let rel_path = entry
                        .path()
                        .strip_prefix(&files_dir)
                        .map_err(|e| BuildError::BuildFailed(format!("path error: {e}")))?;
                    let content = std::fs::read(entry.path())?;
                    // Normalize to forward slashes for cross-platform consistency
                    let normalized = PathBuf::from(rel_path.to_string_lossy().replace('\\', "/"));
                    files.insert(normalized, content);
                }
            }
        }

        // Build metadata
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
            build_date: Utc::now(),
            checksums: HashMap::new(),
            installed_size: 0,
        };

        // Create package
        let mut package = Package::new(metadata);
        for (path, content) in files {
            package.add_file(path, content);
        }

        // Add scripts
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

        // Sign
        package.sign(keypair);
        tracing::info!("Package signed successfully");

        // Write package
        std::fs::create_dir_all(output_dir)?;
        let output_path = output_dir.join(package.metadata.filename());
        PackageWriter::write_to_file(&package, &output_path)?;

        tracing::info!("Package written to {:?}", output_path);
        Ok(output_path)
    }
}
