//! ggrust-core: the plotting engine behind the `ggrust` Python package.
//!
//! Pure Rust, no Python. Everything here is testable with `cargo test`.
//! The Python bindings live in the `ggrust-python` crate.

pub mod error;
pub mod spec;

pub use error::{Error, Result};

/// Version of the engine, taken from the workspace manifest.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
