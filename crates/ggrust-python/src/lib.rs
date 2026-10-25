//! Python extension module `ggrust._ggrust`.
//!
//! Only conversion and dispatch live here. Engine logic belongs in `ggrust-core`.

use pyo3::prelude::*;

/// Native extension module of ggrust.
#[pymodule]
mod _ggrust {
    use super::*;

    /// Return the version of the Rust engine.
    #[pyfunction]
    fn engine_version() -> &'static str {
        ggrust_core::VERSION
    }
}
