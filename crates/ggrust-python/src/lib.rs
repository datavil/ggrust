//! Python extension module `ggrust._ggrust`.
//!
//! Only conversion and dispatch live here. Engine logic belongs in `ggrust-core`.

use pyo3::prelude::*;

/// Native extension module of ggrust.
#[pymodule]
mod _ggrust {
    use super::*;
    use ggrust_core::data::{Column, DataFrame};
    use pyo3::exceptions::PyValueError;
    use pyo3::types::PyDict;
    use pyo3_arrow::input::AnyRecordBatch;

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
            .map_err(to_value_error)
    }

    /// Describe how ggrust reads each column of an Arrow table (debugging aid).
    ///
    /// Accepts any object exporting `__arrow_c_stream__` or `__arrow_c_array__`
    /// (polars, pyarrow, pandas >= 2.2). Returns one dict per column with keys
    /// `name`, `kind` (`continuous` | `discrete`), `length`, `null_count`, and
    /// `levels` (discrete only).
    #[pyfunction]
    fn describe_frame<'py>(
        python: Python<'py>,
        data: AnyRecordBatch,
    ) -> PyResult<Vec<Bound<'py, PyDict>>> {
        let (batches, schema) = data.into_table()?.into_inner();
        let frame = DataFrame::new(schema, batches).map_err(to_value_error)?;
        let names: Vec<String> = frame.column_names().map(str::to_owned).collect();
        names
            .into_iter()
            .map(|name| {
                let column = frame.column(&name).map_err(to_value_error)?;
                let summary = PyDict::new(python);
                summary.set_item("name", &name)?;
                summary.set_item("length", column.len())?;
                match &column {
                    Column::Continuous(continuous) => {
                        summary.set_item("kind", "continuous")?;
                        summary.set_item("null_count", continuous.null_count())?;
                    }
                    Column::Discrete(discrete) => {
                        summary.set_item("kind", "discrete")?;
                        summary.set_item("null_count", discrete.null_count())?;
                        summary.set_item("levels", discrete.levels().to_vec())?;
                    }
                }
                Ok(summary)
            })
            .collect()
    }

    fn to_value_error(error: ggrust_core::Error) -> PyErr {
        PyValueError::new_err(error.to_string())
    }
}
