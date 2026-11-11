//! Kernel density estimation shared by `ydensity` (violins) and, later,
//! `densityridges`, as lets-plot's `DensityStatUtil`.
//!
//! Bandwidths follow R (`bw.nrd0`, `bw.nrd`): sample standard deviation, type 7
//! IQR and R's fallbacks for constant data. lets-plot uses the population
//! standard deviation and its own quartiles there (a deliberate difference).
//! Densities are evaluated on `n` equally spaced points; above `fs_max` values
//! only those within 5 bandwidths are summed (lets-plot `densityFunctionFast`).
//!
//! `bounds` (ggplot2): values outside are dropped, and the kernels are reflected at
//! each finite bound, so mass that would leak past it is folded back (summed
//! exactly at each grid point, where ggplot2 interpolates a wider estimate).
//!
//! What it does not do: weighted bandwidths (weights only scale the kernels).

use std::collections::HashMap;
use std::f64::consts::PI;

use rayon::prelude::*;

use super::boxplot::quantile;
use crate::data::ContinuousColumn;
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
    pub(super) fn parse(name: &str) -> Option<Kernel> {
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

    pub(super) fn weight(self, value: f64) -> f64 {
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
    /// Known lower and upper bounds of the data (infinite: none).
    pub bounds: (f64, f64),
}

impl DensityOptions {
    /// Read the options; `trim` and `tails_cutoff` defaults differ per stat.
    pub fn from_parameters(
        parameters: &Parameters,
        default_trim: bool,
        default_tails_cutoff: Option<f64>,
    ) -> Result<Self> {
        let invalid = |name: &str, expected: &str, value: &Value| {
            Error::spec(
                parameters.path_of(name),
                format!("expected {expected}, got {value}"),
            )
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
            return Err(Error::spec(
                parameters.path_of("adjust"),
                format!("expected a positive number, got {adjust}"),
            ));
        }
        let count = |name: &str, default: usize| -> Result<usize> {
            match parameters.number(name)? {
                None => Ok(default),
                Some(value) if value >= 2.0 => Ok(value as usize),
                Some(value) => Err(Error::spec(
                    parameters.path_of(name),
                    format!("expected a whole number of at least 2, got {value}"),
                )),
            }
        };
        let n = count("n", DEFAULT_N)?;
        if n > MAX_N {
            return Err(Error::spec(
                parameters.path_of("n"),
                format!("n = {n} is too large; at most {MAX_N}"),
            ));
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
            bounds: bounds(parameters)?,
        })
    }

    /// Whether a value lies within the bounds (data outside is dropped).
    pub fn within_bounds(&self, value: f64) -> bool {
        (self.bounds.0..=self.bounds.1).contains(&value)
    }
}

/// `bounds=[low, high]`: null (or infinity) is an open end.
fn bounds(parameters: &Parameters) -> Result<(f64, f64)> {
    let Some(value) = parameters.value("bounds") else {
        return Ok((f64::NEG_INFINITY, f64::INFINITY));
    };
    let invalid = || {
        Error::spec(
            parameters.path_of("bounds"),
            format!(
                "expected [low, high] with low < high, None (or inf) for an open end, \
                 got {value}"
            ),
        )
    };
    let Value::Array(items) = value else {
        return Err(invalid());
    };
    let [low, high] = items.as_slice() else {
        return Err(invalid());
    };
    let end = |item: &Value, open: f64| match item {
        Value::Null => Some(open),
        item => item.as_f64().filter(|number| !number.is_nan()),
    };
    match (end(low, f64::NEG_INFINITY), end(high, f64::INFINITY)) {
        (Some(low), Some(high)) if low < high => Ok((low, high)),
        _ => Err(invalid()),
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
    let (lower_bound, upper_bound) = options.bounds;
    let reflected = reflect(sorted, weights, options.bounds);
    let (sorted, weights) = match &reflected {
        Some((values, weights, _)) => (values.as_slice(), weights.as_slice()),
        None => (sorted, weights),
    };
    // ggplot2 `compute_density`: a finite bound widens the range by its width on
    // that side, then the grid stops at the bound.
    let width = high - low;
    let low = if lower_bound.is_finite() {
        (low - width).max(lower_bound)
    } else {
        low
    };
    let high = if upper_bound.is_finite() {
        (high + width).min(upper_bound)
    } else {
        high
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
    // Reflected copies fold mass back; they add no weight of their own.
    let total: f64 = weights.iter().sum::<f64>() / reflected.as_ref().map_or(1.0, |r| r.2);
    let largest = count.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    Estimate {
        density: count.iter().map(|value| value / total).collect(),
        scaled: count.iter().map(|value| value / largest).collect(),
        grid,
        count,
    }
}

/// The values with their mirror images at each finite bound, sorted, with their
/// weights and how many copies of the data that is; `None` without bounds.
fn reflect(
    sorted: &[f64],
    weights: &[f64],
    (low, high): (f64, f64),
) -> Option<(Vec<f64>, Vec<f64>, f64)> {
    let mirrors: Vec<f64> = [low, high].into_iter().filter(|b| b.is_finite()).collect();
    if mirrors.is_empty() {
        return None;
    }
    let mut pairs: Vec<(f64, f64)> = sorted
        .iter()
        .copied()
        .zip(weights.iter().copied())
        .collect();
    for bound in &mirrors {
        pairs.extend(
            sorted
                .iter()
                .zip(weights)
                .map(|(value, weight)| (2.0 * bound - value, *weight)),
        );
    }
    pairs.sort_by(|left, right| left.0.total_cmp(&right.0));
    let (values, weights) = pairs.into_iter().unzip();
    Some((values, weights, (mirrors.len() + 1) as f64))
}

/// Finite (value, weight) pairs of `rows` per finite bin, bins in first
/// appearance (lets-plot `handleBinnedData`). Without a bin column every row is in
/// bin 0; without weights every weight is 1.
pub fn collect_bins(
    rows: &[usize],
    bin: Option<&ContinuousColumn>,
    value: &ContinuousColumn,
    weight: Option<&ContinuousColumn>,
) -> Vec<(f64, Vec<(f64, f64)>)> {
    let mut bins: Vec<(f64, Vec<(f64, f64)>)> = Vec::new();
    let mut slot_by_bin: HashMap<u64, usize> = HashMap::new();
    for &row in rows {
        let location = bin.map_or(Some(0.0), |bin| bin.get(row));
        let weight = weight.map_or(Some(1.0), |weight| weight.get(row));
        let (Some(location), Some(value), Some(weight)) = (location, value.get(row), weight) else {
            continue;
        };
        if !(location.is_finite() && value.is_finite() && weight.is_finite()) {
            continue;
        }
        let slot = *slot_by_bin.entry(location.to_bits()).or_insert_with(|| {
            bins.push((location, Vec::new()));
            bins.len() - 1
        });
        bins[slot].1.push((value, weight));
    }
    bins
}

/// The densities of every bin (in parallel, joined in bin order) as columns bin,
/// grid value, density, count, scaled, quantile band (lets-plot `binnedStat`).
pub fn estimate_bins(
    bins: Vec<(f64, Vec<(f64, f64)>)>,
    options: &DensityOptions,
    quantiles: &[f64],
    overall: (f64, f64),
) -> [Vec<f64>; 6] {
    let per_bin: Vec<[Vec<f64>; 6]> = bins
        .into_par_iter()
        .map(|(location, mut pairs)| {
            pairs.sort_by(|left, right| left.0.total_cmp(&right.0));
            let (values, weights): (Vec<f64>, Vec<f64>) = pairs.into_iter().unzip();
            let estimate = estimate(&values, &weights, options, overall);
            let bands = quantile_bands(&estimate.grid, &estimate.count, quantiles);
            let mut bin = vec![
                estimate.grid,
                bands,
                estimate.density,
                estimate.count,
                estimate.scaled,
            ];
            expand_band_ends(&mut bin, 1);
            let [grid, bands, density, count, scaled] =
                <[Vec<f64>; 5]>::try_from(bin).expect("five columns");
            let locations = vec![location; grid.len()];
            [locations, grid, density, count, scaled, bands]
        })
        .collect();
    let mut columns: [Vec<f64>; 6] = Default::default();
    for bin in per_bin {
        for (column, values) in columns.iter_mut().zip(bin) {
            column.extend(values);
        }
    }
    columns
}

/// lets-plot `createStepValues`: `n` points from low to high (a zero-width range
/// is widened by 0.5 each side).
pub(super) fn step_values(mut low: f64, mut high: f64, n: usize) -> Vec<f64> {
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
            bounds: (f64::NEG_INFINITY, f64::INFINITY),
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
