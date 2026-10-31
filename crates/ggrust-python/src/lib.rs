//! Python extension module `ggrust._ggrust`.
//!
//! Only conversion and dispatch live here. Engine logic belongs in `ggrust-core`.

use pyo3::prelude::*;

/// Native extension module of ggrust.
#[pymodule]
mod _ggrust {
    use super::*;
    use ggrust_core::data::{Column, DataFrame};
    use ggrust_core::grid::build_figure_with;
    use ggrust_core::render::{
        HtmlDocument, HtmlOptions, SCREEN_DPI, rgba_png_data_url, to_html, to_pdf, to_png_with_dpi,
        to_svg,
    };
    use ggrust_core::spec::Figure;
    use pyo3::exceptions::PyValueError;
    use pyo3::types::{PyBytes, PyDict};
    use pyo3_arrow::input::AnyRecordBatch;

    /// Return the version of the Rust engine.
    #[pyfunction]
    fn engine_version() -> &'static str {
        ggrust_core::VERSION
    }

    /// Encode straight-alpha RGBA bytes (`width` x `height`, top row first) as a
    /// `data:image/png;base64,...` URL (`geom_imshow`). `compression` is a zlib
    /// level 0..9; `None` picks the fastest. The GIL is released while encoding.
    ///
    /// Raises `ValueError` if `rgba` does not hold `width * height * 4` bytes.
    #[pyfunction]
    #[pyo3(signature = (rgba, width, height, compression=None))]
    fn png_data_url(
        python: Python<'_>,
        rgba: &[u8],
        width: u32,
        height: u32,
        compression: Option<u8>,
    ) -> PyResult<String> {
        python
            .detach(|| rgba_png_data_url(width, height, rgba, compression))
            .map_err(to_value_error)
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

    /// Column names a spec reads in every output (mappings, `order_by`, facets),
    /// sorted. Only these (plus [`tooltip_columns`] for HTML) are exported as Arrow.
    ///
    /// Raises `ValueError` naming the offending path if the spec is invalid.
    #[pyfunction]
    fn referenced_columns(spec_json: &str) -> PyResult<Vec<String>> {
        Figure::from_json(spec_json)
            .map(|figure| figure.referenced_columns().into_iter().collect())
            .map_err(to_value_error)
    }

    /// Column names `layer_tooltips()` reads, sorted: exported for HTML output only.
    ///
    /// Raises `ValueError` naming the offending path if the spec is invalid.
    #[pyfunction]
    fn tooltip_columns(spec_json: &str) -> PyResult<Vec<String>> {
        Figure::from_json(spec_json)
            .map(|figure| figure.tooltip_columns().into_iter().collect())
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
    /// `output` is `"svg"`, `"png"`, `"pdf"`, `"html"` (notebook fragment),
    /// `"html_page"` or `"html_iframe"`; `scale` multiplies PNG/PDF pixel size;
    /// `dpi` is the PNG resolution and the PDF page's (default 96); `html_id` is the
    /// root element id of HTML outputs, unique per display (required for them).
    /// Returns the encoded output (UTF-8 for text) and the build warnings.
    /// The GIL is released while building and rendering.
    #[pyfunction]
    #[pyo3(signature = (spec_json, frames, output, scale, html_id=None, dpi=None))]
    fn render<'py>(
        python: Python<'py>,
        spec_json: &str,
        frames: Vec<AnyRecordBatch>,
        output: &str,
        scale: f64,
        html_id: Option<String>,
        dpi: Option<f64>,
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
                    "svg" | "png" | "pdf" => None,
                    "html" => Some(HtmlDocument::Fragment),
                    "html_page" => Some(HtmlDocument::Page),
                    "html_iframe" => Some(HtmlDocument::Iframe),
                    other => {
                        return Err(ggrust_core::Error::Render(format!(
                            "unknown output format '{other}'; expected 'svg', 'png', 'pdf', \
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
                let built = build_figure_with(&figure, &frames, html.is_some())?;
                let bytes = match html {
                    Some(options) => {
                        to_html(&built.scene, built.interaction.as_ref(), &options).into_bytes()
                    }
                    None if output == "png" => {
                        to_png_with_dpi(&built.scene, scale, dpi.unwrap_or(SCREEN_DPI))?
                    }
                    None if output == "pdf" => {
                        to_pdf(&built.scene, scale, dpi.unwrap_or(SCREEN_DPI))?
                    }
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
