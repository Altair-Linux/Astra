use astra_pkg::Dependency;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// A package build recipe (Astrafile.yaml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    /// Package name.
    pub name: String,
    /// Package version string.
    pub version: String,
    /// Target architecture.
    #[serde(default = "default_arch")]
    pub architecture: String,
    /// Package description.
    pub description: String,
    /// Package maintainer.
    pub maintainer: String,
    /// Package license.
    pub license: String,
    /// Dependencies.
    #[serde(default)]
    pub dependencies: Vec<RecipeDependency>,
    /// Optional dependencies.
    #[serde(default)]
    pub optional_dependencies: Vec<RecipeDependency>,
    /// Conflicting packages.
    #[serde(default)]
    pub conflicts: Vec<String>,
    /// Packages this provides.
    #[serde(default)]
    pub provides: Vec<String>,
    /// Install scripts.
    #[serde(default)]
    pub scripts: HashMap<String, String>,
    /// Source directory containing files to package (relative to Astrafile.yaml).
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

/// A dependency entry in a recipe.
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
    /// Load a recipe from a YAML file.
    pub fn load(path: &Path) -> Result<Self, crate::BuildError> {
        let content = std::fs::read_to_string(path)?;
        let recipe: Self = serde_yaml::from_str(&content)?;
        recipe.validate()?;
        Ok(recipe)
    }

    /// Validate the recipe.
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
        // Validate version is valid semver
        semver::Version::parse(&self.version).map_err(|e| {
            crate::BuildError::InvalidRecipe(format!("invalid version '{}': {}", self.version, e))
        })?;
        Ok(())
    }
}
