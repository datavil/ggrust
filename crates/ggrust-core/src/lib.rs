//! ggrust-core: the plotting engine behind the `ggrust` Python package.
//!
//! Pure Rust, no Python. Everything here is testable with `cargo test`.
//! The Python bindings live in the `ggrust-python` crate.

pub mod data;
pub mod error;
pub mod render;
pub mod scene;
pub mod spec;
pub mod text;

pub use error::{Error, Result};

/// Version of the engine, taken from the workspace manifest.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
