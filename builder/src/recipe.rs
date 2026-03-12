use astra_pkg::Dependency;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// a package build recipe (Astrafile.yaml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    /// package name.
    pub name: String,
    /// version string.
    pub version: String,
    /// target architecture.
    #[serde(default = "default_arch")]
    pub architecture: String,
    /// package description.
    pub description: String,
    /// package maintainer.
    pub maintainer: String,
    /// package license.
    pub license: String,
    /// dependencies.
    #[serde(default)]
    pub dependencies: Vec<RecipeDependency>,
    /// optional dependencies.
    #[serde(default)]
    pub optional_dependencies: Vec<RecipeDependency>,
    /// conflicting packages.
    #[serde(default)]
    pub conflicts: Vec<String>,
    /// packages this provides.
    #[serde(default)]
    pub provides: Vec<String>,
    /// install scripts.
    #[serde(default)]
    pub scripts: HashMap<String, String>,
    /// source directory containing files to package (relative to Astrafile.yaml).
    #[serde(default = "default_files_dir")]
    pub files_dir: String,
}

fn default_arch() -> String {
    #[cfg(target_arch = "x86_64")]
    return "x86_64".to_string();
    #[cfg(target_arch = "aarch64")]
    return "aarch64".to_string();
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    return "any".to_string();
}

fn default_files_dir() -> String {
    "files".to_string()
}

/// a dependency entry in a recipe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeDependency {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
}

impl From<&RecipeDependency> for Dependency {
    fn from(rd: &RecipeDependency) -> Self {
        Dependency {
            name: rd.name.clone(),
            version_req: rd.version.clone(),
        }
    }
}

impl Recipe {
    /// loads a recipe from a yaml file.
    pub fn load(path: &Path) -> Result<Self, crate::BuildError> {
        let content = std::fs::read_to_string(path)?;
        let recipe: Self = serde_yaml::from_str(&content)?;
        recipe.validate()?;
        Ok(recipe)
    }

    /// validates the recipe fields.
    pub fn validate(&self) -> Result<(), crate::BuildError> {
        if self.name.is_empty() {
            return Err(crate::BuildError::InvalidRecipe("name is required".into()));
        }
        if self.version.is_empty() {
            return Err(crate::BuildError::InvalidRecipe(
                "version is required".into(),
            ));
        }
        if self.description.is_empty() {
            return Err(crate::BuildError::InvalidRecipe(
                "description is required".into(),
            ));
        }
        if self.maintainer.is_empty() {
            return Err(crate::BuildError::InvalidRecipe(
                "maintainer is required".into(),
            ));
        }
        if self.license.is_empty() {
            return Err(crate::BuildError::InvalidRecipe(
                "license is required".into(),
            ));
        }
        // validate version is valid semver
        semver::Version::parse(&self.version).map_err(|e| {
            crate::BuildError::InvalidRecipe(format!("invalid version '{}': {}", self.version, e))
        })?;
        Ok(())
    }
}
