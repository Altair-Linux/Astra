# Astra Security Model

## Overview

Astra implements a security-first design where every package must be
cryptographically signed and verified before installation.

## Threat Model

Astra protects against:

1. **Tampered packages** — Modified packages are detected via checksums
   and signatures
2. **Unauthorized packages** — Only packages signed by trusted keys are
   accepted
3. **Rollback attacks** — Version comparison prevents downgrade
4. **Man-in-the-middle** — Checksums verified against repository index;
   signatures verified against trusted keys

## Cryptographic Primitives

| Primitive     | Algorithm    | Use                        |
|---------------|-------------|----------------------------|
| Signing       | Ed25519     | Package signatures         |
| Hashing       | SHA-256     | Package and file checksums |
| Encoding      | Base64      | Key import/export          |

## Verification Pipeline

When installing a package from a repository:

```
1. Fetch index.json from repository
2. Download package file
3. Verify SHA-256 checksum matches index entry
4. Read package archive
5. Verify Ed25519 signature against keyring
6. Extract files to filesystem
7. Record in database
```

If any step fails, the installation is aborted immediately.

## Key Management

### Key Generation

```bash
astra key generate
```

Generates an Ed25519 key pair:
- **Private key**: Stored at `/var/lib/astra/signing.key` (base64-encoded)
- **Public key**: Displayed and can be exported

### Key Import

```bash
astra key import mykey /path/to/public.key
```

Adds a public key to the trusted keyring at `/var/lib/astra/keyring.json`.

### Key Export

```bash
astra key export -o /path/to/public.key
```

### Keyring

The keyring is a JSON file mapping key names to public keys. A package
is accepted if its signature validates against **any** key in the keyring.

## Package Signing

Packages are signed during the build process:

```bash
astra build ./my-package -o ./output
```

The signature covers:
1. Serialized metadata JSON
2. All file paths and contents (sorted for determinism)
3. All script names and contents (sorted)

This ensures that any modification to the package content, metadata, or
scripts will invalidate the signature.

## Unsigned Package Policy

By default, unsigned packages are **rejected**. This cannot be overridden
through the CLI. The only exception is local installation during development
where `--local` flag is used with a trusted package file.

## File Integrity Verification

After installation, file integrity can be verified:

```bash
astra verify mypackage
```

This checks:
- All installed files still exist
- SHA-256 checksums match the recorded values

## Best Practices

1. **Protect your private key** — The signing key should be stored securely
   and backed up
2. **Rotate keys periodically** — Generate new keys and transition to them
3. **Use separate keys** — Use different keys for different repositories
4. **Verify before trusting** — Only import keys from trusted sources
5. **Review packages** — Always review package contents before signing
