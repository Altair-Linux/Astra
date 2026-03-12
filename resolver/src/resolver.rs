use crate::ResolverError;
use astra_pkg::Dependency;
use semver::{Version, VersionReq};
use std::collections::{HashMap, HashSet, VecDeque};

/// A package candidate available for installation.
#[derive(Debug, Clone)]
pub struct PackageCandidate {
    pub name: String,
    pub version: Version,
    pub dependencies: Vec<Dependency>,
    pub optional_dependencies: Vec<Dependency>,
    pub conflicts: Vec<String>,
    pub provides: Vec<String>,
}

/// Result of dependency resolution.
#[derive(Debug, Clone)]
pub struct ResolutionResult {
    /// Packages to install, in topological order.
    pub install_order: Vec<String>,
    /// Map of package name to selected version.
    pub selected: HashMap<String, Version>,
}

/// The dependency resolver.
///
/// Uses a BFS-based forward resolution algorithm.
/// Architecture allows future replacement with a SAT solver.
pub struct Resolver {
    /// Available packages: name -> list of candidates (different versions).
    available: HashMap<String, Vec<PackageCandidate>>,
    /// Already installed packages.
    installed: HashMap<String, Version>,
}

impl Resolver {
    /// Create a new resolver.
    pub fn new() -> Self {
        Self {
            available: HashMap::new(),
            installed: HashMap::new(),
        }
    }

    /// Add an available package candidate.
    pub fn add_candidate(&mut self, candidate: PackageCandidate) {
        self.available
            .entry(candidate.name.clone())
            .or_default()
            .push(candidate);
    }

    /// Register an already-installed package.
    pub fn add_installed(&mut self, name: String, version: Version) {
        self.installed.insert(name, version);
    }

    /// Resolve dependencies for the requested packages.
    pub fn resolve(&self, requests: &[String]) -> Result<ResolutionResult, ResolverError> {
        let mut selected: HashMap<String, PackageCandidate> = HashMap::new();
        let mut queue: VecDeque<String> = VecDeque::new();

        // Enqueue initial requests
        for name in requests {
            if !self.installed.contains_key(name) {
                queue.push_back(name.clone());
            }
        }

        // BFS resolution
        while let Some(name) = queue.pop_front() {
            if selected.contains_key(&name) || self.installed.contains_key(&name) {
                continue;
            }

            let candidate = self.select_best_candidate(&name, None)?;

            // Check conflicts
            for conflict in &candidate.conflicts {
                if self.installed.contains_key(conflict) || selected.contains_key(conflict) {
                    return Err(ResolverError::Conflict {
                        package_a: name.clone(),
                        package_b: conflict.clone(),
                    });
                }
            }

            // Enqueue dependencies
            for dep in &candidate.dependencies {
                if !self.installed.contains_key(&dep.name) && !selected.contains_key(&dep.name) {
                    // Verify the dependency can be satisfied
                    self.select_best_candidate(&dep.name, dep.version_req.as_deref())?;
                    queue.push_back(dep.name.clone());
                } else if let Some(req_str) = &dep.version_req {
                    // Check installed version satisfies requirement
                    if let Some(installed_ver) = self.installed.get(&dep.name) {
                        if let Ok(req) = VersionReq::parse(req_str) {
                            if !req.matches(installed_ver) {
                                return Err(ResolverError::NoSatisfyingVersion {
                                    package: dep.name.clone(),
                                    requirement: req_str.clone(),
                                });
                            }
                        }
                    }
                }
            }

            selected.insert(name, candidate);
        }

        // Check for circular dependencies and produce topological order
        let install_order = self.topological_sort(&selected)?;

        let selected_versions = selected
            .into_iter()
            .map(|(name, c)| (name, c.version))
            .collect();

        Ok(ResolutionResult {
            install_order,
            selected: selected_versions,
        })
    }

    /// Select the best candidate for a package, optionally satisfying a version requirement.
    fn select_best_candidate(
        &self,
        name: &str,
        version_req: Option<&str>,
    ) -> Result<PackageCandidate, ResolverError> {
        // Check "provides" as well
        let candidates = self.available.get(name).or_else(|| {
            // Look for packages that provide this name
            for (_, cands) in &self.available {
                for c in cands {
                    if c.provides.contains(&name.to_string()) {
                        return Some(cands);
                    }
                }
            }
            None
        });

        let candidates =
            candidates.ok_or_else(|| ResolverError::PackageNotFound(name.to_string()))?;

        let req = match version_req {
            Some(s) => VersionReq::parse(s).map_err(|_| ResolverError::NoSatisfyingVersion {
                package: name.to_string(),
                requirement: s.to_string(),
            })?,
            None => VersionReq::STAR,
        };

        // Find the highest version that matches
        let mut matching: Vec<_> = candidates.iter().filter(|c| req.matches(&c.version)).collect();
        matching.sort_by(|a, b| b.version.cmp(&a.version));

        matching.first().cloned().cloned().ok_or_else(|| {
            ResolverError::NoSatisfyingVersion {
                package: name.to_string(),
                requirement: version_req.unwrap_or("*").to_string(),
            }
        })
    }

    /// Topological sort of selected packages (Kahn's algorithm).
    fn topological_sort(
        &self,
        selected: &HashMap<String, PackageCandidate>,
    ) -> Result<Vec<String>, ResolverError> {
        let names: HashSet<&String> = selected.keys().collect();

        // Build adjacency: edges from dependency -> dependent
        let mut in_degree: HashMap<&String, usize> = HashMap::new();
        let mut dependents: HashMap<&String, Vec<&String>> = HashMap::new();

        for name in &names {
            in_degree.entry(name).or_insert(0);
            dependents.entry(name).or_default();
        }

        for (name, candidate) in selected {
            for dep in &candidate.dependencies {
                if names.contains(&dep.name) {
                    *in_degree.entry(&name).or_insert(0) += 1;
                    dependents.entry(&dep.name).or_default().push(&name);
                }
            }
        }

        let mut queue: VecDeque<&String> = VecDeque::new();
        for (name, &deg) in &in_degree {
            if deg == 0 {
                queue.push_back(name);
            }
        }

        let mut order = Vec::new();
        while let Some(name) = queue.pop_front() {
            order.push((*name).clone());
            if let Some(deps) = dependents.get(name) {
                for dep in deps {
                    let deg = in_degree.get_mut(dep).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(dep);
                    }
                }
            }
        }

        if order.len() != names.len() {
            // Circular dependency detected - find the cycle
            let remaining: Vec<String> = names
                .iter()
                .filter(|n| !order.contains(n))
                .map(|n| (*n).clone())
                .collect();
            return Err(ResolverError::CircularDependency { cycle: remaining });
        }

        Ok(order)
    }
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(name: &str, version: &str, deps: Vec<Dependency>) -> PackageCandidate {
        PackageCandidate {
            name: name.into(),
            version: Version::parse(version).unwrap(),
            dependencies: deps,
            optional_dependencies: vec![],
            conflicts: vec![],
            provides: vec![],
        }
    }

    #[test]
    fn test_simple_resolve() {
        let mut resolver = Resolver::new();
        resolver.add_candidate(candidate("app", "1.0.0", vec![Dependency::new("lib")]));
        resolver.add_candidate(candidate("lib", "2.0.0", vec![]));

        let result = resolver.resolve(&["app".into()]).unwrap();
        assert_eq!(result.install_order.len(), 2);
        assert_eq!(result.install_order[0], "lib");
        assert_eq!(result.install_order[1], "app");
    }

    #[test]
    fn test_already_installed() {
        let mut resolver = Resolver::new();
        resolver.add_candidate(candidate(
            "app",
            "1.0.0",
            vec![Dependency::new("lib")],
        ));
        resolver.add_installed("lib".into(), Version::new(2, 0, 0));

        let result = resolver.resolve(&["app".into()]).unwrap();
        assert_eq!(result.install_order.len(), 1);
        assert_eq!(result.install_order[0], "app");
    }

    #[test]
    fn test_conflict() {
        let mut resolver = Resolver::new();
        let mut c = candidate("a", "1.0.0", vec![]);
        c.conflicts = vec!["b".into()];
        resolver.add_candidate(c);
        resolver.add_installed("b".into(), Version::new(1, 0, 0));

        let result = resolver.resolve(&["a".into()]);
        assert!(matches!(result, Err(ResolverError::Conflict { .. })));
    }

    #[test]
    fn test_circular_dependency() {
        let mut resolver = Resolver::new();
        resolver.add_candidate(candidate("a", "1.0.0", vec![Dependency::new("b")]));
        resolver.add_candidate(candidate("b", "1.0.0", vec![Dependency::new("a")]));

        let result = resolver.resolve(&["a".into()]);
        assert!(matches!(
            result,
            Err(ResolverError::CircularDependency { .. })
        ));
    }

    #[test]
    fn test_version_requirement() {
        let mut resolver = Resolver::new();
        resolver.add_candidate(candidate(
            "app",
            "1.0.0",
            vec![Dependency::with_version("lib", ">=2.0.0")],
        ));
        resolver.add_candidate(candidate("lib", "1.0.0", vec![]));
        resolver.add_candidate(candidate("lib", "2.5.0", vec![]));

        let result = resolver.resolve(&["app".into()]).unwrap();
        assert_eq!(result.selected.get("lib").unwrap(), &Version::new(2, 5, 0));
    }

    #[test]
    fn test_missing_dependency() {
        let mut resolver = Resolver::new();
        resolver.add_candidate(candidate(
            "app",
            "1.0.0",
            vec![Dependency::new("nonexistent")],
        ));

        let result = resolver.resolve(&["app".into()]);
        assert!(matches!(result, Err(ResolverError::PackageNotFound(_))));
    }

    #[test]
    fn test_diamond_dependency() {
        let mut resolver = Resolver::new();
        resolver.add_candidate(candidate(
            "app",
            "1.0.0",
            vec![Dependency::new("lib-a"), Dependency::new("lib-b")],
        ));
        resolver.add_candidate(candidate(
            "lib-a",
            "1.0.0",
            vec![Dependency::new("common")],
        ));
        resolver.add_candidate(candidate(
            "lib-b",
            "1.0.0",
            vec![Dependency::new("common")],
        ));
        resolver.add_candidate(candidate("common", "1.0.0", vec![]));

        let result = resolver.resolve(&["app".into()]).unwrap();
        assert_eq!(result.install_order.len(), 4);
        // common must come before lib-a and lib-b
        let common_pos = result.install_order.iter().position(|n| n == "common").unwrap();
        let a_pos = result.install_order.iter().position(|n| n == "lib-a").unwrap();
        let b_pos = result.install_order.iter().position(|n| n == "lib-b").unwrap();
        let app_pos = result.install_order.iter().position(|n| n == "app").unwrap();
        assert!(common_pos < a_pos);
        assert!(common_pos < b_pos);
        assert!(a_pos < app_pos);
        assert!(b_pos < app_pos);
    }
}
