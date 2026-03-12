# Astra Package Format Specification

## Overview

Astra packages use the `.astpkg` file extension. Each package is a **tar
archive compressed with zstd** (level 3 by default).

## Structure

```
package.astpkg (tar+zstd)
├── metadata.json    Package metadata
├── files/           Installed files (filesystem tree)
│   ├── usr/
│   │   ├── bin/
│   │   │   └── myapp
│   │   └── share/
│   │       └── doc/
│   │           └── myapp/
│   │               └── README
│   └── etc/
│       └── myapp.conf
├── scripts/         Install/remove hooks
│   ├── pre_install.sh
│   ├── post_install.sh
│   ├── pre_remove.sh
│   ├── post_remove.sh
│   ├── pre_upgrade.sh
│   └── post_upgrade.sh
└── signature        Ed25519 signature (64 bytes)
```

## metadata.json

```json
{
  "name": "myapp",
  "version": "1.2.3",
  "architecture": "x86_64",
  "description": "My application",
  "dependencies": [
    { "name": "glibc", "version_req": ">=2.35" },
    { "name": "openssl" }
  ],
  "optional_dependencies": [],
  "conflicts": ["myapp-legacy"],
  "provides": ["myapp-bin"],
  "maintainer": "Author <author@example.com>",
  "license": "ZPL-2.0",
  "build_date": "2026-01-01T00:00:00Z",
  "checksums": {
    "usr/bin/myapp": {
      "sha256": "abcdef1234567890...",
      "size": 1048576
    }
  },
  "installed_size": 1048576
}
```

### Fields

| Field                  | Type     | Required | Description                              |
|------------------------|----------|----------|------------------------------------------|
| `name`                 | string   | yes      | Package name (no whitespace)             |
| `version`              | string   | yes      | Semantic version (e.g., "1.2.3")         |
| `architecture`         | string   | yes      | Target arch: x86_64, aarch64, any        |
| `description`          | string   | yes      | Human-readable description               |
| `dependencies`         | array    | no       | Required dependencies                    |
| `optional_dependencies`| array    | no       | Optional dependencies                    |
| `conflicts`            | array    | no       | Conflicting package names                |
| `provides`             | array    | no       | Virtual package names provided           |
| `maintainer`           | string   | yes      | Maintainer name and email                |
| `license`              | string   | yes      | SPDX license identifier                  |
| `build_date`           | string   | yes      | ISO 8601 build timestamp                 |
| `checksums`            | object   | no       | SHA-256 checksums of included files      |
| `installed_size`       | integer  | no       | Total installed size in bytes            |

### Dependency Format

```json
{
  "name": "package-name",
  "version_req": ">=1.0.0"
}
```

The `version_req` field uses semver range syntax:
- `>=1.0.0` — at least version 1.0.0
- `^1.2` — compatible with 1.2.x
- `=1.0.0` — exactly version 1.0.0
- omitted — any version

## Files Directory

The `files/` directory mirrors the target filesystem. Files are extracted
relative to the system root (`/`).

For example, `files/usr/bin/myapp` installs to `/usr/bin/myapp`.

## Scripts

Scripts are shell scripts executed at various lifecycle stages:

| Script              | When executed                              |
|---------------------|--------------------------------------------|
| `pre_install.sh`    | Before files are extracted                 |
| `post_install.sh`   | After files are extracted                  |
| `pre_remove.sh`     | Before files are removed                  |
| `post_remove.sh`    | After files are removed                   |
| `pre_upgrade.sh`    | Before upgrade (old version still present) |
| `post_upgrade.sh`   | After upgrade (new version installed)      |

Scripts must be valid POSIX shell and must have a shebang line.

## Signature

The signature file contains a 64-byte Ed25519 signature computed over the
SHA-256 hash of:

1. The JSON-serialized metadata
2. All file paths and contents (sorted by path for determinism)
3. All script names and contents (sorted by name)

Unsigned packages are rejected by default.

## Determinism

To ensure reproducible builds:

- Files in the tar archive are sorted alphabetically by path
- Scripts are sorted by their type name
- Metadata JSON uses consistent field ordering via serde
- Timestamps in tar headers are set to a fixed value
- Zstd compression uses a fixed level (3)
