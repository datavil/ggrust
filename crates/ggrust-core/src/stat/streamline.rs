//! `stat="streamline"` (ggrust extension; R: metR's `stat_streamline`):
//! streamlines of a 2D vector field, one output group per line.
//!
//! Per group, the rows' positions (`x`, `y`) and vectors (`u`, `v`) become a
//! field on a regular grid, then [`crate::streamplot`] (matplotlib's integrator)
//! traces the lines:
//!
//! - Rows that already form a regular grid (every pair of the distinct x and y
//!   values exactly once) are the field as given. `gridded=True` also accepts an
//!   incomplete grid (missing cells stop lines); `gridded=False` never reads rows
//!   as a grid.
//! - Other rows are scattered vectors, averaged onto a grid by
//!   [`crate::velocity_grid`] (Gaussian-weighted neighbours, stream cutoffs).
//!
//! Positions are in axis units, so lines and arrows are computed after scale
//! transforms, as matplotlib computes them in data space. The geom places the
//! arrow of each line ([`crate::streamplot::Streamline::arrow`]).
//!
//! What it does not do: `start_points`, color or width varying along a line
//! (speed is not exported), or 3D fields.

use super::groups::per_group_pieces;
use super::{Stat, StatContext};
use crate::data::{ContinuousColumn, LayerData, Variable};
use crate::parameters::Parameters;
use crate::spec::{Aesthetic, Value};
use crate::streamplot::{IntegrationDirection, StreamplotOptions, streamplot};
use crate::velocity_grid::{VelocityGridOptions, velocity_grid};
use crate::{Error, Result};

/// Streamlines of `(u, v)` at `(x, y)`.
pub struct StreamlineStat;

/// The parameters this stat reads, in documentation order.
const PARAMETERS: &[&str] = &[
    "density",
    "minlength",
    "maxlength",
    "integration_direction",
    "broken_streamlines",
    "gridded",
    "grid_density",
    "smooth",
    "n_neighbors",
    "min_mass",
    "cutoff_percentile",
];

impl Stat for StreamlineStat {
    fn name(&self) -> &'static str {
        "streamline"
    }

    fn parameters(&self) -> &'static [&'static str] {
        PARAMETERS
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y, Aesthetic::U, Aesthetic::V]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y, Aesthetic::U, Aesthetic::V]
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        _context: &StatContext,
    ) -> Result<LayerData> {
        let settings = Settings::parse(parameters)?;
        let empty = ContinuousColumn::from_values(Vec::new());
        let column = |aesthetic| data.continuous(aesthetic).unwrap_or(&empty);
        let (x, y, u, v) = (
            column(Aesthetic::X),
            column(Aesthetic::Y),
            column(Aesthetic::U),
            column(Aesthetic::V),
        );
        per_group_pieces(&data, |rows| {
            let vectors: Vec<[f64; 4]> = rows
                .iter()
                .filter_map(|&row| Some([x.get(row)?, y.get(row)?, u.get(row)?, v.get(row)?]))
                .collect();
            let lines = streamlines(&vectors, &settings, parameters)?;
            let pieces: Vec<usize> = lines.iter().map(|line| line.x.len()).collect();
            let (xs, ys) = lines
                .into_iter()
                .map(|line| (line.x, line.y))
                .reduce(|(mut xs, mut ys), (x, y)| {
                    xs.extend(x);
                    ys.extend(y);
                    (xs, ys)
                })
                .unwrap_or_default();
            Ok((
                vec![
                    (Variable::Aesthetic(Aesthetic::X), xs),
                    (Variable::Aesthetic(Aesthetic::Y), ys),
                ],
                Some(pieces),
            ))
        })
    }
}

/// How rows become a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Gridded {
    /// A grid when the rows form a complete regular grid, else scattered.
    Auto,
    /// Always a grid (missing cells are NaN).
    Yes,
    /// Always scattered.
    No,
}

/// The layer's options, parsed once.
struct Settings {
    streams: StreamplotOptions,
    grid: VelocityGridOptions,
    gridded: Gridded,
}

impl Settings {
    fn parse(parameters: &Parameters) -> Result<Settings> {
        let defaults = StreamplotOptions::default();
        let direction = match parameters.string("integration_direction")? {
            None | Some("both") => IntegrationDirection::Both,
            Some("forward") => IntegrationDirection::Forward,
            Some("backward") => IntegrationDirection::Backward,
            Some(other) => {
                return Err(Error::spec(
                    parameters.path_of("integration_direction"),
                    format!(
                        "unknown integration_direction '{other}'{}; accepted: 'both', \
                         'forward', 'backward'",
                        crate::suggest::did_you_mean(other, ["both", "forward", "backward"])
                    ),
                ));
            }
        };
        let streams = StreamplotOptions {
            density: density(parameters)?,
            minlength: parameters
                .number("minlength")?
                .unwrap_or(defaults.minlength),
            maxlength: parameters
                .number("maxlength")?
                .unwrap_or(defaults.maxlength),
            direction,
            broken_streamlines: parameters
                .boolean("broken_streamlines")?
                .unwrap_or(defaults.broken_streamlines),
            start_points: None,
        };
        let grid_defaults = VelocityGridOptions::default();
        let n_neighbors = match parameters.number("n_neighbors")? {
            None => None,
            Some(value) if value >= 1.0 && value.fract() == 0.0 && value <= u32::MAX as f64 => {
                Some(value as usize)
            }
            Some(value) => {
                return Err(Error::spec(
                    parameters.path_of("n_neighbors"),
                    format!(
                        "n_neighbors must be a whole number of at least 1, got {value}; \
                         leave it out for int(n_points / 50)"
                    ),
                ));
            }
        };
        let grid = VelocityGridOptions {
            density: parameters
                .number("grid_density")?
                .unwrap_or(grid_defaults.density),
            smooth: parameters.number("smooth")?.unwrap_or(grid_defaults.smooth),
            n_neighbors,
            min_mass: parameters
                .number("min_mass")?
                .unwrap_or(grid_defaults.min_mass),
            cutoff_percentile: parameters
                .number("cutoff_percentile")?
                .unwrap_or(grid_defaults.cutoff_percentile),
            adjust_for_stream: true,
        };
        let gridded = match parameters.boolean("gridded")? {
            None => Gridded::Auto,
            Some(true) => Gridded::Yes,
            Some(false) => Gridded::No,
        };
        Ok(Settings {
            streams,
            grid,
            gridded,
        })
    }
}

/// `density=`: a number or `[x, y]`.
fn density(parameters: &Parameters) -> Result<(f64, f64)> {
    let invalid = |value: &Value| {
        Error::spec(
            parameters.path_of("density"),
            format!("expected a positive number or a pair [x, y] of them, got {value}"),
        )
    };
    match parameters.value("density") {
        None => Ok(StreamplotOptions::default().density),
        Some(Value::Array(items)) if items.len() == 2 => {
            match (items[0].as_f64(), items[1].as_f64()) {
                (Some(x), Some(y)) => Ok((x, y)),
                _ => Err(invalid(&Value::Array(items.clone()))),
            }
        }
        Some(value) => value
            .as_f64()
            .map(|number| (number, number))
            .ok_or_else(|| invalid(value)),
    }
}

/// The streamlines of one group's `[x, y, u, v]` rows.
fn streamlines(
    vectors: &[[f64; 4]],
    settings: &Settings,
    parameters: &Parameters,
) -> Result<Vec<crate::streamplot::Streamline>> {
    if vectors.is_empty() {
        return Ok(Vec::new());
    }
    let field = match settings.gridded {
        Gridded::No => None,
        Gridded::Auto => regular_grid(vectors, false),
        Gridded::Yes => Some(regular_grid(vectors, true).ok_or_else(|| {
            Error::spec(
                parameters.path_of("gridded"),
                "gridded=True needs at most one row per (x, y) grid point with finite x and y \
                 and at least 2 distinct values of each; pass gridded=False (or leave it out) \
                 to smooth scattered vectors onto a grid",
            )
        })?),
    };
    let from_rows = field.is_some();
    let field = match field {
        Some(field) => field,
        None => {
            let (points, directions): (Vec<[f64; 2]>, Vec<[f64; 2]>) = vectors
                .iter()
                .map(|[x, y, u, v]| ([*x, *y], [*u, *v]))
                .unzip();
            let grid = velocity_grid(&points, &directions, &settings.grid)
                .map_err(|error| relocate(error, parameters, GRID_NAMES))?;
            Field {
                x: grid.x,
                y: grid.y,
                u: grid.u,
                v: grid.v,
            }
        }
    };
    streamplot(&field.x, &field.y, &field.u, &field.v, &settings.streams).map_err(|error| {
        let error = relocate(error, parameters, STREAM_NAMES);
        if from_rows && is_data_error(&error, parameters) {
            error.with_notes(&[String::from(
                "the rows were read as a regular grid, which must be evenly spaced on the \
                 axis scale (a log scale needs log-spaced values); pass gridded=False to \
                 average them onto a grid instead",
            )])
        } else {
            error
        }
    })
}

/// Whether a relocated error points at the layer's data (a grid axis).
fn is_data_error(error: &Error, parameters: &Parameters) -> bool {
    matches!(error, Error::InvalidSpec { path, .. } if *path == parameters.path_of("data"))
}

/// Core option names (velocity_grid's) and the layer options they are.
const GRID_NAMES: &[(&str, &str)] = &[
    ("density", "grid_density"),
    ("smooth", "smooth"),
    ("n_neighbors", "n_neighbors"),
    ("min_mass", "min_mass"),
    ("cutoff_percentile", "cutoff_percentile"),
];

/// Core option names (streamplot's) and the layer options they are.
const STREAM_NAMES: &[(&str, &str)] = &[
    ("density", "density"),
    ("minlength", "minlength"),
    ("maxlength", "maxlength"),
];

/// A core error located at the layer option it came from (other paths, such as
/// the grid axes, become the layer's data and keep their name in the message).
fn relocate(error: Error, parameters: &Parameters, names: &[(&str, &str)]) -> Error {
    match error {
        Error::InvalidSpec { path, message, .. } => {
            match names.iter().find(|(core, _)| *core == path) {
                Some((core, layer)) => {
                    // "density must be ..." reads "grid_density must be ...".
                    let message = match message.strip_prefix(core) {
                        Some(rest) => format!("{layer}{rest}"),
                        None => message,
                    };
                    let message = if *layer == "n_neighbors" {
                        format!(
                            "{message} (every group and facet panel is gridded from its own \
                             rows, so the default needs 50 rows per group)"
                        )
                    } else {
                        message
                    };
                    Error::spec(parameters.path_of(layer), message)
                }
                None => Error::spec(
                    parameters.path_of("data"),
                    format!("stat='streamline' ({path}): {message}"),
                ),
            }
        }
        other => other,
    }
}

/// A field on a regular grid, laid out as [`streamplot`] reads it.
struct Field {
    x: Vec<f64>,
    y: Vec<f64>,
    u: Vec<Vec<f64>>,
    v: Vec<Vec<f64>>,
}

/// The rows as a grid over their distinct x and y values, or `None` when they
/// are not one: a non-finite or repeated position, fewer than 2 distinct values
/// of x or y, or (unless `allow_missing`) a missing grid point. Missing grid
/// points are NaN (lines stop there).
fn regular_grid(vectors: &[[f64; 4]], allow_missing: bool) -> Option<Field> {
    if vectors
        .iter()
        .any(|[x, y, _, _]| !x.is_finite() || !y.is_finite())
    {
        return None;
    }
    // `+ 0.0` turns -0.0 into 0.0: one grid value (total_cmp orders them apart).
    let distinct = |index: usize| {
        let mut values: Vec<f64> = vectors.iter().map(|row| row[index] + 0.0).collect();
        values.sort_by(f64::total_cmp);
        values.dedup();
        values
    };
    let (x, y) = (distinct(0), distinct(1));
    let cells = x.len().checked_mul(y.len())?;
    if x.len() < 2 || y.len() < 2 || (!allow_missing && cells != vectors.len()) {
        return None;
    }
    if cells > MAX_GRID_CELLS {
        return None;
    }
    let mut u = vec![vec![f64::NAN; x.len()]; y.len()];
    let mut v = u.clone();
    let mut seen = vec![vec![false; x.len()]; y.len()];
    for [row_x, row_y, row_u, row_v] in vectors {
        let (row_x, row_y) = (row_x + 0.0, row_y + 0.0);
        let column = x.binary_search_by(|value| value.total_cmp(&row_x)).ok()?;
        let row = y.binary_search_by(|value| value.total_cmp(&row_y)).ok()?;
        if std::mem::replace(&mut seen[row][column], true) {
            return None;
        }
        u[row][column] = *row_u;
        v[row][column] = *row_v;
    }
    Some(Field { x, y, u, v })
}

/// Largest grid built from rows (an incomplete `gridded=True` grid can be far
/// larger than its rows).
const MAX_GRID_CELLS: usize = 10_000_000;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Column;
    use crate::spec::Options;
    use crate::streamplot::Streamline;

    fn layer(rows: &[[f64; 4]]) -> LayerData {
        let mut data = LayerData::new(rows.len());
        for (index, aesthetic) in [Aesthetic::X, Aesthetic::Y, Aesthetic::U, Aesthetic::V]
            .into_iter()
            .enumerate()
        {
            let values = rows.iter().map(|row| row[index]).collect();
            data.insert(
                aesthetic,
                Column::Continuous(ContinuousColumn::from_values(values)),
            )
            .expect("insert");
        }
        data
    }

    fn options(json: &str) -> Options {
        serde_json::from_str(json).expect("options")
    }

    /// A rotating field on a 21 x 21 grid.
    fn rotation() -> Vec<[f64; 4]> {
        let mut rows = Vec::new();
        for row in 0..21 {
            for column in 0..21 {
                let (x, y) = (column as f64 / 10.0 - 1.0, row as f64 / 10.0 - 1.0);
                rows.push([x, y, -y, x]);
            }
        }
        rows
    }

    fn lines_of(data: &LayerData) -> Vec<Streamline> {
        let x = data.continuous(Aesthetic::X).expect("x");
        let y = data.continuous(Aesthetic::Y).expect("y");
        let mut lines: Vec<Streamline> = Vec::new();
        let mut last_group = None;
        for row in 0..data.row_count() {
            let group = data.group_index().map_or(0, |groups| groups[row]);
            if last_group != Some(group) {
                lines.push(Streamline {
                    x: Vec::new(),
                    y: Vec::new(),
                });
                last_group = Some(group);
            }
            let line = lines.last_mut().expect("line");
            line.x.push(x.get(row).expect("x"));
            line.y.push(y.get(row).expect("y"));
        }
        lines
    }

    #[test]
    fn grid_rows_are_the_field_as_given() {
        let rows = rotation();
        let empty = Options::default();
        let parameters = Parameters::new(&empty, "layers[0]");
        let result = StreamlineStat
            .compute(layer(&rows), &parameters, &StatContext::default())
            .expect("stat");
        let axis: Vec<f64> = (0..21).map(|index| index as f64 / 10.0 - 1.0).collect();
        let u: Vec<Vec<f64>> = axis.iter().map(|y| vec![-y; 21]).collect();
        let v: Vec<Vec<f64>> = axis.iter().map(|_| axis.clone()).collect();
        let expected =
            streamplot(&axis, &axis, &u, &v, &StreamplotOptions::default()).expect("streamplot");
        assert!(expected.len() > 5);
        assert_eq!(lines_of(&result), expected);
    }

    #[test]
    fn scattered_rows_are_smoothed_onto_a_grid() {
        // Rows in a shuffled, incomplete layout: not a grid.
        let mut rows = rotation();
        rows.remove(7);
        let options = options(r#"{"grid_density": 0.5, "n_neighbors": 20, "density": 0.8}"#);
        let parameters = Parameters::new(&options, "layers[0]");
        let result = StreamlineStat
            .compute(layer(&rows), &parameters, &StatContext::default())
            .expect("stat");
        let points: Vec<[f64; 2]> = rows.iter().map(|row| [row[0], row[1]]).collect();
        let vectors: Vec<[f64; 2]> = rows.iter().map(|row| [row[2], row[3]]).collect();
        let grid = velocity_grid(
            &points,
            &vectors,
            &VelocityGridOptions {
                density: 0.5,
                n_neighbors: Some(20),
                ..VelocityGridOptions::default()
            },
        )
        .expect("grid");
        let expected = streamplot(
            &grid.x,
            &grid.y,
            &grid.u,
            &grid.v,
            &StreamplotOptions {
                density: (0.8, 0.8),
                ..StreamplotOptions::default()
            },
        )
        .expect("streamplot");
        assert!(!expected.is_empty());
        assert_eq!(lines_of(&result), expected);
    }

    #[test]
    fn gridded_true_accepts_missing_cells_and_false_forces_smoothing() {
        let mut rows = rotation();
        rows.remove(7);
        assert!(regular_grid(&rows, false).is_none());
        let field = regular_grid(&rows, true).expect("grid");
        assert!(field.u[0][7].is_nan());
        let full = rotation();
        assert!(regular_grid(&full, false).is_some());
        let options = options(r#"{"gridded": false, "n_neighbors": 20}"#);
        let parameters = Parameters::new(&options, "layers[0]");
        let smoothed = StreamlineStat
            .compute(layer(&full), &parameters, &StatContext::default())
            .expect("stat");
        let given = StreamlineStat
            .compute(
                layer(&full),
                &Parameters::new(&Options::default(), ""),
                &StatContext::default(),
            )
            .expect("stat");
        assert_ne!(lines_of(&smoothed), lines_of(&given));
    }

    #[test]
    fn negative_zero_is_the_same_grid_value() {
        let mut rows = rotation();
        for row in rows.iter_mut().step_by(2) {
            if row[0] == 0.0 {
                row[0] = -0.0;
            }
        }
        let field = regular_grid(&rows, false).expect("grid");
        assert_eq!(field.x.len(), 21);
    }

    #[test]
    fn duplicate_positions_are_not_a_grid() {
        let mut rows = rotation();
        rows.push(rows[0]);
        assert!(regular_grid(&rows, true).is_none());
        let options = options(r#"{"gridded": true}"#);
        let parameters = Parameters::new(&options, "layers[0]");
        let error = StreamlineStat
            .compute(layer(&rows), &parameters, &StatContext::default())
            .expect_err("duplicates");
        let text = error.to_string();
        assert!(text.contains("layers[0].gridded"), "{text}");
    }

    #[test]
    fn groups_get_their_own_lines() {
        let mut rows = rotation();
        let shifted: Vec<[f64; 4]> = rows
            .iter()
            .map(|[x, y, u, v]| [x + 5.0, *y, *v, *u])
            .collect();
        rows.extend(shifted);
        let mut data = layer(&rows);
        let groups: Vec<u32> = (0..rows.len()).map(|row| (row >= 441) as u32).collect();
        data.set_group_index(Some(groups)).expect("groups");
        let result = StreamlineStat
            .compute(
                data,
                &Parameters::new(&Options::default(), ""),
                &StatContext::default(),
            )
            .expect("stat");
        let lines = lines_of(&result);
        let left = lines.iter().filter(|line| line.x[0] < 2.0).count();
        assert!(left > 0 && left < lines.len());
    }

    #[test]
    fn errors_name_the_layer_option() {
        let rows = rotation();
        for (json, path) in [
            (r#"{"density": 0.01}"#, "layers[0].density"),
            (
                r#"{"integration_direction": "forwards"}"#,
                "layers[0].integration_direction",
            ),
            (r#"{"gridded": false, "smooth": -1}"#, "layers[0].smooth"),
            (
                r#"{"gridded": false, "grid_density": 0}"#,
                "layers[0].grid_density",
            ),
            (r#"{"n_neighbors": 1.5}"#, "layers[0].n_neighbors"),
        ] {
            let options = options(json);
            let parameters = Parameters::new(&options, "layers[0]");
            let error = StreamlineStat
                .compute(layer(&rows), &parameters, &StatContext::default())
                .expect_err(json);
            let text = error.to_string();
            assert!(text.contains(path), "{json}: {text}");
        }
    }
}
