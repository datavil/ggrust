//! R's `stats::density()` with the gaussian kernel and `bw.nrd0`, as the
//! ggbeeswarm stats need it (vipor scales offsets by it, `priority="density"`
//! orders by it).
//!
//! Follows R's algorithm: linear binning of the data onto `max(n, 512)` points
//! (a power of two above 512) spanning the data plus 7 bandwidths each side, a
//! kernel sum over the bins (R does the same sum by FFT), then linear
//! interpolation onto `n` points from `min - 3 bw` to `max + 3 bw`. Kernel
//! ordinates use R 4.4's coordinates (`old.coords = FALSE`).
//!
//! What it does not do: weights, other kernels or bandwidth rules (the violin
//! density in [`super::density`] has those, with lets-plot's grid).

use super::density::Bandwidth;

/// A density estimate on its grid.
pub struct RDensity {
    /// Evaluation points, increasing.
    pub grid: Vec<f64>,
    /// Density at each grid point.
    pub density: Vec<f64>,
}

impl RDensity {
    /// `density(values, n = n, adjust = adjust)`; `None` with fewer than two
    /// finite values (R's `bw.nrd0` needs two).
    pub fn estimate(values: &[f64], n: usize, adjust: f64) -> Option<RDensity> {
        let mut sorted: Vec<f64> = values
            .iter()
            .copied()
            .filter(|value| value.is_finite())
            .collect();
        if sorted.len() < 2 || n < 2 {
            return None;
        }
        sorted.sort_by(f64::total_cmp);
        let bandwidth = Bandwidth::Nrd0.of(&sorted) * adjust;
        let (low, high) = (sorted[0], sorted[sorted.len() - 1]);
        let from = low - 3.0 * bandwidth;
        let to = high + 3.0 * bandwidth;
        let bins = if n > 512 { n.next_power_of_two() } else { 512 };
        let lower = from - 4.0 * bandwidth;
        let upper = to + 4.0 * bandwidth;
        let mass = bin_distribution(&sorted, lower, upper, bins);

        // Kernel ordinates at the bin spacing (R >= 4.4, `old.coords = FALSE`).
        let spacing = (upper - lower) / (bins - 1) as f64;
        let normal = 1.0 / (bandwidth * (2.0 * std::f64::consts::PI).sqrt());
        let kernel: Vec<f64> = (0..bins)
            .map(|step| {
                let distance = step as f64 * spacing / bandwidth;
                normal * (-0.5 * distance * distance).exp()
            })
            .collect();
        let mut binned = vec![0.0; bins];
        for (source, weight) in mass.iter().enumerate() {
            if *weight == 0.0 {
                continue;
            }
            for (target, value) in binned.iter_mut().enumerate() {
                *value += weight * kernel[source.abs_diff(target)];
            }
        }
        for value in &mut binned {
            *value = value.max(0.0);
        }
        let binned_grid = sequence(lower, upper, bins);
        let grid = sequence(from, to, n);
        let density = grid
            .iter()
            .map(|point| interpolate(&binned_grid, &binned, *point))
            .collect();
        Some(RDensity { grid, density })
    }

    /// Linear interpolation of the density at `value` (R `approx`, ends held).
    pub fn at(&self, value: f64) -> f64 {
        interpolate(&self.grid, &self.density, value)
    }
}

/// R `BinDist`: each value's weight `1 / count` split linearly between its two
/// neighbouring grid points.
fn bin_distribution(values: &[f64], lower: f64, upper: f64, bins: usize) -> Vec<f64> {
    let mut mass = vec![0.0; bins];
    let weight = 1.0 / values.len() as f64;
    let step = (upper - lower) / (bins - 1) as f64;
    let last = bins as i64 - 2;
    for value in values {
        let position = (value - lower) / step;
        let index = position.floor() as i64;
        let fraction = position - index as f64;
        if (0..=last).contains(&index) {
            mass[index as usize] += weight * (1.0 - fraction);
            mass[index as usize + 1] += weight * fraction;
        } else if index == -1 {
            mass[0] += weight * fraction;
        } else if index == last + 1 {
            mass[index as usize] += weight * (1.0 - fraction);
        }
    }
    mass
}

/// R `seq(from, to, length.out = n)`: exact ends.
fn sequence(from: f64, to: f64, n: usize) -> Vec<f64> {
    let step = (to - from) / (n - 1) as f64;
    (0..n)
        .map(|index| match index {
            0 => from,
            _ if index == n - 1 => to,
            _ => from + index as f64 * step,
        })
        .collect()
}

/// Linear interpolation on an increasing grid; the end values outside it.
pub(super) fn interpolate(grid: &[f64], values: &[f64], t: f64) -> f64 {
    let index = grid.partition_point(|point| *point < t);
    match index {
        0 => values.first().copied().unwrap_or(f64::NAN),
        _ if index >= grid.len() => values.last().copied().unwrap_or(f64::NAN),
        _ => {
            let (left, right) = (grid[index - 1], grid[index]);
            let share = (t - left) / (right - left);
            values[index - 1] + share * (values[index] - values[index - 1])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_r_density() {
        // R: d <- density(c(1, 2, 2.5, 4, 7), n = 5); d$x; d$y
        let estimate = RDensity::estimate(&[1.0, 2.0, 2.5, 4.0, 7.0], 5, 1.0).unwrap();
        let grid = [-1.9207539, 1.0396231, 4.0, 6.9603769, 9.9207539];
        let density = [
            0.00094066754,
            0.15966218053,
            0.11831910868,
            0.08266460374,
            0.00091311899,
        ];
        for (actual, expected) in estimate.grid.iter().zip(grid) {
            assert!((actual - expected).abs() < 1e-6, "{actual} vs {expected}");
        }
        for (actual, expected) in estimate.density.iter().zip(density) {
            assert!((actual - expected).abs() < 1e-6, "{actual} vs {expected}");
        }
    }

    #[test]
    fn needs_two_values() {
        assert!(RDensity::estimate(&[1.0], 512, 1.0).is_none());
        assert!(RDensity::estimate(&[1.0, f64::NAN], 512, 1.0).is_none());
    }
}
