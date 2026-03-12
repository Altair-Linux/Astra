# Contributing to Astra

Thank you for your interest in contributing to the Astra Package Manager!

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/your-username/Astra.git`
3. Create a feature branch: `git checkout -b feature/my-feature`
4. Make your changes
5. Run tests: `cargo test --workspace`
6. Commit: `git commit -m "Add my feature"`
7. Push: `git push origin feature/my-feature`
8. Open a Pull Request

## Development Setup

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs))
- Git

### Building

```bash
cargo build --workspace
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p astra-crypto
cargo test -p astra-resolver

# Run with output
cargo test --workspace -- --nocapture
```

### Code Style

- Follow standard Rust formatting (`cargo fmt`)
- Pass all clippy lints (`cargo clippy --workspace`)
- Write documentation for public APIs
- Include tests for new functionality

## Architecture

The project is organized as a Cargo workspace with these crates:

| Crate             | Purpose                              |
|-------------------|--------------------------------------|
| `astra`           | CLI binary                           |
| `astra-core`      | Package management orchestration     |
| `astra-crypto`    | Ed25519 signing and verification     |
| `astra-pkg`       | Package format (.astpkg)             |
| `astra-db`        | SQLite local database                |
| `astra-resolver`  | Dependency resolution                |
| `astra-repo`      | Repository client                    |
| `astra-repo-server` | HTTP repository server             |
| `astra-builder`   | Package builder                      |

## Pull Request Guidelines

- Keep PRs focused on a single change
- Include tests for bug fixes and new features
- Update documentation if needed
- Ensure CI passes

## Reporting Issues

- Use GitHub Issues
- Include steps to reproduce
- Include expected vs actual behavior
- Include Astra version (`astra --version`)

## License

By contributing, you agree that your contributions will be licensed under
the Zorvia Public License v2.0 (ZPL 2.0).
