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
        DEFAULT_JPEG_QUALITY, HtmlDocument, HtmlOptions, SCREEN_DPI, rasterize_layers,
        rgba_png_data_url, to_html, to_jpeg, to_pdf, to_png_with_dpi, to_svg, to_tiff,
    };
    use ggrust_core::spec::Figure;
    use pyo3::IntoPyObjectExt;
    use pyo3::exceptions::PyValueError;
    use pyo3::types::{PyBytes, PyDict};
    use pyo3_arrow::input::AnyRecordBatch;

    use ggrust_core::scale::secondary::{AxisFunction, AxisFunctions};
    use std::sync::Arc;

    /// A Python callable used as a secondary axis transform or a facet labeller
    /// function: called once per value (with the GIL taken again while the engine
    /// runs detached).
    struct PythonFunction(Py<PyAny>);

    impl AxisFunction for PythonFunction {
        fn call(&self, values: &[f64]) -> Result<Vec<f64>, String> {
            Python::attach(|python| {
                let function = self.0.bind(python);
                values
                    .iter()
                    .map(|value| {
                        let result = function
                            .call1((*value,))
                            .map_err(|error| error.to_string())?;
                        result.extract::<f64>().map_err(|_| {
                            format!(
                                "it returned {} for {value}; return a number",
                                result.repr().map_or_else(
                                    |_| "a non-number".to_owned(),
                                    |repr| repr.to_string()
                                )
                            )
                        })
                    })
                    .collect()
            })
        }

        fn call_text(&self, values: &[String]) -> Result<Vec<String>, String> {
            Python::attach(|python| {
                let function = self.0.bind(python);
                values
                    .iter()
                    .map(|value| {
                        let result = function
                            .call1((value.as_str(),))
                            .map_err(|error| error.to_string())?;
                        result.extract::<String>().map_err(|_| {
                            format!(
                                "it returned {} for '{value}'; return a string",
                                result.repr().map_or_else(
                                    |_| "a non-string".to_owned(),
                                    |repr| repr.to_string()
                                )
                            )
                        })
                    })
                    .collect()
            })
        }

        /// The Python front end wraps a `rescaler` so it takes and returns lists
        /// of floats (`ggrust._render._rescaler_call`).
        fn rescale(
            &self,
            values: &[f64],
            to: (f64, f64),
            from: (f64, f64),
        ) -> Result<Vec<f64>, String> {
            Python::attach(|python| {
                let result = self
                    .0
                    .bind(python)
                    .call1((values.to_vec(), to, from))
                    .map_err(|error| error.to_string())?;
                result
                    .extract::<Vec<f64>>()
                    .map_err(|_| "it did not return numbers; return one number per value".into())
            })
        }
    }

    /// Parse a figure and attach the secondary axis transforms (`functions[i]` is
    /// `sec_axis.transform == i` in the spec).
    fn figure_with_functions(
        spec_json: &str,
        functions: &AxisFunctions,
    ) -> ggrust_core::Result<Figure> {
        let mut figure = Figure::from_json(spec_json)?;
        figure.attach_functions(functions);
        Ok(figure)
    }

    fn axis_functions(functions: Option<Vec<Py<PyAny>>>) -> AxisFunctions {
        AxisFunctions(
            functions
                .unwrap_or_default()
                .into_iter()
                .map(|function| Arc::new(PythonFunction(function)) as Arc<dyn AxisFunction>)
                .collect(),
        )
    }

    /// Return the version of the Rust engine.
    ///
    /// Returns
    /// -------
    /// str
    ///     The engine version, e.g. ``"0.4.0"``.
    ///
    /// Examples
    /// --------
    /// >>> from ggrust import engine_version
    /// >>> version = engine_version()
    #[pyfunction]
    fn engine_version() -> &'static str {
        ggrust_core::VERSION
    }

    /// Register the faces of the font file at `path` (`family` overrides the
    /// name read from the file); returns the families registered.
    ///
    /// Raises `ggrust.errors.FontNotFoundError` or `ggrust.errors.FontError`.
    #[pyfunction]
    #[pyo3(signature = (path, family=None))]
    fn register_font(
        python: Python<'_>,
        path: std::path::PathBuf,
        family: Option<String>,
    ) -> PyResult<Vec<String>> {
        python
            .detach(|| ggrust_core::text::register_font(&path, family.as_deref()))
            .map_err(to_python_error)
    }

    /// Families registered from a directory, and skipped files with their reasons.
    type DirectoryFonts = (Vec<String>, Vec<(String, String)>);

    /// Register every font file under the directory `path`; returns the families
    /// and the skipped files with their reasons.
    ///
    /// Raises `ggrust.errors.FontNotFoundError` or `ggrust.errors.FontError`.
    #[pyfunction]
    fn register_font_dir(python: Python<'_>, path: std::path::PathBuf) -> PyResult<DirectoryFonts> {
        let registered = python
            .detach(|| ggrust_core::text::register_font_dir(&path))
            .map_err(to_python_error)?;
        let skipped = registered
            .skipped
            .into_iter()
            .map(|(file, reason)| (file.display().to_string(), reason))
            .collect();
        Ok((registered.families, skipped))
    }

    /// Every registered family and its faces (`plain`, `bold`, `italic`,
    /// `bold_italic`), in registration order.
    #[pyfunction]
    fn registered_fonts() -> Vec<(String, Vec<&'static str>)> {
        ggrust_core::text::registered_fonts()
            .into_iter()
            .map(|(family, faces)| (family, faces.into_iter().map(|face| face.label()).collect()))
            .collect()
    }

    /// The font files of registered family `family` (case-insensitive), each once,
    /// with whether the font license allows embedding. Empty when not registered.
    #[pyfunction]
    fn registered_font_files<'py>(
        python: Python<'py>,
        family: &str,
    ) -> Vec<(pyo3::Bound<'py, pyo3::types::PyBytes>, bool)> {
        ggrust_core::text::registered_font_files(family)
            .into_iter()
            .map(|(data, embeddable)| (pyo3::types::PyBytes::new(python, data), embeddable))
            .collect()
    }

    /// Register the faces of font file bytes under `family`; return the families.
    ///
    /// Raises `ValueError` with the reason if the bytes are not a usable font.
    #[pyfunction]
    fn register_font_bytes(
        python: Python<'_>,
        data: Vec<u8>,
        family: &str,
    ) -> PyResult<Vec<String>> {
        python
            .detach(|| ggrust_core::text::register_font_data(data, Some(family)))
            .map_err(PyValueError::new_err)
    }

    /// Encode straight-alpha RGBA bytes (`width` x `height`, top row first) as a
    /// `data:image/png;base64,...` URL (`geom_imshow`). `compression` is a zlib
    /// level 0..9; `None` picks the fastest. The GIL is released while encoding.
    ///
    /// Raises `ggrust.errors.RenderError` if `rgba` does not hold `width * height * 4` bytes.
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
            .map_err(to_python_error)
    }

    /// Advance width in pixels of one line of `text` at `size` px in the default
    /// font, with `fontface` parsed as lets-plot does (`"bold"`, `"bold_italic"`, ...).
    #[pyfunction]
    #[pyo3(signature = (text, size, fontface="plain"))]
    fn text_width(text: &str, size: f64, fontface: &str) -> f64 {
        use ggrust_core::text::{FontFace, measure};
        measure(text, size, FontFace::from_name(fontface)).width
    }

    /// Parse a plot spec (JSON of `as_dict()` without data) and return the
    /// paths of options ggrust does not support.
    ///
    /// Raises `ggrust.errors.SpecError` (with its `path`) if the spec is invalid.
    #[pyfunction]
    fn validate_spec(spec_json: &str) -> PyResult<Vec<String>> {
        ggrust_core::spec::Figure::from_json(spec_json)
            .map(|figure| figure.unsupported_options())
            .map_err(to_python_error)
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
    /// Raises `ggrust.errors.SpecError` (with its `path`) if the spec is invalid.
    #[pyfunction]
    fn referenced_columns(spec_json: &str) -> PyResult<Vec<String>> {
        Figure::from_json(spec_json)
            .map(|figure| figure.referenced_columns().into_iter().collect())
            .map_err(to_python_error)
    }

    /// Pearson correlations between the numeric columns of an Arrow table
    /// (`corr_plot`), or the table itself when it already is a correlation matrix.
    /// Returns `(names, row-major values, given)`. The GIL is released while computing.
    #[pyfunction]
    fn correlation_matrix(
        python: Python<'_>,
        data: AnyRecordBatch,
    ) -> PyResult<(Vec<String>, Vec<f64>, bool)> {
        let frame = to_frame(data)?;
        python
            .detach(|| ggrust_core::data::correlation::correlation_matrix(&frame))
            .map(|matrix| (matrix.names, matrix.values, matrix.given))
            .map_err(to_python_error)
    }

    /// Hierarchical clustering of an Arrow table's rows (or numeric columns,
    /// `by_columns`) with R `hclust` results (see `ggrust_core::cluster`).
    /// Returns `(labels, merge, height, order)`: merge in R's signed 1-based form,
    /// order 0-based. The GIL is released while computing.
    #[pyfunction]
    #[pyo3(signature = (data, labels, by_columns, method, distance))]
    #[allow(clippy::type_complexity)]
    fn hclust(
        python: Python<'_>,
        data: AnyRecordBatch,
        labels: Option<String>,
        by_columns: bool,
        method: &str,
        distance: &str,
    ) -> PyResult<(Vec<String>, Vec<[i64; 2]>, Vec<f64>, Vec<usize>)> {
        use ggrust_core::cluster::{Distance, Linkage, cluster_frame};
        let linkage = Linkage::from_name(method).ok_or_else(|| {
            PyValueError::new_err(format!(
                "unknown method '{method}'; accepted: complete, average, single, ward.D2"
            ))
        })?;
        let distance = Distance::from_name(distance).ok_or_else(|| {
            PyValueError::new_err(format!(
                "unknown distance '{distance}'; accepted: euclidean, correlation"
            ))
        })?;
        let frame = to_frame(data)?;
        python
            .detach(|| cluster_frame(&frame, labels.as_deref(), by_columns, distance, linkage))
            .map(|tree| (tree.labels, tree.merge, tree.height, tree.order))
            .map_err(to_python_error)
    }

    /// The links of a tree (checked as in a plot spec) with leaf `i` of `order`
    /// at `i`: `(position, height, position_end, height_end)` columns. `kind` is
    /// `"rectangle"` or `"triangle"`.
    #[pyfunction]
    #[allow(clippy::type_complexity)]
    fn dendrogram_segments(
        labels: Vec<String>,
        merge: Vec<[i64; 2]>,
        height: Vec<f64>,
        order: Vec<usize>,
        kind: &str,
    ) -> PyResult<(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> {
        use ggrust_core::cluster::{DendrogramType, Tree};
        let kind = match kind {
            "rectangle" => DendrogramType::Rectangle,
            "triangle" => DendrogramType::Triangle,
            other => {
                return Err(PyValueError::new_err(format!(
                    "unknown dendrogram type '{other}'; accepted: rectangle, triangle"
                )));
            }
        };
        let tree =
            Tree::from_spec("tree", labels, merge, height, order).map_err(to_python_error)?;
        let segments = tree.segments(kind);
        Ok((
            segments.iter().map(|segment| segment.position).collect(),
            segments.iter().map(|segment| segment.height).collect(),
            segments
                .iter()
                .map(|segment| segment.position_end)
                .collect(),
            segments.iter().map(|segment| segment.height_end).collect(),
        ))
    }

    /// Streamlines of `(u, v)` on the grid `x` by `y` (matplotlib's `streamplot`):
    /// `(lines, arrows)` with lines as `(x, y, group)` columns and arrows as
    /// `(x, y, xend, yend, group)` columns.
    #[pyfunction]
    #[pyo3(signature = (x, y, u, v, density, minlength, maxlength, integration_direction, broken_streamlines, start_points=None))]
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    fn streamplot(
        python: Python<'_>,
        x: Vec<f64>,
        y: Vec<f64>,
        u: Vec<Vec<f64>>,
        v: Vec<Vec<f64>>,
        density: (f64, f64),
        minlength: f64,
        maxlength: f64,
        integration_direction: &str,
        broken_streamlines: bool,
        start_points: Option<Vec<[f64; 2]>>,
    ) -> PyResult<(
        (Vec<f64>, Vec<f64>, Vec<u32>),
        (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<u32>),
    )> {
        use ggrust_core::streamplot::{IntegrationDirection, StreamplotOptions, streamplot};
        let direction = match integration_direction {
            "both" => IntegrationDirection::Both,
            "forward" => IntegrationDirection::Forward,
            "backward" => IntegrationDirection::Backward,
            other => {
                return Err(PyValueError::new_err(format!(
                    "unknown integration_direction '{other}'; accepted: both, forward, backward"
                )));
            }
        };
        let options = StreamplotOptions {
            density,
            minlength,
            maxlength,
            direction,
            broken_streamlines,
            start_points,
        };
        let lines = python
            .detach(|| streamplot(&x, &y, &u, &v, &options))
            .map_err(to_python_error)?;
        let (mut line_x, mut line_y, mut line_group) = (Vec::new(), Vec::new(), Vec::new());
        let mut arrows = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for (group, line) in (0u32..).zip(&lines) {
            line_x.extend_from_slice(&line.x);
            line_y.extend_from_slice(&line.y);
            line_group.extend(std::iter::repeat_n(group, line.x.len()));
            if let Some(arrow) = line.arrow() {
                arrows.0.push(arrow.x);
                arrows.1.push(arrow.y);
                arrows.2.push(arrow.x_end);
                arrows.3.push(arrow.y_end);
                arrows.4.push(group);
            }
        }
        Ok(((line_x, line_y, line_group), arrows))
    }

    /// Cell velocities averaged onto a grid (scVelo's `compute_velocity_on_grid`):
    /// `(x, y, u, v)` with `u[row][column]` at `(x[column], y[row])`.
    #[pyfunction]
    #[pyo3(signature = (points, vectors, density, smooth, n_neighbors, min_mass, cutoff_percentile, adjust_for_stream))]
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    fn velocity_grid(
        python: Python<'_>,
        points: Vec<[f64; 2]>,
        vectors: Vec<[f64; 2]>,
        density: f64,
        smooth: f64,
        n_neighbors: Option<usize>,
        min_mass: f64,
        cutoff_percentile: f64,
        adjust_for_stream: bool,
    ) -> PyResult<(Vec<f64>, Vec<f64>, Vec<Vec<f64>>, Vec<Vec<f64>>)> {
        use ggrust_core::velocity_grid::{VelocityGridOptions, velocity_grid};
        let options = VelocityGridOptions {
            density,
            smooth,
            n_neighbors,
            min_mass,
            cutoff_percentile,
            adjust_for_stream,
        };
        let grid = python
            .detach(|| velocity_grid(&points, &vectors, &options))
            .map_err(to_python_error)?;
        Ok((grid.x, grid.y, grid.u, grid.v))
    }

    /// Outline of a 2D point cloud at `level` of its peak kernel density:
    /// `(x, y, path)` vertex columns. `bandwidth` is `"scott"`, `"silverman"` or a factor.
    #[pyfunction]
    #[pyo3(signature = (points, level, padding, grid_size, bandwidth))]
    #[allow(clippy::type_complexity)]
    fn density_outline(
        python: Python<'_>,
        points: Vec<[f64; 2]>,
        level: f64,
        padding: f64,
        grid_size: usize,
        bandwidth: Bound<'_, PyAny>,
    ) -> PyResult<(Vec<f64>, Vec<f64>, Vec<u32>)> {
        use ggrust_core::density_outline::{DensityOutlineOptions, density_outline};
        let bandwidth = bandwidth_method(&bandwidth)?;
        let options = DensityOutlineOptions {
            level,
            padding,
            grid_size,
            bandwidth,
        };
        let outline = python
            .detach(|| density_outline(&points, &options))
            .map_err(to_python_error)?;
        Ok((outline.x, outline.y, outline.path))
    }

    /// scipy's `bw_method`: `"scott"`, `"silverman"` or a factor.
    fn bandwidth_method(
        bandwidth: &Bound<'_, PyAny>,
    ) -> PyResult<ggrust_core::kde::BandwidthMethod> {
        use ggrust_core::kde::BandwidthMethod;
        if let Ok(name) = bandwidth.extract::<String>() {
            return match name.as_str() {
                "scott" => Ok(BandwidthMethod::Scott),
                "silverman" => Ok(BandwidthMethod::Silverman),
                _ => Err(to_python_error(ggrust_core::Error::spec(
                    "bandwidth",
                    format!(
                        "unknown bandwidth rule '{name}'; pass 'scott', 'silverman' or a number"
                    ),
                ))),
            };
        }
        Ok(BandwidthMethod::Factor(bandwidth.extract::<f64>()?))
    }

    /// One-dimensional Gaussian kernel density of `values` at each of `at`
    /// (scipy's `gaussian_kde(values)(at)`): `(at, density)` columns. `bandwidth` is
    /// `"scott"`, `"silverman"` or a factor.
    #[pyfunction]
    #[pyo3(signature = (values, at, bandwidth))]
    fn kernel_density(
        python: Python<'_>,
        values: Vec<f64>,
        at: Vec<f64>,
        bandwidth: Bound<'_, PyAny>,
    ) -> PyResult<(Vec<f64>, Vec<f64>)> {
        let bandwidth = bandwidth_method(&bandwidth)?;
        let density = python
            .detach(|| ggrust_core::kernel_density::kernel_density(&values, &at, bandwidth))
            .map_err(to_python_error)?;
        Ok((at, density))
    }

    /// ggpubr `compare_means()` p-values (see `ggrust_core::stat::compare_means`):
    /// `partitions[k][level]` holds the values of each level in partition `k`.
    /// `reference` is a level index, or `reference_all` compares each level with
    /// all values. Returns `(partition, group1, group2, p, p_adj, p_format,
    /// p_signif, method)`; `group1` is `None` for `.all.`. The GIL is released
    /// while computing.
    #[pyfunction]
    #[pyo3(signature = (partitions, levels, method, paired, reference, reference_all, p_adjust_method))]
    #[allow(clippy::type_complexity, clippy::too_many_arguments)]
    fn compare_means(
        python: Python<'_>,
        partitions: Vec<Vec<Vec<f64>>>,
        levels: Vec<String>,
        method: &str,
        paired: bool,
        reference: Option<usize>,
        reference_all: bool,
        p_adjust_method: &str,
    ) -> PyResult<(
        Vec<usize>,
        Vec<Option<usize>>,
        Vec<usize>,
        Vec<f64>,
        Vec<f64>,
        Vec<String>,
        Vec<String>,
        String,
    )> {
        use ggrust_core::stat::CompareReference;
        let reference = match (reference_all, reference) {
            (true, _) => CompareReference::All,
            (false, Some(index)) => CompareReference::Level(index),
            (false, None) => CompareReference::None,
        };
        let table = python
            .detach(|| {
                ggrust_core::stat::compare_means(
                    &partitions,
                    &levels,
                    method,
                    paired,
                    reference,
                    p_adjust_method,
                )
            })
            .map_err(to_python_error)?;
        Ok((
            table.partition,
            table.group1,
            table.group2,
            table.p,
            table.p_adjusted,
            table.p_format,
            table.p_signif,
            table.method.to_owned(),
        ))
    }

    /// Numbers as R's `format(x, digits)` prints each (`as.character` with 15).
    #[pyfunction]
    fn format_r_numbers(values: Vec<f64>, digits: usize) -> Vec<String> {
        values
            .iter()
            .map(|value| ggrust_core::stat::r_format(*value, digits))
            .collect()
    }

    /// Column names `layer_tooltips()` reads, sorted: exported for HTML output only.
    ///
    /// Raises `ggrust.errors.SpecError` (with its `path`) if the spec is invalid.
    #[pyfunction]
    fn tooltip_columns(spec_json: &str) -> PyResult<Vec<String>> {
        Figure::from_json(spec_json)
            .map(|figure| figure.tooltip_columns().into_iter().collect())
            .map_err(to_python_error)
    }

    /// The `ggrust.errors` exception for an engine error, with its structured fields
    /// as attributes (`SpecError.path`, `ColumnNotFoundError.column`, ...).
    fn to_python_error(error: ggrust_core::Error) -> PyErr {
        Python::attach(|python| {
            python_error(python, &error)
                .unwrap_or_else(|_| PyValueError::new_err(error.to_string()))
        })
    }

    fn python_error(python: Python<'_>, error: &ggrust_core::Error) -> PyResult<PyErr> {
        use ggrust_core::Error;
        let keywords = PyDict::new(python);
        let class = match error {
            Error::InvalidSpec { path, message, .. } => {
                keywords.set_item("path", path)?;
                keywords.set_item("reason", message)?;
                "SpecError"
            }
            Error::MissingColumn {
                name,
                available,
                path,
                ..
            } => {
                keywords.set_item("column", name)?;
                keywords.set_item("available", available.to_vec())?;
                keywords.set_item("path", path)?;
                "ColumnNotFoundError"
            }
            Error::UnsupportedColumnType {
                name,
                data_type,
                path,
                ..
            } => {
                keywords.set_item("column", name)?;
                keywords.set_item("data_type", data_type)?;
                keywords.set_item("path", path)?;
                "UnsupportedColumnTypeError"
            }
            Error::Render { .. } => "RenderError",
            Error::InvalidData { .. } => "DataError",
            Error::FontNotFound { path, .. } => {
                keywords.set_item("path", path)?;
                "FontNotFoundError"
            }
            Error::InvalidFont { path, .. } => {
                keywords.set_item("path", path)?;
                "FontError"
            }
        };
        let instance = python
            .import("ggrust.errors")?
            .getattr(class)?
            .call((error.to_string(),), Some(&keywords))?;
        Ok(PyErr::from_value(instance))
    }

    /// Build and render a figure (a plot, or a `gggrid` of figures).
    ///
    /// `spec_json` is the plot spec whose `data` keys are frame references
    /// (`{"frame": i}`); `frames` are the Arrow tables they point to (any object
    /// exporting `__arrow_c_stream__`).
    /// `output` is `"svg"`, `"png"`, `"jpeg"`, `"tiff"`, `"pdf"`, `"html"` (notebook
    /// fragment), `"html_page"` or `"html_iframe"`; `scale` multiplies raster/PDF pixel
    /// size; `dpi` is the raster resolution and the PDF page's (default 96);
    /// `quality` is the JPEG quality (1..=100, default 75); `html_id` is the
    /// root element id of HTML outputs, unique per display (required for them);
    /// `functions` are the secondary axis transforms the spec indexes (`sec_axis`);
    /// `metadata` records `ggrust vX.Y.Z` in the output (`render::GENERATOR`).
    /// Returns the encoded output (UTF-8 for text) and the build warnings.
    /// The GIL is released while building and rendering.
    #[pyfunction]
    #[pyo3(signature = (spec_json, frames, output, scale, html_id=None, dpi=None, functions=None, quality=None, metadata=true))]
    #[allow(clippy::too_many_arguments)]
    fn render<'py>(
        python: Python<'py>,
        spec_json: &str,
        frames: Vec<AnyRecordBatch>,
        output: &str,
        scale: f64,
        html_id: Option<String>,
        dpi: Option<f64>,
        functions: Option<Vec<Py<PyAny>>>,
        quality: Option<u8>,
        metadata: bool,
    ) -> PyResult<(Bound<'py, PyBytes>, Vec<String>)> {
        let functions = axis_functions(functions);
        let frames = frames
            .into_iter()
            .map(to_frame)
            .collect::<PyResult<Vec<_>>>()?;
        let spec_json = spec_json.to_owned();
        let output = output.to_owned();
        let (bytes, warnings) = python
            .detach(move || -> ggrust_core::Result<(Vec<u8>, Vec<String>)> {
                let document = match output.as_str() {
                    "svg" | "png" | "jpeg" | "tiff" | "pdf" => None,
                    "html" => Some(HtmlDocument::Fragment),
                    "html_page" => Some(HtmlDocument::Page),
                    "html_iframe" => Some(HtmlDocument::Iframe),
                    other => {
                        return Err(ggrust_core::Error::render(format!(
                            "unknown output format '{other}'; expected 'svg', 'png', 'jpeg', \
                             'tiff', 'pdf', 'html', 'html_page' or 'html_iframe'"
                        )));
                    }
                };
                let html = match (document, html_id) {
                    (Some(document), Some(id)) => Some(HtmlOptions { id, document }),
                    (Some(_), None) => {
                        return Err(ggrust_core::Error::render(format!(
                            "output '{output}' needs 'html_id' (unique per display)"
                        )));
                    }
                    (None, _) => None,
                };
                let figure = figure_with_functions(&spec_json, &functions)?;
                let mut built = build_figure_with(&figure, &frames, html.is_some())?;
                built.scene.metadata = metadata;
                let bytes = match html {
                    Some(options) => {
                        to_html(&built.scene, built.interaction.as_ref(), &options).into_bytes()
                    }
                    None if output == "png" => {
                        to_png_with_dpi(&built.scene, scale, dpi.unwrap_or(SCREEN_DPI))?
                    }
                    None if output == "jpeg" => to_jpeg(
                        &built.scene,
                        scale,
                        dpi.unwrap_or(SCREEN_DPI),
                        quality.unwrap_or(DEFAULT_JPEG_QUALITY),
                    )?,
                    None if output == "tiff" => {
                        to_tiff(&built.scene, scale, dpi.unwrap_or(SCREEN_DPI))?
                    }
                    None if output == "pdf" => {
                        to_pdf(&built.scene, scale, dpi.unwrap_or(SCREEN_DPI))?
                    }
                    // A layout pixel is a CSS pixel (1/96 inch) in SVG.
                    None => to_svg(rasterize_layers(&built.scene, 1.0 / SCREEN_DPI)?.as_ref())
                        .into_bytes(),
                };
                Ok((bytes, built.warnings))
            })
            .map_err(to_python_error)?;
        Ok((PyBytes::new(python, &bytes), warnings))
    }

    /// What `inspect` returns: layer tables, panel scale dicts, warnings.
    type Inspection<'py> = (
        Vec<Bound<'py, PyAny>>,
        Vec<Bound<'py, PyDict>>,
        Vec<Bound<'py, PyDict>>,
        Vec<String>,
        Option<String>,
    );

    /// Prepare and lay out a single plot without drawing it (``PlotSpec.build()``).
    /// Arguments as `render`. Returns one Arrow table per spec layer (data after
    /// stat and position), one dict per panel with its `x` and `y` scales
    /// (`discrete`, `domain`, `breaks`, `labels`), one dict per shown style guide
    /// (`aesthetic`, `group`, `title`, `labels`, `values`), the build warnings, and
    /// why panel pixels are not linear in transformed units (`None` when they are).
    /// Each panel also has its pixel `box` and `flipped`; each axis `reversed`.
    /// The GIL is released while building.
    #[pyfunction]
    #[pyo3(signature = (spec_json, frames, functions=None))]
    fn inspect<'py>(
        python: Python<'py>,
        spec_json: &str,
        frames: Vec<AnyRecordBatch>,
        functions: Option<Vec<Py<PyAny>>>,
    ) -> PyResult<Inspection<'py>> {
        let functions = axis_functions(functions);
        use ggrust_core::build::PlotData;
        use ggrust_core::build::inspect::{AxisScale, GuideValue, inspect_plot};
        use pyo3_arrow::PyTable;

        let frames = frames
            .into_iter()
            .map(to_frame)
            .collect::<PyResult<Vec<_>>>()?;
        let spec_json = spec_json.to_owned();
        let inspection = python
            .detach(move || -> ggrust_core::Result<_> {
                match figure_with_functions(&spec_json, &functions)? {
                    Figure::Plot(spec) => {
                        inspect_plot(&spec, &PlotData::from_frames(&spec, &frames)?)
                    }
                    Figure::Subplots(_) => Err(ggrust_core::Error::render(
                        "only a single plot can be inspected; inspect each figure of a \
                         gggrid on its own (grid.plots())",
                    )),
                }
            })
            .map_err(to_python_error)?;
        let layers = inspection
            .layers
            .into_iter()
            .map(|batch| {
                let schema = batch.schema();
                let table = PyTable::try_new(vec![batch], schema)?;
                Ok(Bound::new(python, table)?.into_any())
            })
            .collect::<PyResult<Vec<_>>>()?;
        let axis = |scale: &AxisScale| -> PyResult<Bound<'py, PyDict>> {
            let dict = PyDict::new(python);
            dict.set_item("discrete", scale.discrete)?;
            dict.set_item("limits", scale.limits)?;
            dict.set_item("levels", &scale.levels)?;
            dict.set_item("domain", scale.domain)?;
            dict.set_item("breaks", &scale.breaks)?;
            dict.set_item("labels", &scale.labels)?;
            dict.set_item("reversed", scale.reversed)?;
            if let Some((breaks, labels)) = &scale.secondary {
                let secondary = PyDict::new(python);
                secondary.set_item("breaks", breaks)?;
                secondary.set_item("labels", labels)?;
                dict.set_item("secondary", secondary)?;
            }
            Ok(dict)
        };
        let panels = inspection
            .panels
            .iter()
            .map(|panel| {
                let dict = PyDict::new(python);
                dict.set_item("x", axis(&panel.x)?)?;
                dict.set_item("y", axis(&panel.y)?)?;
                dict.set_item("box", panel.rect)?;
                dict.set_item("flipped", inspection.flipped)?;
                Ok(dict)
            })
            .collect::<PyResult<Vec<_>>>()?;
        let guides = inspection
            .guides
            .iter()
            .map(|guide| {
                let dict = PyDict::new(python);
                dict.set_item("aesthetic", guide.aesthetic.as_str())?;
                dict.set_item("group", guide.group)?;
                dict.set_item("title", &guide.title)?;
                dict.set_item("labels", &guide.labels)?;
                let values = guide
                    .values
                    .iter()
                    .map(|value| match value {
                        GuideValue::Text(text) => text.into_bound_py_any(python),
                        GuideValue::Number(number) => number.into_bound_py_any(python),
                        GuideValue::Integer(number) => number.into_bound_py_any(python),
                    })
                    .collect::<PyResult<Vec<_>>>()?;
                dict.set_item("values", values)?;
                Ok(dict)
            })
            .collect::<PyResult<Vec<_>>>()?;
        Ok((
            layers,
            panels,
            guides,
            inspection.warnings,
            inspection.nonlinear,
        ))
    }

    /// A geom's default stat, position and style aesthetics as JSON
    /// (``LayerSpec.defaults``). Raises `ValueError` for an unknown geom name, with
    /// the closest known one.
    #[pyfunction]
    fn geom_defaults(name: &str) -> PyResult<String> {
        ggrust_core::build::inspect::geom_defaults(name)
            .map(|value| value.to_string())
            .ok_or_else(|| {
                let names = ggrust_core::geom::names();
                let hint = ggrust_core::suggest::did_you_mean(name, names.iter().copied());
                PyValueError::new_err(format!("unknown geom '{name}'{hint}"))
            })
    }

    /// A named theme's options in `theme()` vocabulary as JSON (``theme_defaults()``).
    /// Raises `ValueError` for an unknown theme or flavor name.
    #[pyfunction]
    #[pyo3(signature = (name, flavor=None))]
    fn theme_defaults(name: &str, flavor: Option<&str>) -> PyResult<String> {
        ggrust_core::theme::theme_defaults(name, flavor)
            .map(|value| value.to_string())
            .map_err(PyValueError::new_err)
    }

    fn to_frame(data: AnyRecordBatch) -> PyResult<DataFrame> {
        let (batches, schema) = data.into_table()?.into_inner();
        DataFrame::new(schema, batches).map_err(to_python_error)
    }
}
