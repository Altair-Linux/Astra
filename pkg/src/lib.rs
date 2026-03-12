//! # astra package format
//!
//! defines the `.astpkg` file format — a tar archive compressed with zstd
//! that holds metadata, files, scripts, and a cryptographic signature.

mod error;
mod metadata;
mod package;

pub use error::PackageError;
pub use metadata::{Checksum, Dependency, Metadata, ScriptType};
pub use package::{Package, PackageReader, PackageWriter};
