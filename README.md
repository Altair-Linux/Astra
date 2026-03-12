
# Astra Package Manager

[![CI](https://github.com/Altair-Linux/Astra/actions/workflows/ci.yml/badge.svg)](https://github.com/Altair-Linux/Astra/actions)
[![License](https://img.shields.io/badge/License-ZPL%202.0-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-1.0.0-blueviolet)]()
[![Rust](https://img.shields.io/badge/made%20with-Rust-orange.svg)]()

**Astra** is the official package manager for **Altair Linux**, designed for deterministic builds, strong cryptographic verification, and high-performance package management.

---

## Overview

Astra manages the installation, removal, upgrading, and verification of software packages on Altair Linux systems. Its design emphasizes reliability, reproducibility, and security.

**Key Goals:**

- Deterministic, reproducible builds  
- Mandatory Ed25519 cryptographic signing for all packages  
- Optimized dependency resolution for speed and efficiency  
- Simple, automation-friendly command-line interface  
- Fully self-contained; independent of other package managers  

---

## Features

### Security

- All packages must be signed with Ed25519  
- Unsigned packages are rejected automatically  
- Post-install verification via `astra verify`  

### Performance

- Zstd compression for fast package transfer  
- Optimized resolver to minimize system overhead  
- Minimal runtime footprint  

### Developer Tools

- JSON output for automation and scripting  
- YAML-based package recipes (`Astrafile.yaml`)  
- Clear, structured error messages  

---

## Quick Start

### Install from Source

```bash
git clone https://github.com/Altair-Linux/Astra.git
cd Astra
cargo build --release
sudo install -m755 target/release/astra /usr/local/bin/astra
````

### Initialize Astra

```bash
sudo astra init
sudo astra repo add myrepo http://repo.example.com/
sudo astra update
sudo astra install hello
```

### Example Package Recipe (`Astrafile.yaml`)

```yaml
name: hello
version: "1.0.0"
architecture: x86_64
description: Hello World package
maintainer: "Your Name <you@example.com>"
license: ZPL-2.0
dependencies:
  - name: glibc
    version: ">=2.35"
files_dir: files
scripts:
  post_install: "echo 'Hello installed!'"
```

---

## Architecture

```
Astra/
├── cmd/astra/       CLI entrypoint
├── core/            Orchestration engine
├── resolver/        Dependency resolver
├── db/              SQLite database
├── pkg/             Package handler (.astpkg)
├── repo/            Repository client
├── repo-server/     HTTP repository server
├── builder/         Package builder
├── crypto/          Ed25519 signing & verification
├── docs/            Documentation
├── examples/        Sample packages
└── tests/           Integration tests
```

---

## Package Format (.astpkg)

| File          | Description                     |
| ------------- | ------------------------------- |
| metadata.json | Package metadata & checksums    |
| files/        | Files to install                |
| scripts/      | Pre/post-install/remove scripts |
| signature     | Ed25519 signature               |

For full details, see [docs/package-format.md](docs/package-format.md).

---

## CLI Reference

### Repository Management

```bash
astra repo add <name> <url>
astra repo remove <name>
astra repo list
astra update
```

### Package Management

```bash
astra search <query>
astra info <package>
astra install <package...>
astra remove <package...>
astra upgrade
astra list
astra verify <package>
```

### Developer Commands

```bash
astra build <directory>
astra serve-repo <directory>
astra key generate
astra key import <name> <path>
astra key export
astra key list
```

---

## Repository Layout

```
repo/
├── index.json        Package index
├── packages/         Package files (.astpkg)
└── signatures/       Signature files (.astpkg.sig)
```

See [docs/repository-format.md](docs/repository-format.md).

---

## Security

* Mandatory Ed25519 signatures for all packages
* Checksums verified prior to installation
* Trusted keys stored in a keyring
* Post-install verification with `astra verify`

See [docs/security.md](docs/security.md).

---

## Development

```bash
# Run tests for the entire workspace
cargo test --workspace

# Verbose run
RUST_LOG=debug cargo run -- --verbose list

# Build release binary
cargo build --release
```

---

## License & Contribution

Astra is licensed under the **Zorvia Public License v2.0 (ZPL 2.0)** — see [LICENSE](LICENSE)
Contribution guidelines: [docs/contributing.md](docs/contributing.md)
