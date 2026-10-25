//! Python extension module `ggrust._ggrust`.
//!
//! Only conversion and dispatch live here. Engine logic belongs in `ggrust-core`.

use pyo3::prelude::*;

/// Native extension module of ggrust.
#[pymodule]
mod _ggrust {
    use super::*;
    use pyo3::exceptions::PyValueError;

    /// Return the version of the Rust engine.
    #[pyfunction]
    fn engine_version() -> &'static str {
        ggrust_core::VERSION
    }

    /// Parse a plot spec (JSON of `as_dict()` without data) and return the
    /// paths of options ggrust does not support.
    ///
    /// Raises `ValueError` naming the offending path if the spec is invalid.
    #[pyfunction]
    fn validate_spec(spec_json: &str) -> PyResult<Vec<String>> {
        ggrust_core::spec::Figure::from_json(spec_json)
            .map(|figure| figure.unsupported_options())
            .map_err(|error| PyValueError::new_err(error.to_string()))
    }
}
