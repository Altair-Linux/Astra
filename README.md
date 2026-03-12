# Astra Package Manager

[![Build Status](https://github.com/Altair-Linux/Astra/actions/workflows/ci.yml/badge.svg)](https://github.com/Altair-Linux/Astra/actions)
[![License: ZPL 2.0](https://img.shields.io/badge/License-ZPL%202.0-blue.svg)](LICENSE)

**Astra** is a modern, secure, high-performance package manager for
[Altair Linux](https://github.com/Altair-Linux). Written in Rust, it provides
deterministic builds, Ed25519 cryptographic verification, and a clean CLI
experience.

## Features

- **Secure by default** — Ed25519 signed packages; unsigned packages are rejected
- **Deterministic** — Reproducible package builds with sorted archives
- **Fast** — Zstd-compressed packages, efficient dependency resolution
- **Self-contained** — No dependency on apt, dnf, pacman, or any other package manager
- **Developer-friendly** — JSON output mode, excellent error messages, simple recipe format

## Quick Start

### Install from Source

```bash
# Clone the repository
git clone https://github.com/Altair-Linux/Astra.git
cd Astra

# Build
cargo build --release

# The binary is at target/release/astra
sudo cp target/release/astra /usr/local/bin/
```

### Initialize

```bash
sudo astra init
```

### Add a Repository

```bash
sudo astra repo add myrepo http://repo.example.com/
sudo astra update
```

### Install a Package

```bash
sudo astra install hello
```

### Build a Package

```bash
# Create a package directory with an Astrafile.yaml
astra key generate
astra build ./my-package -o ./output
```

## Architecture

```
Astra/
├── cmd/astra/       CLI entrypoint
├── core/            Package management orchestration
├── resolver/        Dependency resolver
├── db/              SQLite local package database
├── pkg/             Package format (.astpkg) implementation
├── repo/            Repository client
├── repo-server/     HTTP repository server
├── builder/         Package builder (Astrafile.yaml → .astpkg)
├── crypto/          Ed25519 signing and verification
├── docs/            Documentation
├── examples/        Example packages and repositories
└── tests/           Integration tests
```

## Package Format

Astra packages use the `.astpkg` extension. Each package is a **tar archive
compressed with zstd** containing:

| File             | Description                    |
|------------------|--------------------------------|
| `metadata.json`  | Package metadata and checksums |
| `files/`         | Installed filesystem files     |
| `scripts/`       | Install/remove scripts         |
| `signature`      | Ed25519 signature              |

See [docs/package-format.md](docs/package-format.md) for the full specification.

## CLI Reference

| Command                          | Description                        |
|----------------------------------|------------------------------------|
| `astra init`                     | Initialize the Astra system        |
| `astra repo add <name> <url>`    | Add a repository                   |
| `astra repo remove <name>`       | Remove a repository                |
| `astra repo list`                | List repositories                  |
| `astra update`                   | Fetch repository indices           |
| `astra search <query>`           | Search for packages                |
| `astra info <package>`           | Show package details               |
| `astra install <package...>`     | Install packages                   |
| `astra remove <package...>`      | Remove packages                    |
| `astra upgrade`                  | Upgrade all packages               |
| `astra list`                     | List installed packages            |
| `astra verify <package>`         | Verify package integrity           |
| `astra build <directory>`        | Build a package                    |
| `astra serve-repo <directory>`   | Serve a repository over HTTP       |
| `astra key generate`             | Generate signing key pair          |
| `astra key import <name> <path>` | Import a public key                |
| `astra key export`               | Export the public key              |
| `astra key list`                 | List trusted keys                  |

All commands support `--json` for machine-readable output and `--verbose` for
debug logging.

## Recipe Format (Astrafile.yaml)

```yaml
name: hello
version: "1.0.0"
architecture: x86_64
description: A simple hello world package
maintainer: "Your Name <you@example.com>"
license: ZPL-2.0
dependencies:
  - name: glibc
    version: ">=2.35"
provides: []
conflicts: []
files_dir: files
scripts:
  post_install: "echo 'Hello installed!'"
```

## Repository Format

Repositories are static HTTP servers with the following structure:

```
repo/
├── index.json        Package index
├── packages/         Package files (.astpkg)
└── signatures/       Signature files (.astpkg.sig)
```

See [docs/repository-format.md](docs/repository-format.md) for details.

## Security

- All packages **must** be signed with Ed25519
- Package checksums are verified before installation
- File integrity can be verified post-installation with `astra verify`
- The keyring stores trusted public keys

See [docs/security.md](docs/security.md) for the full security model.

## Development

```bash
# Run all tests
cargo test --workspace

# Run with verbose logging
RUST_LOG=debug cargo run -- --verbose list

# Build for release
cargo build --release
```

## License

This project is licensed under the **Zorvia Public License v2.0 (ZPL 2.0)**.
See [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome. Please see [docs/contributing.md](docs/contributing.md)
for guidelines.
