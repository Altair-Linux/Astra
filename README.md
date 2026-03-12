
---

# ![Astra Logo](https://github.com/Altair-Linux/Astra/blob/main/docs/logo.svg) Astra Package Manager

[![CI](https://github.com/Altair-Linux/Astra/actions/workflows/ci.yml/badge.svg)](https://github.com/Altair-Linux/Astra/actions)
[![License](https://img.shields.io/badge/License-ZPL%202.0-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-1.0.0-blueviolet)]()
[![Rust](https://img.shields.io/badge/made%20with-Rust-orange.svg)]()

**Astra** is the official package manager for Altair Linux.
It provides **deterministic builds, Ed25519-signed packages**, and **high-performance dependency resolution**.

---

<details open>
<summary>Overview</summary>

Astra manages the installation, removal, upgrading, and verification of packages on Altair Linux systems.

**Design goals:**

* Deterministic, reproducible builds
* Strong cryptographic verification
* High performance, minimal resource usage
* Clean CLI for automation and scripting
* Fully self-contained, independent of other package managers

</details>

---

<details>
<summary>Features</summary>

<details>
<summary>Security</summary>
- Mandatory Ed25519 signing for all packages  
- Unsigned packages rejected  
- Post-install verification with `astra verify`
</details>

<details>
<summary>Performance</summary>
- Zstd compression for packages  
- Optimized dependency resolver  
- Minimal system overhead
</details>

<details>
<summary>Developer-Friendly</summary>
- JSON output mode for automation  
- Simple YAML package recipes (`Astrafile.yaml`)  
- Structured error messages
</details>

</details>

---

<details>
<summary>Quick Start</summary>

<mermade-tabs>
<mermade-tab label="Bash">
```bash id="quickstart-bash"
git clone https://github.com/Altair-Linux/Astra.git
cd Astra
cargo build --release
sudo install -m755 target/release/astra /usr/local/bin/astra

sudo astra init
sudo astra repo add myrepo [http://repo.example.com/](http://repo.example.com/)
sudo astra update
sudo astra install hello

````
</mermade-tab>

<mermade-tab label="Rust">
```rust id="quickstart-rust"
// Build Astra from source
use std::process::Command;

Command::new("cargo")
    .args(&["build", "--release"])
    .status()
    .expect("Failed to build Astra");
````

</mermade-tab>

<mermade-tab label="YAML">
```yaml id="quickstart-yaml"
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
</mermade-tab>
</mermade-tabs>

</details>

---

<details>
<summary>Architecture</summary>

```text id="architecture-text"
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

### Animated Flow Diagram

```html id="architecture-svg"
<svg width="100%" height="250" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 250">

  <!-- Nodes -->
  <rect x="50" y="30" width="120" height="50" fill="#5C6BC0" rx="6"/>
  <text x="110" y="60" font-size="14" fill="#fff" text-anchor="middle">CLI</text>

  <rect x="220" y="30" width="120" height="50" fill="#42A5F5" rx="6"/>
  <text x="280" y="60" font-size="14" fill="#fff" text-anchor="middle">Resolver</text>

  <rect x="390" y="30" width="120" height="50" fill="#26A69A" rx="6"/>
  <text x="450" y="60" font-size="14" fill="#fff" text-anchor="middle">Database</text>

  <rect x="220" y="130" width="120" height="50" fill="#FFB74D" rx="6"/>
  <text x="280" y="160" font-size="14" fill="#fff" text-anchor="middle">Repo Client</text>

  <rect x="390" y="130" width="120" height="50" fill="#FF7043" rx="6"/>
  <text x="450" y="160" font-size="14" fill="#fff" text-anchor="middle">Package Builder</text>

  <!-- Animated Arrows -->
  <line x1="170" y1="55" x2="220" y2="55" stroke="#000" stroke-width="2">
    <animate attributeName="x2" values="170;220" dur="1s" repeatCount="indefinite"/>
  </line>
  <line x1="340" y1="55" x2="390" y2="55" stroke="#000" stroke-width="2">
    <animate attributeName="x2" values="340;390" dur="1s" repeatCount="indefinite"/>
  </line>
  <line x1="280" y1="80" x2="280" y2="130" stroke="#000" stroke-width="2">
    <animate attributeName="y2" values="80;130" dur="1s" repeatCount="indefinite"/>
  </line>
  <line x1="340" y1="155" x2="390" y2="155" stroke="#000" stroke-width="2">
    <animate attributeName="x2" values="340;390" dur="1s" repeatCount="indefinite"/>
  </line>
</svg>
```

</details>

---

<details>
<summary>Package Format (.astpkg)</summary>

| File          | Description                     |
| ------------- | ------------------------------- |
| metadata.json | Package metadata & checksums    |
| files/        | Files to install                |
| scripts/      | Pre/post-install/remove scripts |
| signature     | Ed25519 signature               |

See [docs/package-format.md](docs/package-format.md)

</details>

---

<details>
<summary>CLI Reference</summary>

<details>
<summary>Repository Management</summary>
```bash id="repo-cli"
astra repo add <name> <url>
astra repo remove <name>
astra repo list
astra update
```
</details>

<details>
<summary>Package Management</summary>
```bash id="pkg-cli"
astra search <query>
astra info <package>
astra install <package...>
astra remove <package...>
astra upgrade
astra list
astra verify <package>
```
</details>

<details>
<summary>Developer Commands</summary>
```bash id="dev-cli"
astra build <directory>
astra serve-repo <directory>
astra key generate
astra key import <name> <path>
astra key export
astra key list
```
</details>

</details>

---

<details>
<summary>Repository Format</summary>

```text id="repo-format"
repo/
├── index.json        Package index
├── packages/         Package files (.astpkg)
└── signatures/       Signature files (.astpkg.sig)
```

See [docs/repository-format.md](docs/repository-format.md)

</details>

---

<details>
<summary>Security</summary>

* All packages must be signed (Ed25519)
* Checksums verified pre-installation
* Keyring stores trusted public keys
* Post-install verification with `astra verify`

See [docs/security.md](docs/security.md)

</details>

---

<details>
<summary>Development</summary>

<mermade-tabs>
<mermade-tab label="Run Tests">
```bash id="dev-tests"
cargo test --workspace
```
</mermade-tab>

<mermade-tab label="Verbose Run">
```bash id="dev-verbose"
RUST_LOG=debug cargo run -- --verbose list
```
</mermade-tab>

<mermade-tab label="Build Release">
```bash id="dev-release"
cargo build --release
```
</mermade-tab>
</mermade-tabs>

</details>

---

<details>
<summary>License & Contribution</summary>

Licensed under **Zorvia Public License v2.0 (ZPL 2.0)** — [LICENSE](LICENSE)

Contribution guidelines: [docs/contributing.md](docs/contributing.md)

</details>

---



Do you want me to add that next?
