use crate::DbError;
use astra_pkg::Metadata;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use semver::Version;
use std::path::{Path, PathBuf};

/// Record of an installed package.
#[derive(Debug, Clone)]
pub struct InstalledPackage {
    pub name: String,
    pub version: Version,
    pub architecture: String,
    pub description: String,
    pub maintainer: String,
    pub license: String,
    pub installed_size: u64,
    pub install_date: DateTime<Utc>,
    pub files: Vec<PathBuf>,
    pub dependencies: Vec<String>,
    pub reason: InstallReason,
}

/// Why a package was installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallReason {
    /// Explicitly requested by user.
    Explicit,
    /// Installed as a dependency.
    Dependency,
}

impl InstallReason {
    fn as_str(&self) -> &'static str {
        match self {
            InstallReason::Explicit => "explicit",
            InstallReason::Dependency => "dependency",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "dependency" => InstallReason::Dependency,
            _ => InstallReason::Explicit,
        }
    }
}

/// The local package database.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create the database at the given path.
    pub fn open(path: &Path) -> Result<Self, DbError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;

        // Enable WAL mode for crash recovery
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;

        let db = Self { conn };
        db.init_tables()?;
        Ok(db)
    }

    /// Open an in-memory database (for testing).
    pub fn open_memory() -> Result<Self, DbError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        let db = Self { conn };
        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&self) -> Result<(), DbError> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS packages (
                name TEXT PRIMARY KEY,
                version TEXT NOT NULL,
                architecture TEXT NOT NULL,
                description TEXT NOT NULL,
                maintainer TEXT NOT NULL,
                license TEXT NOT NULL,
                installed_size INTEGER NOT NULL DEFAULT 0,
                install_date TEXT NOT NULL,
                reason TEXT NOT NULL DEFAULT 'explicit',
                metadata_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS files (
                path TEXT PRIMARY KEY,
                package_name TEXT NOT NULL,
                FOREIGN KEY (package_name) REFERENCES packages(name) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS dependencies (
                package_name TEXT NOT NULL,
                dependency_name TEXT NOT NULL,
                version_req TEXT,
                PRIMARY KEY (package_name, dependency_name),
                FOREIGN KEY (package_name) REFERENCES packages(name) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_files_package ON files(package_name);
            CREATE INDEX IF NOT EXISTS idx_deps_package ON dependencies(package_name);
            CREATE INDEX IF NOT EXISTS idx_deps_dependency ON dependencies(dependency_name);",
        )?;
        Ok(())
    }

    /// Register an installed package.
    pub fn install_package(
        &self,
        metadata: &Metadata,
        files: &[PathBuf],
        reason: InstallReason,
    ) -> Result<(), DbError> {
        let tx = self.conn.unchecked_transaction()?;

        let meta_json = serde_json::to_string(metadata)?;
        let now = Utc::now().to_rfc3339();

        tx.execute(
            "INSERT OR REPLACE INTO packages
             (name, version, architecture, description, maintainer, license,
              installed_size, install_date, reason, metadata_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                metadata.name,
                metadata.version.to_string(),
                metadata.architecture,
                metadata.description,
                metadata.maintainer,
                metadata.license,
                metadata.installed_size,
                now,
                reason.as_str(),
                meta_json,
            ],
        )?;

        // Remove old files first (for upgrades)
        tx.execute("DELETE FROM files WHERE package_name = ?1", params![metadata.name])?;
        tx.execute(
            "DELETE FROM dependencies WHERE package_name = ?1",
            params![metadata.name],
        )?;

        // Insert files
        for file_path in files {
            tx.execute(
                "INSERT OR REPLACE INTO files (path, package_name) VALUES (?1, ?2)",
                params![file_path.to_string_lossy().to_string(), metadata.name],
            )?;
        }

        // Insert dependencies
        for dep in &metadata.dependencies {
            tx.execute(
                "INSERT OR REPLACE INTO dependencies (package_name, dependency_name, version_req)
                 VALUES (?1, ?2, ?3)",
                params![metadata.name, dep.name, dep.version_req],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Remove a package from the database.
    pub fn remove_package(&self, name: &str) -> Result<Vec<PathBuf>, DbError> {
        let files = self.get_package_files(name)?;
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM files WHERE package_name = ?1", params![name])?;
        tx.execute(
            "DELETE FROM dependencies WHERE package_name = ?1",
            params![name],
        )?;
        tx.execute("DELETE FROM packages WHERE name = ?1", params![name])?;
        tx.commit()?;
        Ok(files)
    }

    /// Check if a package is installed.
    pub fn is_installed(&self, name: &str) -> Result<bool, DbError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM packages WHERE name = ?1",
            params![name],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Get an installed package by name.
    pub fn get_package(&self, name: &str) -> Result<InstalledPackage, DbError> {
        let row = self.conn.query_row(
            "SELECT name, version, architecture, description, maintainer, license,
                    installed_size, install_date, reason
             FROM packages WHERE name = ?1",
            params![name],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                ))
            },
        );

        match row {
            Ok((name, version, arch, desc, maint, lic, size, date, reason)) => {
                let files = self.get_package_files(&name)?;
                let deps = self.get_package_dependencies(&name)?;
                Ok(InstalledPackage {
                    name,
                    version: Version::parse(&version)
                        .unwrap_or_else(|_| Version::new(0, 0, 0)),
                    architecture: arch,
                    description: desc,
                    maintainer: maint,
                    license: lic,
                    installed_size: size as u64,
                    install_date: DateTime::parse_from_rfc3339(&date)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    files,
                    dependencies: deps,
                    reason: InstallReason::from_str(&reason),
                })
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                Err(DbError::PackageNotFound(name.to_string()))
            }
            Err(e) => Err(DbError::Sqlite(e)),
        }
    }

    /// Get all installed packages.
    pub fn list_packages(&self) -> Result<Vec<InstalledPackage>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT name FROM packages ORDER BY name",
        )?;
        let names: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        let mut packages = Vec::new();
        for name in names {
            packages.push(self.get_package(&name)?);
        }
        Ok(packages)
    }

    /// Search packages by name or description.
    pub fn search_packages(&self, query: &str) -> Result<Vec<InstalledPackage>, DbError> {
        let pattern = format!("%{query}%");
        let mut stmt = self.conn.prepare(
            "SELECT name FROM packages WHERE name LIKE ?1 OR description LIKE ?1 ORDER BY name",
        )?;
        let names: Vec<String> = stmt
            .query_map(params![pattern], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        let mut packages = Vec::new();
        for name in names {
            packages.push(self.get_package(&name)?);
        }
        Ok(packages)
    }

    /// Get the files installed by a package.
    pub fn get_package_files(&self, name: &str) -> Result<Vec<PathBuf>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT path FROM files WHERE package_name = ?1 ORDER BY path")?;
        let files = stmt
            .query_map(params![name], |row| {
                let p: String = row.get(0)?;
                Ok(PathBuf::from(p))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(files)
    }

    /// Get dependencies of a package.
    pub fn get_package_dependencies(&self, name: &str) -> Result<Vec<String>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT dependency_name FROM dependencies WHERE package_name = ?1 ORDER BY dependency_name",
        )?;
        let deps = stmt
            .query_map(params![name], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(deps)
    }

    /// Get packages that depend on a given package.
    pub fn get_reverse_dependencies(&self, name: &str) -> Result<Vec<String>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT package_name FROM dependencies WHERE dependency_name = ?1 ORDER BY package_name",
        )?;
        let deps = stmt
            .query_map(params![name], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(deps)
    }

    /// Find which package owns a file.
    pub fn find_file_owner(&self, path: &str) -> Result<Option<String>, DbError> {
        let result = self.conn.query_row(
            "SELECT package_name FROM files WHERE path = ?1",
            params![path],
            |row| row.get(0),
        );
        match result {
            Ok(name) => Ok(Some(name)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DbError::Sqlite(e)),
        }
    }

    /// Get the stored metadata JSON for a package.
    pub fn get_metadata(&self, name: &str) -> Result<Metadata, DbError> {
        let json: String = self.conn.query_row(
            "SELECT metadata_json FROM packages WHERE name = ?1",
            params![name],
            |row| row.get(0),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => DbError::PackageNotFound(name.to_string()),
            other => DbError::Sqlite(other),
        })?;
        let metadata: Metadata = serde_json::from_str(&json)?;
        Ok(metadata)
    }

    /// Get count of installed packages.
    pub fn package_count(&self) -> Result<usize, DbError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM packages",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astra_pkg::{Dependency, Metadata};
    use chrono::Utc;
    use semver::Version;
    use std::collections::HashMap;

    fn test_metadata(name: &str) -> Metadata {
        Metadata {
            name: name.into(),
            version: Version::new(1, 0, 0),
            architecture: "x86_64".into(),
            description: format!("Test package {name}"),
            dependencies: vec![],
            optional_dependencies: vec![],
            conflicts: vec![],
            provides: vec![],
            maintainer: "Test <test@test.com>".into(),
            license: "ZPL-2.0".into(),
            build_date: Utc::now(),
            checksums: HashMap::new(),
            installed_size: 1024,
        }
    }

    #[test]
    fn test_install_and_query() {
        let db = Database::open_memory().unwrap();
        let meta = test_metadata("hello");
        let files = vec![
            PathBuf::from("usr/bin/hello"),
            PathBuf::from("usr/share/doc/hello/README"),
        ];
        db.install_package(&meta, &files, InstallReason::Explicit).unwrap();

        assert!(db.is_installed("hello").unwrap());
        assert!(!db.is_installed("nonexistent").unwrap());

        let pkg = db.get_package("hello").unwrap();
        assert_eq!(pkg.name, "hello");
        assert_eq!(pkg.version, Version::new(1, 0, 0));
        assert_eq!(pkg.files.len(), 2);
    }

    #[test]
    fn test_remove() {
        let db = Database::open_memory().unwrap();
        let meta = test_metadata("hello");
        let files = vec![PathBuf::from("usr/bin/hello")];
        db.install_package(&meta, &files, InstallReason::Explicit).unwrap();
        assert!(db.is_installed("hello").unwrap());

        let removed_files = db.remove_package("hello").unwrap();
        assert_eq!(removed_files.len(), 1);
        assert!(!db.is_installed("hello").unwrap());
    }

    #[test]
    fn test_list_and_search() {
        let db = Database::open_memory().unwrap();
        db.install_package(&test_metadata("alpha"), &[], InstallReason::Explicit).unwrap();
        db.install_package(&test_metadata("beta"), &[], InstallReason::Explicit).unwrap();

        let all = db.list_packages().unwrap();
        assert_eq!(all.len(), 2);

        let results = db.search_packages("alpha").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "alpha");
    }

    #[test]
    fn test_file_owner() {
        let db = Database::open_memory().unwrap();
        let meta = test_metadata("hello");
        let files = vec![PathBuf::from("usr/bin/hello")];
        db.install_package(&meta, &files, InstallReason::Explicit).unwrap();

        let owner = db.find_file_owner("usr/bin/hello").unwrap();
        assert_eq!(owner.unwrap(), "hello");
        assert!(db.find_file_owner("usr/bin/nonexistent").unwrap().is_none());
    }

    #[test]
    fn test_dependencies() {
        let db = Database::open_memory().unwrap();
        let mut meta = test_metadata("app");
        meta.dependencies = vec![
            Dependency::new("lib-a"),
            Dependency::with_version("lib-b", ">=1.0.0"),
        ];
        db.install_package(&meta, &[], InstallReason::Explicit).unwrap();

        // Install dependency too
        db.install_package(&test_metadata("lib-a"), &[], InstallReason::Dependency).unwrap();

        let deps = db.get_package_dependencies("app").unwrap();
        assert_eq!(deps.len(), 2);

        let rdeps = db.get_reverse_dependencies("lib-a").unwrap();
        assert_eq!(rdeps.len(), 1);
        assert_eq!(rdeps[0], "app");
    }
}
