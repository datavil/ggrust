//! Python extension module `ggrust._ggrust`.
//!
//! Only conversion and dispatch live here. Engine logic belongs in `ggrust-core`.

use pyo3::prelude::*;

/// Native extension module of ggrust.
#[pymodule]
mod _ggrust {
    use super::*;
    use ggrust_core::data::{Column, DataFrame};
    use ggrust_core::grid::build_figure;
    use ggrust_core::render::{HtmlDocument, HtmlOptions, to_html, to_png, to_svg};
    use ggrust_core::spec::Figure;
    use pyo3::exceptions::PyValueError;
    use pyo3::types::{PyBytes, PyDict};
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
    /// `name`, `kind` (`continuous` | `discrete` | `unsupported`), and then
    /// `length` + `null_count` (+ `levels` for discrete) or `error` (unsupported).
    #[pyfunction]
    fn describe_frame<'py>(
        python: Python<'py>,
        data: AnyRecordBatch,
    ) -> PyResult<Vec<Bound<'py, PyDict>>> {
        let frame = to_frame(data)?;
        let names: Vec<String> = frame.column_names().map(str::to_owned).collect();
        names
            .into_iter()
            .map(|name| {
                let summary = PyDict::new(python);
                summary.set_item("name", &name)?;
                let column = match frame.column(&name) {
                    Ok(column) => column,
                    Err(error) => {
                        summary.set_item("kind", "unsupported")?;
                        summary.set_item("error", error.to_string())?;
                        return Ok(summary);
                    }
                };
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

    /// Column names a spec can read (mappings, `order_by`, facets, tooltips),
    /// sorted. Only these need to be exported as Arrow.
    ///
    /// Raises `ValueError` naming the offending path if the spec is invalid.
    #[pyfunction]
    fn referenced_columns(spec_json: &str) -> PyResult<Vec<String>> {
        Figure::from_json(spec_json)
            .map(|figure| figure.referenced_columns().into_iter().collect())
            .map_err(to_value_error)
    }

    fn to_value_error(error: ggrust_core::Error) -> PyErr {
        PyValueError::new_err(error.to_string())
    }

    /// Build and render a figure (a plot, or a `gggrid` of figures).
    ///
    /// `spec_json` is the plot spec whose `data` keys are frame references
    /// (`{"frame": i}`); `frames` are the Arrow tables they point to (any object
    /// exporting `__arrow_c_stream__`).
    /// `output` is `"svg"`, `"png"`, `"html"` (notebook fragment), `"html_page"`
    /// or `"html_iframe"`; `scale` multiplies PNG pixel size; `html_id` is the
    /// root element id of HTML outputs, unique per display (required for them).
    /// Returns the encoded output (UTF-8 for text) and the build warnings.
    /// The GIL is released while building and rendering.
    #[pyfunction]
    #[pyo3(signature = (spec_json, frames, output, scale, html_id=None))]
    fn render<'py>(
        python: Python<'py>,
        spec_json: &str,
        frames: Vec<AnyRecordBatch>,
        output: &str,
        scale: f64,
        html_id: Option<String>,
    ) -> PyResult<(Bound<'py, PyBytes>, Vec<String>)> {
        let frames = frames
            .into_iter()
            .map(to_frame)
            .collect::<PyResult<Vec<_>>>()?;
        let spec_json = spec_json.to_owned();
        let output = output.to_owned();
        let (bytes, warnings) = python
            .detach(move || -> ggrust_core::Result<(Vec<u8>, Vec<String>)> {
                let document = match output.as_str() {
                    "svg" | "png" => None,
                    "html" => Some(HtmlDocument::Fragment),
                    "html_page" => Some(HtmlDocument::Page),
                    "html_iframe" => Some(HtmlDocument::Iframe),
                    other => {
                        return Err(ggrust_core::Error::Render(format!(
                            "unknown output format '{other}'; expected 'svg', 'png', \
                             'html', 'html_page' or 'html_iframe'"
                        )));
                    }
                };
                let html = match (document, html_id) {
                    (Some(document), Some(id)) => Some(HtmlOptions { id, document }),
                    (Some(_), None) => {
                        return Err(ggrust_core::Error::Render(format!(
                            "output '{output}' needs 'html_id' (unique per display)"
                        )));
                    }
                    (None, _) => None,
                };
                let figure = Figure::from_json(&spec_json)?;
                let built = build_figure(&figure, &frames)?;
                let bytes = match html {
                    Some(options) => to_html(&built.scene, &options).into_bytes(),
                    None if output == "png" => to_png(&built.scene, scale)?,
                    None => to_svg(&built.scene).into_bytes(),
                };
                Ok((bytes, built.warnings))
            })
            .map_err(to_value_error)?;
        Ok((PyBytes::new(python, &bytes), warnings))
    }

    fn to_frame(data: AnyRecordBatch) -> PyResult<DataFrame> {
        let (batches, schema) = data.into_table()?.into_inner();
        DataFrame::new(schema, batches).map_err(to_value_error)
    }
}
