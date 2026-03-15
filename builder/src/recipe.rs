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
    /// optional source URL to fetch during build.
    #[serde(default)]
    pub source_url: Option<String>,
    /// expected sha256 for downloaded source artifact.
    #[serde(default)]
    pub source_sha256: Option<String>,
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
        let recipe: Self = if path
            .file_name()
            .map(|n| n == "astra.pkg")
            .unwrap_or(false)
        {
            Self::parse_astra_pkg(&content)?
        } else {
            serde_yaml::from_str(&content)?
        };
        recipe.validate()?;
        Ok(recipe)
    }

    fn parse_astra_pkg(content: &str) -> Result<Self, crate::BuildError> {
        let mut map: HashMap<String, String> = HashMap::new();

        for raw in content.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            map.insert(
                key.trim().to_string(),
                value.trim().trim_matches('"').to_string(),
            );
        }

        let dependencies = map
            .get("dependencies")
            .map(|deps| {
                deps.split(',')
                    .filter_map(|entry| {
                        let part = entry.trim();
                        if part.is_empty() {
                            return None;
                        }
                        if let Some((name, req)) = part.split_once(':') {
                            return Some(RecipeDependency {
                                name: name.trim().to_string(),
                                version: Some(req.trim().to_string()),
                            });
                        }
                        Some(RecipeDependency {
                            name: part.to_string(),
                            version: None,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let provides = map
            .get("provides")
            .map(|value| {
                value
                    .split(',')
                    .map(|x| x.trim().to_string())
                    .filter(|x| !x.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let conflicts = map
            .get("conflicts")
            .map(|value| {
                value
                    .split(',')
                    .map(|x| x.trim().to_string())
                    .filter(|x| !x.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(Self {
            name: map.get("name").cloned().unwrap_or_default(),
            version: map.get("version").cloned().unwrap_or_default(),
            architecture: map
                .get("architecture")
                .cloned()
                .unwrap_or_else(default_arch),
            description: map.get("description").cloned().unwrap_or_default(),
            maintainer: map.get("maintainer").cloned().unwrap_or_default(),
            license: map.get("license").cloned().unwrap_or_default(),
            dependencies,
            optional_dependencies: Vec::new(),
            conflicts,
            provides,
            scripts: HashMap::new(),
            files_dir: map
                .get("files_dir")
                .cloned()
                .unwrap_or_else(default_files_dir),
            source_url: map.get("source_url").cloned(),
            source_sha256: map.get("source_sha256").cloned(),
        })
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
        if self.source_url.is_some() && self.source_sha256.is_none() {
            return Err(crate::BuildError::InvalidRecipe(
                "source_sha256 is required when source_url is provided".into(),
            ));
        }
        // validate version is valid semver
        semver::Version::parse(&self.version).map_err(|e| {
            crate::BuildError::InvalidRecipe(format!("invalid version '{}': {}", self.version, e))
        })?;
        Ok(())
    }
}
