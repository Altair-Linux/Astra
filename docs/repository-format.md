# Astra Repository Format Specification

## Overview

An Astra repository is a static directory served over HTTP. It contains a
package index, package files, and signature files.

## Structure

```
repo/
├── index.json         Package index
├── packages/          Package files (.astpkg)
│   ├── hello-1.0.0-x86_64.astpkg
│   └── world-2.1.0-x86_64.astpkg
└── signatures/        Detached signatures
    ├── hello-1.0.0-x86_64.astpkg.sig
    └── world-2.1.0-x86_64.astpkg.sig
```

## index.json

```json
{
  "name": "altair-main",
  "description": "Main Altair Linux repository",
  "last_updated": "2026-01-01T00:00:00Z",
  "packages": [
    {
      "name": "hello",
      "version": "1.0.0",
      "architecture": "x86_64",
      "description": "A hello world package",
      "dependencies": [],
      "conflicts": [],
      "provides": [],
      "checksum": "sha256_hex_of_package_file",
      "filename": "hello-1.0.0-x86_64.astpkg",
      "size": 4096,
      "license": "ZPL-2.0",
      "maintainer": "Altair <contact@altairlinux.org>"
    }
  ]
}
```

### Index Fields

| Field           | Type   | Description                         |
|-----------------|--------|-------------------------------------|
| `name`          | string | Repository name                     |
| `description`   | string | Repository description              |
| `last_updated`  | string | ISO 8601 last update timestamp      |
| `packages`      | array  | List of package entries             |

### Package Entry Fields

| Field           | Type    | Description                        |
|-----------------|---------|------------------------------------|
| `name`          | string  | Package name                       |
| `version`       | string  | Package version (semver)           |
| `architecture`  | string  | Target architecture                |
| `description`   | string  | Package description                |
| `dependencies`  | array   | Dependency list                    |
| `conflicts`     | array   | Conflicting package names          |
| `provides`      | array   | Virtual packages provided          |
| `checksum`      | string  | SHA-256 hex hash of package file   |
| `filename`      | string  | Filename in packages/ directory    |
| `size`          | integer | File size in bytes                 |
| `license`       | string  | License identifier                 |
| `maintainer`    | string  | Maintainer information             |

## Hosting

### Development Server

Astra includes a built-in development server:

```bash
astra serve-repo ./my-repo --bind 0.0.0.0:8080
```

### Production Hosting

Any HTTP server can host a repository. Simply serve the repository
directory as static files.

Example with nginx:

```nginx
server {
    listen 80;
    server_name repo.altairlinux.org;
    root /srv/astra-repo;
    autoindex off;

    location / {
        try_files $uri =404;
    }
}
```

## Creating a Repository

```bash
mkdir -p my-repo/packages my-repo/signatures

# Build and place packages
astra build ./my-package -o my-repo/packages/

# Generate index (future: astra repo generate-index)
# For now, create index.json manually or with a script

# Serve
astra serve-repo ./my-repo
```

## Updating a Repository

When adding or removing packages:

1. Add/remove package files in `packages/`
2. Add/remove corresponding signature files in `signatures/`
3. Regenerate `index.json`

## Mirroring

Repositories can be mirrored by copying the entire directory structure
to another HTTP server. The index references packages by relative paths,
so mirrors work without modification.
