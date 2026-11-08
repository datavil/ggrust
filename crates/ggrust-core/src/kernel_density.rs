//! One-dimensional Gaussian kernel densities evaluated at given points, as
//! `scipy.stats.gaussian_kde(values)(at)`.
//!
//! A thin wrapper over [`crate::kde::GaussianKde`] with `D = 1`: the kernel
//! variance is the unbiased data variance times `factor ** 2` (Scott's rule by
//! default). Non-finite values are dropped first; non-finite evaluation points
//! give NaN densities.
//!
//! What it does not do: weights, groups (call it once per group), choosing the
//! evaluation points (the caller passes them), or lets-plot's `stat_density`
//! bandwidths (that stat follows R's `bw.nrd0` and friends).

use crate::Result;
use crate::kde::{BandwidthMethod, GaussianKde};

/// The density of `values` at each of `at`. `bandwidth` is scipy's `bw_method`.
pub fn kernel_density(values: &[f64], at: &[f64], bandwidth: BandwidthMethod) -> Result<Vec<f64>> {
    let points: Vec<[f64; 1]> = values
        .iter()
        .filter(|value| value.is_finite())
        .map(|value| [*value])
        .collect();
    let kde = GaussianKde::new(&points, bandwidth, "values")?;
    let queries: Vec<[f64; 1]> = at.iter().map(|value| [*value]).collect();
    let mut density = kde.evaluate(&queries);
    for (estimate, query) in density.iter_mut().zip(at) {
        if !query.is_finite() {
            *estimate = f64::NAN;
        }
    }
    Ok(density)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_non_finite_values_and_marks_non_finite_points() {
        let values = [0.0, 1.0, f64::NAN, 3.0, f64::INFINITY];
        let density = kernel_density(&values, &[1.0, f64::NAN], BandwidthMethod::Scott).unwrap();
        let expected = kernel_density(&[0.0, 1.0, 3.0], &[1.0], BandwidthMethod::Scott).unwrap();
        assert_eq!(density[0], expected[0]);
        assert!(density[1].is_nan());
    }

    #[test]
    fn constant_or_single_values_are_errors() {
        let error = kernel_density(&[2.0, 2.0, 2.0], &[2.0], BandwidthMethod::Scott).unwrap_err();
        assert!(error.to_string().contains("singular"), "{error}");
        let error = kernel_density(&[2.0, f64::NAN], &[2.0], BandwidthMethod::Scott).unwrap_err();
        assert!(error.to_string().contains("at least 2"), "{error}");
    }
}
