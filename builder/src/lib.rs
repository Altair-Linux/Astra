//! # astra package builder
//!
//! reads an `Astrafile.yaml` recipe, collects the files,
//! builds metadata, creates the `.astpkg` archive, and signs it.

mod builder;
mod error;
mod recipe;

pub use builder::Builder;
pub use error::BuildError;
pub use recipe::Recipe;
