//! `stat="density2d"` and `"density2df"`: contours of a 2D kernel density estimate
//! (lets-plot `Density2dStat`, `Density2dfStat`, `AbstractDensity2dStat`).
//!
//! The density is a product kernel on an `n = [nx, ny]` grid (default 100 x 100)
//! spanning every layer's x and y (scale limits replace the ends):
//! `sum_i w_i K((x - x_i) / hx) K((y - y_i) / hy) / (hx hy sum_i w_i)`, with
//! `h = bw * adjust`. Bandwidths per axis come from `density.rs` (R's `bw.nrd0` /
//! `bw.nrd`, as the 1D densities), or `bw` as one number or `[x, y]`. The grid
//! then goes through the contour core (`contour.rs`): lines for `density2d`, bands
//! for `density2df`, which leaves out the band below the first level (lets-plot)
//! and fills by `..level..` by default (ggplot2).
//!
//! What it does not do: `contour=False` raw density output (lets_plot's Python API
//! does not expose it).

use super::bin2d::number_pair;
use super::contour::{Grid, LevelOptions, contour_columns};
use super::count::row_weight;
use super::density::{Bandwidth, Kernel, step_values};
use super::groups::per_group_pieces;
use super::{Stat, StatContext};
use crate::data::{Computed, ContinuousColumn, LayerData};
use crate::parallel;
use crate::parameters::Parameters;
use crate::spec::{Aesthetic, Value};
use crate::{Error, Result};

/// lets-plot `AbstractDensity2dStat.DEF_N`.
const DEFAULT_N: usize = 100;
/// lets-plot `AbstractDensity2dStat.MAX_N`.
const MAX_N: usize = 999;
/// Points per partial density grid (summed in order, so results never depend on
/// the threads).
const CHUNK: usize = 4096;

/// `stat="density2d"`: density isolines.
pub struct Density2dStat;

/// `stat="density2df"`: filled density bands.
pub struct Density2dfStat;

impl Stat for Density2dStat {
    fn name(&self) -> &'static str {
        "density2d"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["kernel", "adjust", "bw", "n", "bins", "binwidth"]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y, Aesthetic::Weight]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        context: &StatContext,
    ) -> Result<LayerData> {
        compute(&data, parameters, context, false)
    }
}

impl Stat for Density2dfStat {
    fn name(&self) -> &'static str {
        "density2df"
    }

    fn parameters(&self) -> &'static [&'static str] {
        Density2dStat.parameters()
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        Density2dStat.aesthetics()
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        Density2dStat.required_aesthetics()
    }

    /// ggplot2's default (`fill = after_stat(level)`); lets-plot draws every band
    /// in one blue (deviation, the user's choice).
    fn default_mapping(&self) -> &'static [(Aesthetic, Computed)] {
        &[(Aesthetic::Fill, Computed::Level)]
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        context: &StatContext,
    ) -> Result<LayerData> {
        compute(&data, parameters, context, true)
    }
}

fn compute(
    data: &LayerData,
    parameters: &Parameters,
    context: &StatContext,
    filled: bool,
) -> Result<LayerData> {
    let options = Density2dOptions::from_parameters(parameters)?;
    let levels = LevelOptions::from_parameters(parameters)?;
    let empty = ContinuousColumn::from_values(Vec::new());
    let x = data.continuous(Aesthetic::X).unwrap_or(&empty);
    let y = data.continuous(Aesthetic::Y).unwrap_or(&empty);
    let weight = data.continuous(Aesthetic::Weight);
    let ranges = context
        .x_range
        .or_else(|| super::bin::finite_range(x))
        .zip(context.y_range.or_else(|| super::bin::finite_range(y)));
    per_group_pieces(data, |rows| {
        let points: Vec<(f64, f64, f64)> = rows
            .iter()
            .filter_map(|&row| {
                let (x, y) = (x.get(row)?, y.get(row)?);
                (x.is_finite() && y.is_finite()).then(|| (x, y, row_weight(weight, row)))
            })
            .collect();
        let Some((x_range, y_range)) = ranges.filter(|_| !points.is_empty()) else {
            return Ok(contour_columns(Vec::new()));
        };
        let x_steps = step_values(x_range.0, x_range.1, options.x_count);
        let y_steps = step_values(y_range.0, y_range.1, options.y_count);
        let z = options.density(&points, &x_steps, &y_steps);
        let grid = Grid::new(x_steps, y_steps, z);
        let pieces = if filled {
            grid.bands(&levels, false)
        } else {
            grid.lines(&levels)
        };
        Ok(contour_columns(pieces))
    })
}

/// `kernel`, `adjust`, `bw` and `n`.
#[derive(Debug, Clone, Copy)]
pub(super) struct Density2dOptions {
    pub(super) kernel: Kernel,
    pub(super) adjust: f64,
    pub(super) x_bandwidth: Bandwidth,
    pub(super) y_bandwidth: Bandwidth,
    pub(super) x_count: usize,
    pub(super) y_count: usize,
}

impl Density2dOptions {
    fn from_parameters(parameters: &Parameters) -> Result<Self> {
        let invalid = |name: &str, expected: &str, value: &Value| {
            Error::spec(
                parameters.path_of(name),
                format!("expected {expected}, got {value}"),
            )
        };
        let kernel = match parameters.value("kernel") {
            None => Kernel::Gaussian,
            Some(value) => value.as_str().and_then(Kernel::parse).ok_or_else(|| {
                invalid(
                    "kernel",
                    "one of gaussian, rectangular, triangular, biweight, epanechikov, \
                     optcosine, cosine",
                    value,
                )
            })?,
        };
        let adjust = parameters.number("adjust")?.unwrap_or(1.0);
        if !(adjust.is_finite() && adjust > 0.0) {
            return Err(Error::spec(
                parameters.path_of("adjust"),
                format!("expected a positive number, got {adjust}"),
            ));
        }
        let bandwidth_expected = "'nrd0', 'nrd', a positive number or a list of two [x, y]";
        let fixed = |value: &Value| match value.as_f64() {
            Some(number) if number.is_finite() && number > 0.0 => Ok(Bandwidth::Fixed(number)),
            _ => Err(invalid("bw", bandwidth_expected, value)),
        };
        let (x_bandwidth, y_bandwidth) = match parameters.value("bw") {
            None => (Bandwidth::Nrd0, Bandwidth::Nrd0),
            Some(Value::String(name)) if name == "nrd0" => (Bandwidth::Nrd0, Bandwidth::Nrd0),
            Some(Value::String(name)) if name == "nrd" => (Bandwidth::Nrd, Bandwidth::Nrd),
            Some(Value::Array(items)) => match items.as_slice() {
                [x, y] => (fixed(x)?, fixed(y)?),
                _ => {
                    return Err(invalid(
                        "bw",
                        bandwidth_expected,
                        &Value::Array(items.clone()),
                    ));
                }
            },
            Some(value) => {
                let both = fixed(value)?;
                (both, both)
            }
        };
        let (x_count, y_count) = number_pair(parameters, "n")?;
        let count = |value: Option<f64>| -> Result<usize> {
            let value = value.unwrap_or(DEFAULT_N as f64);
            if !(2.0..=MAX_N as f64).contains(&value.trunc()) {
                return Err(Error::spec(
                    parameters.path_of("n"),
                    format!(
                        "n must be a whole number from 2 to {MAX_N} (grid points per axis), \
                         got {value}; pass e.g. n=100 or n=[100, 50]"
                    ),
                ));
            }
            Ok(value.trunc() as usize)
        };
        Ok(Density2dOptions {
            kernel,
            adjust,
            x_bandwidth,
            y_bandwidth,
            x_count: count(x_count)?,
            y_count: count(y_count)?,
        })
    }

    /// Density on the grid, row-major (row = y step), normalised by the summed
    /// weights (lets-plot `density2dGrid`). Also `stat="pointdensity"`'s kde2d.
    pub(super) fn density(
        &self,
        points: &[(f64, f64, f64)],
        x_steps: &[f64],
        y_steps: &[f64],
    ) -> Vec<f64> {
        let sorted = |coordinate: fn(&(f64, f64, f64)) -> f64| {
            let mut values: Vec<f64> = points.iter().map(coordinate).collect();
            values.sort_by(f64::total_cmp);
            values
        };
        let x_bandwidth = self.x_bandwidth.of(&sorted(|point| point.0)) * self.adjust;
        let y_bandwidth = self.y_bandwidth.of(&sorted(|point| point.1)) * self.adjust;
        let (columns, rows) = (x_steps.len(), y_steps.len());
        let chunks: Vec<&[(f64, f64, f64)]> = points.chunks(CHUNK).collect();
        let parallel = parallel::is_worth(points.len() * columns * rows / 1000);
        let partials = parallel::ordered_map(chunks, parallel, |chunk| {
            let mut grid = vec![0.0; columns * rows];
            let mut x_weights = vec![0.0; columns];
            for &(x, y, weight) in chunk {
                if weight == 0.0 {
                    continue;
                }
                for (slot, step) in x_weights.iter_mut().zip(x_steps) {
                    *slot = self.kernel.weight((step - x) / x_bandwidth);
                }
                for (row, step) in y_steps.iter().enumerate() {
                    let y_weight = self.kernel.weight((step - y) / y_bandwidth) * weight;
                    if y_weight == 0.0 {
                        continue;
                    }
                    let line = &mut grid[row * columns..(row + 1) * columns];
                    for (cell, x_weight) in line.iter_mut().zip(&x_weights) {
                        *cell += y_weight * x_weight;
                    }
                }
            }
            grid
        });
        let total_weight: f64 = points.iter().map(|point| point.2).sum();
        let scale = 1.0 / (x_bandwidth * y_bandwidth * total_weight);
        let mut grid = vec![0.0; columns * rows];
        for partial in partials {
            for (cell, value) in grid.iter_mut().zip(partial) {
                *cell += value;
            }
        }
        for cell in &mut grid {
            *cell *= scale;
        }
        grid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> Density2dOptions {
        Density2dOptions {
            kernel: Kernel::Gaussian,
            adjust: 1.0,
            x_bandwidth: Bandwidth::Fixed(0.5),
            y_bandwidth: Bandwidth::Fixed(0.25),
            x_count: 81,
            y_count: 61,
        }
    }

    #[test]
    fn density_integrates_to_one_and_peaks_at_the_point() {
        let x_steps = step_values(-4.0, 4.0, 81);
        let y_steps = step_values(-3.0, 3.0, 61);
        let density = options().density(&[(0.0, 0.0, 1.0)], &x_steps, &y_steps);
        let cell_area = 0.1 * 0.1;
        let total: f64 = density.iter().sum::<f64>() * cell_area;
        assert!((total - 1.0).abs() < 1e-3, "{total}");
        let peak = 1.0 / (2.0 * std::f64::consts::PI * 0.5 * 0.25);
        assert!((density[30 * 81 + 40] - peak).abs() < 1e-12);
    }

    #[test]
    fn weights_scale_points_and_chunks_add_in_order() {
        let x_steps = step_values(-2.0, 2.0, 21);
        let y_steps = step_values(-2.0, 2.0, 21);
        let mut points: Vec<(f64, f64, f64)> = (0..(CHUNK + 10))
            .map(|index| ((index % 7) as f64 / 7.0, (index % 5) as f64 / 5.0, 1.0))
            .collect();
        let even = options().density(&points, &x_steps, &y_steps);
        // Doubling every weight changes nothing (the density is normalised).
        for point in &mut points {
            point.2 = 2.0;
        }
        let doubled = options().density(&points, &x_steps, &y_steps);
        for (left, right) in even.iter().zip(&doubled) {
            assert!((left - right).abs() < 1e-12);
        }
    }

    fn parse(options: &str) -> Result<Density2dOptions> {
        let options: crate::spec::Options = serde_json::from_str(options).unwrap();
        Density2dOptions::from_parameters(&Parameters::new(&options, "layers[0]"))
    }

    #[test]
    fn options_report_bad_values() {
        let error = parse(r#"{"n": 1000}"#).unwrap_err().to_string();
        assert!(error.contains("from 2 to 999"), "{error}");
        assert!(error.contains("layers[0].n"), "{error}");
        let error = parse(r#"{"bw": "silverman"}"#).unwrap_err().to_string();
        assert!(error.contains("'nrd0', 'nrd'"), "{error}");
        let parsed = parse(r#"{"n": [20, 30], "bw": [0.5, 2]}"#).unwrap();
        assert_eq!((parsed.x_count, parsed.y_count), (20, 30));
        assert_eq!(parsed.y_bandwidth, Bandwidth::Fixed(2.0));
    }
}
