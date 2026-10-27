//! Kernel density estimation shared by `ydensity` (violins) and, later,
//! `densityridges`, as lets-plot's `DensityStatUtil`.
//!
//! Bandwidths follow R (`bw.nrd0`, `bw.nrd`): sample standard deviation, type 7
//! IQR and R's fallbacks for constant data. lets-plot uses the population
//! standard deviation and its own quartiles there (a deliberate difference).
//! Densities are evaluated on `n` equally spaced points; above `fs_max` values
//! only those within 5 bandwidths are summed (lets-plot `densityFunctionFast`).
//!
//! What it does not do: weighted bandwidths (weights only scale the kernels).

use std::f64::consts::PI;

use super::boxplot::quantile;
use crate::parameters::Parameters;
use crate::spec::Value;
use crate::{Error, Result};

/// lets-plot `DensityStat.DEF_N`.
pub const DEFAULT_N: usize = 512;
/// lets-plot `DensityStat.MAX_N`.
const MAX_N: usize = 1024;
/// lets-plot `DensityStat.DEF_FULL_SCAN_MAX`.
const DEFAULT_FULL_SCAN_MAX: usize = 5000;

/// A smoothing kernel (lets-plot `DensityStat.Kernel`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kernel {
    Gaussian,
    Rectangular,
    Triangular,
    Biweight,
    Epanechikov,
    Optcosine,
    Cosine,
}

impl Kernel {
    /// lets-plot names, aliases included.
    fn parse(name: &str) -> Option<Kernel> {
        Some(match name {
            "gaussian" => Kernel::Gaussian,
            "rectangular" | "uniform" => Kernel::Rectangular,
            "triangular" => Kernel::Triangular,
            "biweight" | "quartic" => Kernel::Biweight,
            "epanechikov" | "parabolic" => Kernel::Epanechikov,
            "optcosine" => Kernel::Optcosine,
            "cosine" => Kernel::Cosine,
            _ => return None,
        })
    }

    fn weight(self, value: f64) -> f64 {
        let inside = value.abs() <= 1.0;
        match self {
            Kernel::Gaussian => (-0.5 * value * value).exp() / (2.0 * PI).sqrt(),
            Kernel::Rectangular if inside => 0.5,
            Kernel::Triangular if inside => 1.0 - value.abs(),
            Kernel::Biweight if inside => 0.9375 * (1.0 - value * value).powi(2),
            Kernel::Epanechikov if inside => 0.75 * (1.0 - value * value),
            Kernel::Optcosine if inside => PI / 4.0 * (PI / 2.0 * value).cos(),
            Kernel::Cosine if inside => ((PI * value).cos() + 1.0) / 2.0,
            _ => 0.0,
        }
    }
}

/// `bw`: a number, or the rule `nrd0` (default) / `nrd`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Bandwidth {
    Fixed(f64),
    Nrd0,
    Nrd,
}

impl Bandwidth {
    /// The bandwidth for sorted values.
    pub fn of(self, sorted: &[f64]) -> f64 {
        let factor = match self {
            Bandwidth::Fixed(value) => return value,
            Bandwidth::Nrd0 => 0.9,
            Bandwidth::Nrd => 1.06,
        };
        let count = sorted.len() as f64;
        let mean = sorted.iter().sum::<f64>() / count;
        let variance = sorted
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / (count - 1.0);
        let deviation = if variance.is_finite() {
            variance.sqrt()
        } else {
            0.0
        };
        let spread = (quantile(sorted, 0.75) - quantile(sorted, 0.25)) / 1.34;
        // R `bw.nrd0`: min(sd, IQR / 1.34), else sd, else |x[1]|, else 1.
        let low = [
            deviation.min(spread),
            deviation,
            sorted.first().map_or(0.0, |first| first.abs()),
            1.0,
        ]
        .into_iter()
        .find(|value| *value > 0.0 && value.is_finite())
        .unwrap_or(1.0);
        factor * low * count.powf(-0.2)
    }
}

/// Density options of a layer (`bw`, `adjust`, `kernel`, `n`, `fs_max`, `trim`,
/// `tails_cutoff`).
#[derive(Debug, Clone, PartialEq)]
pub struct DensityOptions {
    pub bandwidth: Bandwidth,
    pub adjust: f64,
    pub kernel: Kernel,
    pub n: usize,
    pub full_scan_max: usize,
    pub trim: bool,
    /// Bandwidths added beyond the data when not trimmed; `None`: the overall range.
    pub tails_cutoff: Option<f64>,
}

impl DensityOptions {
    /// Read the options; `trim` and `tails_cutoff` defaults differ per stat.
    pub fn from_parameters(
        parameters: &Parameters,
        default_trim: bool,
        default_tails_cutoff: Option<f64>,
    ) -> Result<Self> {
        let invalid = |name: &str, expected: &str, value: &Value| {
            Error::InvalidSpec(format!(
                "at '{}': expected {expected}, got {value}",
                parameters.path_of(name)
            ))
        };
        let bandwidth = match parameters.value("bw") {
            None => Bandwidth::Nrd0,
            Some(Value::String(name)) if name == "nrd0" => Bandwidth::Nrd0,
            Some(Value::String(name)) if name == "nrd" => Bandwidth::Nrd,
            Some(value) => match value.as_f64() {
                Some(number) if number.is_finite() && number > 0.0 => Bandwidth::Fixed(number),
                _ => return Err(invalid("bw", "a positive number, 'nrd0' or 'nrd'", value)),
            },
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
        if adjust <= 0.0 {
            return Err(Error::InvalidSpec(format!(
                "at '{}': expected a positive number, got {adjust}",
                parameters.path_of("adjust")
            )));
        }
        let count = |name: &str, default: usize| -> Result<usize> {
            match parameters.number(name)? {
                None => Ok(default),
                Some(value) if value >= 2.0 => Ok(value as usize),
                Some(value) => Err(Error::InvalidSpec(format!(
                    "at '{}': expected a whole number of at least 2, got {value}",
                    parameters.path_of(name)
                ))),
            }
        };
        let n = count("n", DEFAULT_N)?;
        if n > MAX_N {
            return Err(Error::InvalidSpec(format!(
                "at '{}': n = {n} is too large; at most {MAX_N}",
                parameters.path_of("n")
            )));
        }
        Ok(DensityOptions {
            bandwidth,
            adjust,
            kernel,
            n,
            full_scan_max: count("fs_max", DEFAULT_FULL_SCAN_MAX)?,
            trim: parameters.boolean("trim")?.unwrap_or(default_trim),
            tails_cutoff: match parameters.value("tails_cutoff") {
                None => default_tails_cutoff,
                Some(_) => parameters.number("tails_cutoff")?,
            },
        })
    }
}

/// The density of one bin of values on its grid (lets-plot `binnedStat`).
pub struct Estimate {
    /// Grid positions, ascending.
    pub grid: Vec<f64>,
    /// Kernel sums (weighted counts per unit).
    pub count: Vec<f64>,
    /// `count` / total weight.
    pub density: Vec<f64>,
    /// `count` / its maximum.
    pub scaled: Vec<f64>,
}

/// Estimate the density of `sorted` values with `weights` (same order).
/// `overall` is the range used when not trimmed and without a tails cutoff.
pub fn estimate(
    sorted: &[f64],
    weights: &[f64],
    options: &DensityOptions,
    overall: (f64, f64),
) -> Estimate {
    let bandwidth = options.bandwidth.of(sorted);
    let (first, last) = (sorted[0], sorted[sorted.len() - 1]);
    let (low, high) = match (options.trim, options.tails_cutoff) {
        (true, _) => (first, last),
        (false, None) => overall,
        (false, Some(cutoff)) => (first - cutoff * bandwidth, last + cutoff * bandwidth),
    };
    let grid = step_values(low, high, options.n);
    let h = bandwidth * options.adjust;
    let fast = sorted.len() > options.full_scan_max;
    let count: Vec<f64> = grid
        .iter()
        .map(|&at| {
            let (from, to) = if fast {
                // lets-plot `densityFunctionFast`: values within 5 bandwidths.
                (
                    sorted.partition_point(|value| *value < at - 5.0 * h),
                    sorted.partition_point(|value| *value < at + 5.0 * h),
                )
            } else {
                (0, sorted.len())
            };
            let sum: f64 = (from..to)
                .map(|index| options.kernel.weight((at - sorted[index]) / h) * weights[index])
                .sum();
            sum / h
        })
        .collect();
    let total: f64 = weights.iter().sum();
    let largest = count.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    Estimate {
        density: count.iter().map(|value| value / total).collect(),
        scaled: count.iter().map(|value| value / largest).collect(),
        grid,
        count,
    }
}

/// lets-plot `createStepValues`: `n` points from low to high (a zero-width range
/// is widened by 0.5 each side).
fn step_values(mut low: f64, mut high: f64, n: usize) -> Vec<f64> {
    if high == low {
        low -= 0.5;
        high += 0.5;
    }
    let step = (high - low) / (n - 1) as f64;
    (0..n).map(|index| low + step * index as f64).collect()
}

/// lets-plot `calculateStatQuantile`: the quantile band of every grid point (the
/// smallest of `quantiles` whose value it does not exceed, else 1), from the
/// cumulative kernel sums.
pub fn quantile_bands(grid: &[f64], count: &[f64], quantiles: &[f64]) -> Vec<f64> {
    if quantiles.is_empty() {
        return vec![1.0; grid.len()];
    }
    let total: f64 = count.iter().sum();
    let mut running = 0.0;
    let cumulative: Vec<f64> = count
        .iter()
        .map(|value| {
            running += value;
            running / total
        })
        .collect();
    let mut sorted = quantiles.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted.dedup();
    let mut index = 0;
    grid.iter()
        .map(|&value| {
            if value <= interpolate(&cumulative, grid, sorted[index]) {
                sorted[index]
            } else if index + 1 < sorted.len() {
                index += 1;
                sorted[index]
            } else {
                1.0
            }
        })
        .collect()
}

/// lets-plot `pwLinInterp`: y at `t` on the piecewise linear (x, y), clamped.
fn interpolate(x: &[f64], y: &[f64], t: f64) -> f64 {
    match x.iter().position(|value| *value >= t) {
        Some(0) => y[0],
        None => y[y.len() - 1],
        Some(index) => {
            // As lets-plot writes it (`a * t + b`), so exact ties round the same.
            let slope = (y[index] - y[index - 1]) / (x[index] - x[index - 1]);
            let intercept = y[index - 1] - slope * x[index - 1];
            slope * t + intercept
        }
    }
}

/// lets-plot `expandByGroupEnds`: where the quantile band changes, repeat the
/// previous point under the new band so adjacent bands share their edge.
/// `columns[0]` is the grid (axis) and `columns[band]` the band of each point.
pub fn expand_band_ends(columns: &mut [Vec<f64>], band: usize) {
    let length = columns[0].len();
    let mut expanded: Vec<Vec<f64>> = columns.iter().map(|_| Vec::with_capacity(length)).collect();
    for row in 0..length {
        if row > 0 && columns[band][row - 1] != columns[band][row] {
            for (index, column) in columns.iter().enumerate() {
                let source = if index == band { row } else { row - 1 };
                expanded[index].push(column[source]);
            }
        }
        for (index, column) in columns.iter().enumerate() {
            expanded[index].push(column[row]);
        }
    }
    for (column, values) in columns.iter_mut().zip(expanded) {
        *column = values;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bandwidths_follow_r() {
        // Rscript: bw.nrd0(c(1, 2, 3, 4, 5, 6, 9)), bw.nrd(...).
        let values = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 9.0];
        assert!((Bandwidth::Nrd0.of(&values) - 1.365_335_422_5).abs() < 1e-9);
        assert!((Bandwidth::Nrd.of(&values) - 1.608_061_719_9).abs() < 1e-9);
        // Constant data: |x[1]| (Rscript: bw.nrd0(c(2, 2)) = 1.5669910139); one value too.
        assert!((Bandwidth::Nrd0.of(&[2.0, 2.0]) - 1.566_991_013_9).abs() < 1e-9);
        assert_eq!(Bandwidth::Nrd0.of(&[0.0]), 0.9);
    }

    #[test]
    fn gaussian_density_integrates_to_one() {
        let options = DensityOptions {
            bandwidth: Bandwidth::Fixed(0.5),
            adjust: 1.0,
            kernel: Kernel::Gaussian,
            n: 512,
            full_scan_max: 5000,
            trim: false,
            tails_cutoff: Some(6.0),
        };
        let values = [1.0, 2.0, 2.5, 4.0];
        let estimate = estimate(&values, &[1.0; 4], &options, (0.0, 0.0));
        let step = estimate.grid[1] - estimate.grid[0];
        let area: f64 = estimate.density.iter().sum::<f64>() * step;
        assert!((area - 1.0).abs() < 1e-3, "{area}");
        assert_eq!(estimate.scaled.iter().copied().fold(0.0, f64::max), 1.0);
    }

    #[test]
    fn band_ends_are_shared() {
        let mut columns = vec![vec![1.0, 2.0, 3.0], vec![0.25, 0.5, 0.5]];
        expand_band_ends(&mut columns, 1);
        assert_eq!(columns[0], [1.0, 1.0, 2.0, 3.0]);
        assert_eq!(columns[1], [0.25, 0.5, 0.5, 0.5]);
    }
}
