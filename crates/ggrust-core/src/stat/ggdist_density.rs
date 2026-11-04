//! Sample densities as the R package ggdist (3.3) draws slabs: `density_bounded`
//! (default) and `density_unbounded`, with the Sheather-Jones direct plug-in
//! bandwidth (`bandwidth_dpi`, R's `bw.SJ(method = "dpi")`) or `nrd0`.
//!
//! ggdist is GPL; these follow its documented algorithms, not its code:
//!
//! - Bounds (`bounder_cdf`, p = 0.01): `2 min(x) - Q(1 - (1 - p)^(1/n))` and
//!   `2 max(x) - Q(p^(1/n))` (Q: R type 7 quantile). A side counts as bounded
//!   when its bound lies within 3 bandwidths of the data; otherwise it is open.
//! - Bounded density: R `density()` over the bounds widened by their width on
//!   every bounded side (`3n - 2` points for two sides), folded back by
//!   reflection, then interpolated onto `n` points over the data range (`trim`).
//! - Unbounded density: R `density()` from `min - cut bw` to `max + cut bw`
//!   (`cut` 0 when trimmed, else 3).
//! - Sheather-Jones: the two-stage plug-in of Sheather and Jones (1991), with
//!   the density functionals estimated from pair distances counted in 1000
//!   equal bins (as R's `bw.SJ`). It falls back to `nrd0` when the sample is too
//!   sparse, as ggdist does.
//! - Mode (`mode_qi`): the most frequent value of integer data, else the peak of
//!   a bounded density on 2001 points.
//!
//! What it does not do: weights, kernels other than gaussian, adaptive
//! bandwidths (`adapt`), histogram densities.

use super::boxplot::quantile;
use super::density::Bandwidth;
use super::r_density::{RDensity, interpolate, sequence};

/// How a slab's density treats the ends of the data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DensityKind {
    /// ggdist `density_bounded()`: reflection at estimated bounds.
    Bounded,
    /// ggdist `density_unbounded()`: plain R `density()`.
    Unbounded,
}

/// A bandwidth rule (or a fixed bandwidth) before `adjust`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BandwidthRule {
    /// Sheather-Jones direct plug-in (ggdist default).
    Dpi,
    /// R `bw.nrd0`.
    Nrd0,
    /// R `bw.nrd`.
    Nrd,
    /// A bandwidth in data units.
    Fixed(f64),
}

impl BandwidthRule {
    /// The bandwidth of sorted finite values (two or more). `true` in the second
    /// place: the rule failed and fell back to `nrd0` (ggdist warns then).
    pub fn of(self, sorted: &[f64]) -> (f64, bool) {
        match self {
            BandwidthRule::Fixed(value) => (value, false),
            BandwidthRule::Nrd0 => (Bandwidth::Nrd0.of(sorted), false),
            BandwidthRule::Nrd => (Bandwidth::Nrd.of(sorted), false),
            BandwidthRule::Dpi => match sheather_jones_dpi(sorted) {
                Some(bandwidth) => (bandwidth, false),
                None => (Bandwidth::Nrd0.of(sorted), true),
            },
        }
    }
}

/// A density on its grid, with the empirical CDF at each grid point.
#[derive(Debug, Clone, PartialEq)]
pub struct SampleDensity {
    /// Evaluation points, increasing.
    pub grid: Vec<f64>,
    /// Density at each point.
    pub density: Vec<f64>,
    /// Share of the sample at or below each point.
    pub cdf: Vec<f64>,
    /// The bandwidth used (after `adjust`).
    pub bandwidth: f64,
}

/// The density of sorted finite `values` (two or more, not all equal) on `n >= 2`
/// points.
pub fn sample_density(
    sorted: &[f64],
    kind: DensityKind,
    bandwidth: f64,
    n: usize,
    trim: bool,
) -> SampleDensity {
    let (low, high) = (sorted[0], sorted[sorted.len() - 1]);
    let (grid, density) = match kind {
        DensityKind::Unbounded => {
            let cut = if trim { 0.0 } else { 3.0 };
            let estimate = RDensity::on_grid(
                sorted,
                bandwidth,
                n,
                low - cut * bandwidth,
                high + cut * bandwidth,
            );
            (estimate.grid, estimate.density)
        }
        DensityKind::Bounded => bounded(sorted, bandwidth, n, trim),
    };
    let cdf = grid
        .iter()
        .map(|point| sorted.partition_point(|value| value <= point) as f64 / sorted.len() as f64)
        .collect();
    SampleDensity {
        grid,
        density,
        cdf,
        bandwidth,
    }
}

/// ggdist `density_bounded()` with bounds from `bounder_cdf()`.
fn bounded(sorted: &[f64], bandwidth: f64, n: usize, trim: bool) -> (Vec<f64>, Vec<f64>) {
    let (low, high) = (sorted[0], sorted[sorted.len() - 1]);
    let (mut lower, mut upper) = cdf_bounds(sorted);
    let (reach_low, reach_high) = (low - 3.0 * bandwidth, high + 3.0 * bandwidth);
    let left_bounded = reach_low <= lower;
    let right_bounded = upper <= reach_high;
    if !left_bounded {
        lower = reach_low;
    }
    if !right_bounded {
        upper = reach_high;
    }
    let width = upper - lower;
    let sides = usize::from(left_bounded) + usize::from(right_bounded);
    let total = n + sides * (n - 1);
    let from = if left_bounded { lower - width } else { lower };
    let to = if right_bounded { upper + width } else { upper };
    let wide = RDensity::on_grid(sorted, bandwidth, total, from, to);
    let start = if left_bounded { n - 1 } else { 0 };
    let mut grid: Vec<f64> = wide.grid[start..start + n].to_vec();
    let mut density: Vec<f64> = wide.density[start..start + n].to_vec();
    if left_bounded {
        // Mass beyond the lower bound, mirrored back: point i takes the wide
        // grid's point n - 1 - i.
        for (index, value) in density.iter_mut().enumerate() {
            *value += wide.density[n - 1 - index];
        }
    }
    if right_bounded {
        let last = wide.density.len() - 1;
        for (index, value) in density.iter_mut().enumerate() {
            *value += wide.density[last - index];
        }
    }
    if trim && (lower < low || upper > high) {
        let trimmed = sequence(low, high, n);
        density = trimmed
            .iter()
            .map(|point| interpolate(&grid, &density, *point))
            .collect();
        grid = trimmed;
    }
    (grid, density)
}

/// ggdist `bounder_cdf(p = 0.01)`: bounds from the order statistic distribution.
fn cdf_bounds(sorted: &[f64]) -> (f64, f64) {
    const P: f64 = 0.01;
    let share = P.powf(1.0 / sorted.len() as f64);
    let first = quantile(sorted, 1.0 - share);
    let last = quantile(sorted, share);
    (
        2.0 * sorted[0] - first,
        2.0 * sorted[sorted.len() - 1] - last,
    )
}

/// The Sheather-Jones direct plug-in bandwidth of sorted finite values (R
/// `bw.SJ(x, method = "dpi")`); `None` when the sample is too sparse.
pub fn sheather_jones_dpi(sorted: &[f64]) -> Option<f64> {
    const BINS: usize = 1000;
    let count = sorted.len();
    if count < 2 {
        return None;
    }
    let n = count as f64;
    let (bin_width, pairs) = pair_distance_counts(sorted, BINS);
    let mean = sorted.iter().sum::<f64>() / n;
    let deviation = (sorted
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (n - 1.0))
        .sqrt();
    let spread = (quantile(sorted, 0.75) - quantile(sorted, 0.25)) / 1.349;
    let scale = deviation.min(spread);
    // Stage one: a normal-reference pilot for the sixth-derivative functional.
    let pilot = 1.23 * scale * n.powf(-1.0 / 9.0);
    let sixth = -functional(&pairs, bin_width, count, pilot, 6);
    if !(sixth.is_finite() && sixth > 0.0) {
        return None;
    }
    // Stage two: the fourth-derivative functional at its plug-in bandwidth.
    let second_pilot = (2.394 / (n * sixth)).powf(1.0 / 7.0);
    let fourth = functional(&pairs, bin_width, count, second_pilot, 4);
    let bandwidth = (1.0 / (2.0 * std::f64::consts::PI.sqrt() * n) / fourth).powf(0.2);
    (bandwidth.is_finite() && bandwidth > 0.0).then_some(bandwidth)
}

/// Pairs of values counted by the distance between their bins: `bins` bins of
/// width `1.01 x range / bins`, a value in bin `trunc(x / width)`. Large samples
/// bin first and count pairs of bins (R's `bw.SJ` does too, for n > bins / 2).
fn pair_distance_counts(sorted: &[f64], bins: usize) -> (f64, Vec<f64>) {
    let range = sorted[sorted.len() - 1] - sorted[0];
    let width = range * 1.01 / bins as f64;
    let mut pairs = vec![0.0; bins];
    let bin_of = |value: f64| (value / width).trunc();
    if sorted.len() > bins / 2 {
        let first = bin_of(sorted[0]);
        let mut counts = vec![0.0; bins];
        for value in sorted {
            let index = ((bin_of(*value) - first) as usize).min(bins - 1);
            counts[index] += 1.0;
        }
        for (high, &weight) in counts.iter().enumerate() {
            if weight == 0.0 {
                continue;
            }
            pairs[0] += weight * (weight - 1.0) / 2.0;
            for (low, &other) in counts.iter().enumerate().take(high) {
                pairs[high - low] += weight * other;
            }
        }
    } else {
        let indices: Vec<i64> = sorted.iter().map(|value| bin_of(*value) as i64).collect();
        for (high, &bin) in indices.iter().enumerate() {
            for &other in &indices[..high] {
                let distance = (bin - other).unsigned_abs() as usize;
                pairs[distance.min(bins - 1)] += 1.0;
            }
        }
    }
    (width, pairs)
}

/// The binned estimate of the integrated squared `order`-th density derivative
/// functional (order 4 or 6) at bandwidth `h`: the gaussian `order`-th
/// derivative summed over every ordered pair of values, the n diagonal terms
/// included, over `n (n - 1) h^(order + 1) sqrt(2 pi)`.
fn functional(pairs: &[f64], bin_width: f64, count: usize, h: f64, order: u32) -> f64 {
    // Pairs further apart than sqrt(1000) bandwidths add nothing measurable.
    const LARGEST_SQUARED: f64 = 1000.0;
    let hermite = |u: f64| match order {
        4 => u * u - 6.0 * u + 3.0,
        _ => u * u * u - 15.0 * u * u + 45.0 * u - 15.0,
    };
    let mut sum = 0.0;
    for (distance, &weight) in pairs.iter().enumerate() {
        let scaled = distance as f64 * bin_width / h;
        let squared = scaled * scaled;
        if squared >= LARGEST_SQUARED {
            break;
        }
        sum += (-squared / 2.0).exp() * hermite(squared) * weight;
    }
    let n = count as f64;
    let total = 2.0 * sum + n * hermite(0.0);
    total / (n * (n - 1.0) * h.powi(order as i32 + 1) * (2.0 * std::f64::consts::PI).sqrt())
}

/// ggdist `Mode()` of sorted finite values: the most frequent value of integer
/// data (the first in data order among ties), else the peak of a bounded dpi
/// density on 2001 points. `data_order` is the sample in its original order.
pub fn mode(sorted: &[f64], data_order: &[f64]) -> f64 {
    let integers = sorted
        .iter()
        .all(|value| (value - value.round()).abs() < 1e-8_f64.sqrt());
    if integers || sorted.len() < 2 || sorted[0] == sorted[sorted.len() - 1] {
        let mut best = (f64::NAN, 0usize);
        for value in data_order {
            let start = sorted.partition_point(|other| other < value);
            let end = sorted.partition_point(|other| other <= value);
            if end - start > best.1 {
                best = (*value, end - start);
            }
        }
        return best.0;
    }
    let (bandwidth, _) = BandwidthRule::Dpi.of(sorted);
    let estimate = sample_density(sorted, DensityKind::Bounded, bandwidth, 2001, true);
    let mut peak = 0;
    for (index, value) in estimate.density.iter().enumerate() {
        if *value > estimate.density[peak] {
            peak = index;
        }
    }
    estimate.grid[peak]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Group A of the raincloud test data (also in `tests/fixtures`).
    fn sample() -> Vec<f64> {
        let mut values = vec![
            5.001, 4.299, 4.71, 3.93, 6.47, 5.45, 4.62, 4.78, 5.17, 3.49, 5.89, 4.33, 5.02, 3.56,
            5.1, 4.41, 6.33, 4.08, 5.83, 4.97,
        ];
        values.sort_by(f64::total_cmp);
        values
    }

    #[test]
    fn functional_counts_every_pair_once() {
        let (_, pairs) = pair_distance_counts(&sample(), 1000);
        let n = sample().len() as f64;
        assert_eq!(pairs.iter().sum::<f64>(), n * (n - 1.0) / 2.0);
    }

    #[test]
    fn binned_and_direct_pair_counts_agree_on_spread_data() {
        let values: Vec<f64> = (0..600).map(|index| index as f64 * 0.37).collect();
        let (_, binned) = pair_distance_counts(&values, 1000);
        let n = values.len() as f64;
        assert_eq!(binned.iter().sum::<f64>(), n * (n - 1.0) / 2.0);
    }

    #[test]
    fn bounded_density_spans_the_data_with_n_points() {
        let values = sample();
        let (bandwidth, fallback) = BandwidthRule::Dpi.of(&values);
        assert!(!fallback && bandwidth > 0.0);
        let estimate = sample_density(&values, DensityKind::Bounded, bandwidth, 501, true);
        assert_eq!(estimate.grid.len(), 501);
        assert_eq!(estimate.grid[0], values[0]);
        assert_eq!(estimate.grid[500], values[values.len() - 1]);
        assert_eq!(estimate.cdf[500], 1.0);
    }

    #[test]
    fn sparse_samples_fall_back_to_nrd0() {
        let values = [1.0, 1.0, 1.0, 1.0, 2.0];
        let (bandwidth, fallback) = BandwidthRule::Dpi.of(&values);
        assert!(fallback);
        assert_eq!(bandwidth, Bandwidth::Nrd0.of(&values));
    }

    #[test]
    fn mode_of_integers_is_the_most_frequent_value() {
        let data = [3.0, 1.0, 2.0, 2.0, 3.0];
        let mut sorted = data.to_vec();
        sorted.sort_by(f64::total_cmp);
        assert_eq!(mode(&sorted, &data), 3.0);
    }

    /// R reference numbers (`tests/fixtures/generate_ggdist_reference.R`).
    fn reference() -> serde_json::Value {
        serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ggdist_reference.json"
        ))
        .unwrap()
    }

    fn numbers(value: &serde_json::Value) -> Vec<f64> {
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item.as_f64().unwrap())
            .collect()
    }

    #[test]
    fn densities_and_bandwidths_match_ggdist() {
        let reference = reference();
        for (name, sample) in reference["samples"].as_object().unwrap() {
            let mut values = numbers(&sample["values"]);
            values.sort_by(f64::total_cmp);
            let expected_bandwidth = sample["bw_dpi"].as_f64().unwrap();
            let (bandwidth, fallback) = BandwidthRule::Dpi.of(&values);
            assert!(!fallback, "{name}");
            assert!(
                (bandwidth / expected_bandwidth - 1.0).abs() < 1e-9,
                "{name}: dpi {bandwidth} vs R {expected_bandwidth}"
            );
            let nrd0 = sample["bw_nrd0"].as_f64().unwrap();
            assert!((BandwidthRule::Nrd0.of(&values).0 / nrd0 - 1.0).abs() < 1e-12);
            for (kind, key) in [
                (DensityKind::Bounded, "bounded"),
                (DensityKind::Unbounded, "unbounded"),
            ] {
                let estimate = sample_density(&values, kind, bandwidth, 501, true);
                let (grid, density) = (numbers(&sample[key]["x"]), numbers(&sample[key]["y"]));
                let peak = density.iter().copied().fold(0.0, f64::max);
                for index in 0..501 {
                    assert!(
                        (estimate.grid[index] - grid[index]).abs() < 1e-9,
                        "{name} {key}"
                    );
                    let error = (estimate.density[index] - density[index]).abs() / peak;
                    assert!(error < 1e-9, "{name} {key} at {index}: {error}");
                }
            }
        }
    }

    #[test]
    fn modes_match_ggdist() {
        let reference = reference();
        for (name, sample) in reference["samples"].as_object().unwrap() {
            let data = numbers(&sample["values"]);
            let mut sorted = data.clone();
            sorted.sort_by(f64::total_cmp);
            let expected = numbers(&sample["mode_qi"]["point"])[0];
            assert!((mode(&sorted, &data) - expected).abs() < 1e-9, "{name}");
        }
    }
}
