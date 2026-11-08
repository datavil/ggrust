//! ggrust-core: the plotting engine behind the `ggrust` Python package.
//!
//! Pure Rust, no Python. Everything here is testable with `cargo test`.
//! The Python bindings live in the `ggrust-python` crate.

pub mod build;
pub mod cluster;
pub mod coord;
pub mod data;
pub mod error;
pub mod facet;
pub mod geom;
pub mod grid;
pub mod grid_axes;
pub mod grid_share;
pub mod grid_tags;
pub mod interactive_legend;
pub mod layout;
pub mod parallel;
pub mod parameters;
pub mod position;
pub mod render;
pub mod scale;
pub mod scene;
pub mod spec;
pub mod stat;
pub mod streamplot;
pub mod style;
pub mod suggest;
pub mod text;
pub mod theme;
pub mod tooltip;

pub use error::{Error, Result};

/// Version of the engine, taken from the workspace manifest.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
