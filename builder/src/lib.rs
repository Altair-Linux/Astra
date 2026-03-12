//! # Astra Package Builder
//!
//! Reads an `Astrafile.yaml` recipe, assembles the filesystem layout,
//! generates metadata, creates the `.astpkg` archive, and signs it.

mod builder;
mod error;
mod recipe;

pub use builder::Builder;
pub use error::BuildError;
pub use recipe::Recipe;
