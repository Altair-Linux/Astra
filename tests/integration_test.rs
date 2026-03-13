//! integration tests for the astra package manager.
//!
//! tests the full lifecycle: build, create repo, install, verify, remove.

use astra_builder::Builder;
use astra_core::{AstraConfig, PackageManager};
use astra_crypto::KeyPair;
use astra_db::{Database, InstallReason};
use astra_pkg::{Metadata, Package, PackageReader, PackageWriter};
use astra_repo::{RepoIndex, RepoPackageEntry};
use astra_resolver::{PackageCandidate, Resolver};
use chrono::Utc;
use semver::Version;
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

// ─── package format tests ──────────────────────────────────────────

#[test]
fn test_package_create_read_verify() {
    let keypair = KeyPair::generate();

    let metadata = Metadata {
        name: "test-pkg".into(),
        version: Version::new(1, 0, 0),
        architecture: "x86_64".into(),
        description: "Integration test package".into(),
        dependencies: vec![],
        optional_dependencies: vec![],
        conflicts: vec![],
        provides: vec![],
        maintainer: "Test <test@test.com>".into(),
        license: "ZPL-2.0".into(),
        build_date: Utc::now(),
        checksums: HashMap::new(),
        installed_size: 0,
    };

    let mut package = Package::new(metadata);
    package.add_file("usr/bin/test", b"#!/bin/sh\necho test\n".to_vec());
    package.add_file("etc/test.conf", b"key=value\n".to_vec());
    package.sign(&keypair);

    // write to buffer
    let mut buf = Vec::new();
    PackageWriter::write(&package, &mut buf).unwrap();

    // read back
    let pkg2 = PackageReader::read(&buf[..]).unwrap();
    assert_eq!(pkg2.metadata.name, "test-pkg");
    assert_eq!(pkg2.files.len(), 2);
    assert!(pkg2.signature.is_some());

    // verify
    assert!(pkg2.verify(&keypair.public_key()).is_ok());

    // verify with wrong key fails
    let wrong_key = KeyPair::generate();
    assert!(pkg2.verify(&wrong_key.public_key()).is_err());
}

#[test]
fn test_package_write_to_file() {
    let tmp = TempDir::new().unwrap();
    let keypair = KeyPair::generate();

    let metadata = Metadata {
        name: "file-test".into(),
        version: Version::new(2, 0, 0),
        architecture: "x86_64".into(),
        description: "File write test".into(),
        dependencies: vec![],
        optional_dependencies: vec![],
        conflicts: vec![],
        provides: vec![],
        maintainer: "Test <test@test.com>".into(),
        license: "ZPL-2.0".into(),
        build_date: Utc::now(),
        checksums: HashMap::new(),
        installed_size: 0,
    };

    let mut package = Package::new(metadata);
    package.add_file("usr/bin/hello", b"hello world".to_vec());
    package.sign(&keypair);

    let pkg_path = tmp.path().join("file-test-2.0.0-x86_64.astpkg");
    PackageWriter::write_to_file(&package, &pkg_path).unwrap();

    assert!(pkg_path.exists());

    // read back from file
    let pkg2 = PackageReader::read_from_file(&pkg_path).unwrap();
    assert_eq!(pkg2.metadata.name, "file-test");
    assert!(pkg2.verify(&keypair.public_key()).is_ok());

    // test metadata-only read
    let meta = PackageReader::read_metadata(&pkg_path).unwrap();
    assert_eq!(meta.name, "file-test");

    // test file checksum
    let checksum = PackageReader::file_checksum(&pkg_path).unwrap();
    assert!(!checksum.is_empty());
}

// ─── database tests ────────────────────────────────────────────────

#[test]
fn test_database_full_lifecycle() {
    let db = Database::open_memory().unwrap();

    let metadata = Metadata {
        name: "db-test".into(),
        version: Version::new(1, 0, 0),
        architecture: "x86_64".into(),
        description: "Database test package".into(),
        dependencies: vec![astra_pkg::Dependency::new("glibc")],
        optional_dependencies: vec![],
        conflicts: vec![],
        provides: vec![],
        maintainer: "Test <test@test.com>".into(),
        license: "ZPL-2.0".into(),
        build_date: Utc::now(),
        checksums: HashMap::new(),
        installed_size: 2048,
    };

    let files = vec![
        PathBuf::from("usr/bin/db-test"),
        PathBuf::from("etc/db-test.conf"),
    ];

    // install
    db.install_package(&metadata, &files, InstallReason::Explicit)
        .unwrap();
    assert!(db.is_installed("db-test").unwrap());
    assert_eq!(db.package_count().unwrap(), 1);

    // query
    let pkg = db.get_package("db-test").unwrap();
    assert_eq!(pkg.version, Version::new(1, 0, 0));
    assert_eq!(pkg.files.len(), 2);

    // file ownership
    let owner = db.find_file_owner("usr/bin/db-test").unwrap();
    assert_eq!(owner.unwrap(), "db-test");

    // search
    let results = db.search_packages("database").unwrap();
    assert_eq!(results.len(), 1);

    // remove
    let removed = db.remove_package("db-test").unwrap();
    assert_eq!(removed.len(), 2);
    assert!(!db.is_installed("db-test").unwrap());
}

// ─── resolver tests ────────────────────────────────────────────────

#[test]
fn test_resolver_complex_scenario() {
    let mut resolver = Resolver::new();

    // setup: app -> lib-a, lib-b; lib-a -> common; lib-b -> common
    resolver.add_candidate(PackageCandidate {
        name: "app".into(),
        version: Version::new(1, 0, 0),
        dependencies: vec![
            astra_pkg::Dependency::new("lib-a"),
            astra_pkg::Dependency::new("lib-b"),
        ],
        optional_dependencies: vec![],
        conflicts: vec![],
        provides: vec![],
    });
    resolver.add_candidate(PackageCandidate {
        name: "lib-a".into(),
        version: Version::new(1, 0, 0),
        dependencies: vec![astra_pkg::Dependency::with_version("common", ">=1.0.0")],
        optional_dependencies: vec![],
        conflicts: vec![],
        provides: vec![],
    });
    resolver.add_candidate(PackageCandidate {
        name: "lib-b".into(),
        version: Version::new(2, 0, 0),
        dependencies: vec![astra_pkg::Dependency::new("common")],
        optional_dependencies: vec![],
        conflicts: vec![],
        provides: vec![],
    });
    resolver.add_candidate(PackageCandidate {
        name: "common".into(),
        version: Version::new(1, 0, 0),
        dependencies: vec![],
        optional_dependencies: vec![],
        conflicts: vec![],
        provides: vec![],
    });
    resolver.add_candidate(PackageCandidate {
        name: "common".into(),
        version: Version::new(2, 0, 0),
        dependencies: vec![],
        optional_dependencies: vec![],
        conflicts: vec![],
        provides: vec![],
    });

    let result = resolver.resolve(&["app".into()]).unwrap();

    // should select common 2.0.0 (highest matching)
    assert_eq!(
        result.selected.get("common").unwrap(),
        &Version::new(2, 0, 0)
    );
    assert_eq!(result.install_order.len(), 4);

    // common must be installed before lib-a and lib-b
    let common_pos = result
        .install_order
        .iter()
        .position(|n| n == "common")
        .unwrap();
    let app_pos = result
        .install_order
        .iter()
        .position(|n| n == "app")
        .unwrap();
    assert!(common_pos < app_pos);
}

// ─── builder tests ─────────────────────────────────────────────────

#[test]
fn test_builder_from_example() {
    let tmp = TempDir::new().unwrap();
    let keypair = KeyPair::generate();

    // create a minimal package directory
    let pkg_dir = tmp.path().join("test-build");
    std::fs::create_dir_all(pkg_dir.join("files/usr/bin")).unwrap();
    std::fs::write(
        pkg_dir.join("files/usr/bin/test-app"),
        "#!/bin/sh\necho 'test'\n",
    )
    .unwrap();
    std::fs::write(
        pkg_dir.join("Astrafile.yaml"),
        r#"
name: test-build
version: "1.0.0"
architecture: x86_64
description: "Build test package"
maintainer: "Test <test@test.com>"
license: ZPL-2.0
dependencies: []
files_dir: files
"#,
    )
    .unwrap();

    let output_dir = tmp.path().join("output");
    let pkg_path = Builder::build(&pkg_dir, &keypair, &output_dir).unwrap();

    assert!(pkg_path.exists());
    assert!(pkg_path
        .to_string_lossy()
        .ends_with("test-build-1.0.0-x86_64.astpkg"));

    // verify the built package
    let package = PackageReader::read_from_file(&pkg_path).unwrap();
    assert_eq!(package.metadata.name, "test-build");
    assert_eq!(package.metadata.version, Version::new(1, 0, 0));
    assert!(package
        .files
        .contains_key(&PathBuf::from("usr/bin/test-app")));
    assert!(package.verify(&keypair.public_key()).is_ok());
}

// ─── crypto tests ──────────────────────────────────────────────────

#[test]
fn test_key_persistence() {
    let tmp = TempDir::new().unwrap();
    let keypair = KeyPair::generate();

    // save and load keypair
    let key_path = tmp.path().join("test.key");
    keypair.save_to_file(&key_path).unwrap();
    let loaded = KeyPair::load_from_file(&key_path).unwrap();

    // sign with original, verify with loaded
    let data = b"test data for signing";
    let sig = astra_crypto::sign_data(data, &keypair);
    assert!(astra_crypto::verify_signature(data, &sig, &loaded.public_key()).is_ok());

    // save and load public key
    let pub_path = tmp.path().join("test.pub");
    keypair.public_key().save_to_file(&pub_path).unwrap();
    let loaded_pub = astra_crypto::PublicKey::load_from_file(&pub_path).unwrap();
    assert!(astra_crypto::verify_signature(data, &sig, &loaded_pub).is_ok());
}

#[test]
fn test_keyring_persistence() {
    let tmp = TempDir::new().unwrap();
    let mut keyring = astra_crypto::KeyRing::new();

    let key1 = KeyPair::generate().public_key();
    let key2 = KeyPair::generate().public_key();

    keyring.add("key1".into(), key1);
    keyring.add("key2".into(), key2);

    let kr_path = tmp.path().join("keyring.json");
    keyring.save_to_file(&kr_path).unwrap();

    let loaded = astra_crypto::KeyRing::load_from_file(&kr_path).unwrap();
    assert_eq!(loaded.list().len(), 2);
    assert!(loaded.get("key1").is_some());
    assert!(loaded.get("key2").is_some());
}

// ─── repository index tests ────────────────────────────────────────

#[test]
fn test_repo_index_serialization() {
    let index = RepoIndex {
        name: "test-repo".into(),
        description: "Test repository".into(),
        last_updated: Utc::now().to_rfc3339(),
        packages: vec![RepoPackageEntry {
            name: "hello".into(),
            version: Version::new(1, 0, 0),
            architecture: "x86_64".into(),
            description: "Hello package".into(),
            dependencies: vec![],
            conflicts: vec![],
            provides: vec![],
            checksum: "abc123".into(),
            filename: "hello-1.0.0-x86_64.astpkg".into(),
            size: 4096,
            license: "ZPL-2.0".into(),
            maintainer: "Test <test@test.com>".into(),
        }],
    };

    let json = serde_json::to_string_pretty(&index).unwrap();
    let deserialized: RepoIndex = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.name, "test-repo");
    assert_eq!(deserialized.packages.len(), 1);
    assert_eq!(deserialized.packages[0].name, "hello");
}

#[test]
fn test_repo_index_search() {
    let index = RepoIndex {
        name: "test".into(),
        description: "".into(),
        last_updated: "".into(),
        packages: vec![
            RepoPackageEntry {
                name: "hello-world".into(),
                version: Version::new(1, 0, 0),
                architecture: "x86_64".into(),
                description: "A greeting program".into(),
                dependencies: vec![],
                conflicts: vec![],
                provides: vec![],
                checksum: "".into(),
                filename: "".into(),
                size: 0,
                license: "".into(),
                maintainer: "".into(),
            },
            RepoPackageEntry {
                name: "goodbye".into(),
                version: Version::new(1, 0, 0),
                architecture: "x86_64".into(),
                description: "A farewell program".into(),
                dependencies: vec![],
                conflicts: vec![],
                provides: vec![],
                checksum: "".into(),
                filename: "".into(),
                size: 0,
                license: "".into(),
                maintainer: "".into(),
            },
        ],
    };

    let results = index.search("hello");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "hello-world");

    let results = index.search("program");
    assert_eq!(results.len(), 2);
}

// ─── full lifecycle test ───────────────────────────────────────────

#[test]
fn test_full_lifecycle_local() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("root");
    let data_dir = tmp.path().join("data");
    std::fs::create_dir_all(&root).unwrap();

    let config = AstraConfig {
        root: root.clone(),
        data_dir: data_dir.clone(),
        cache_dir: data_dir.join("cache"),
        repositories: vec![],
    };

    // initialize
    let mut mgr = PackageManager::init(config).unwrap();

    // generate keys
    let keypair = mgr.generate_keypair().unwrap();
    let pubkey = keypair.public_key();
    mgr.import_key("test", pubkey).unwrap();

    // build a package
    let pkg_dir = tmp.path().join("hello-pkg");
    std::fs::create_dir_all(pkg_dir.join("files/usr/bin")).unwrap();
    std::fs::write(
        pkg_dir.join("files/usr/bin/hello"),
        "#!/bin/sh\necho hello\n",
    )
    .unwrap();
    std::fs::write(
        pkg_dir.join("Astrafile.yaml"),
        r#"
name: hello
version: "1.0.0"
architecture: x86_64
description: "Hello world"
maintainer: "Test <test@test.com>"
license: ZPL-2.0
"#,
    )
    .unwrap();

    let output_dir = tmp.path().join("packages");
    let pkg_path = mgr.build(&pkg_dir, &output_dir).unwrap();
    assert!(pkg_path.exists());

    // install locally
    let name = mgr.install_local(&pkg_path, false).unwrap();
    assert_eq!(name, "hello");

    // verify it's installed
    assert!(mgr.db().is_installed("hello").unwrap());

    // check files exist
    assert!(root.join("usr/bin/hello").exists());

    // verify integrity
    let issues = mgr.verify_installed("hello").unwrap();
    assert!(issues.is_empty());

    // list
    let packages = mgr.db().list_packages().unwrap();
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0].name, "hello");

    // remove
    let files = mgr.remove("hello").unwrap();
    assert!(!files.is_empty());
    assert!(!mgr.db().is_installed("hello").unwrap());
}
