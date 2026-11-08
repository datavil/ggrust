//! Gaussian kernel density estimates with a full covariance: a port of
//! `scipy.stats.gaussian_kde` (scipy 1.17, unweighted points) in `D` dimensions.
//!
//! The kernel covariance is the data covariance (`np.cov`, unbiased) times
//! `factor ** 2`, where `factor` is Scott's `neff ** (-1 / (D + 4))` (the default),
//! Silverman's `(neff * (D + 2) / 4) ** (-1 / (D + 4))`, or a given number. Points
//! and queries are whitened by the kernel's lower Cholesky factor, then the
//! estimate at a query is `sum_i w_i exp(-|z_i - q|^2 / 2) / ((2 pi)^(D/2) prod L_kk)`.
//!
//! The operation order follows scipy on arm64 macOS (measured, bit-identical for
//! `D = 2`): the covariance dot products and the kernel sums are sequential fused
//! multiply-adds, the Cholesky factor and triangular solves multiply by inverse
//! diagonals (OpenBLAS / Accelerate style). `neff` (a BLAS `ddot` in scipy) may
//! differ by one ulp, which moves the result by about 1e-16 relative.
//!
//! In 1D the variance dot product is an Accelerate `ddot` in scipy, which is not
//! reproduced: densities differ by about 1e-15 of the peak (bit-identical for
//! small samples).
//!
//! What it does not do: point weights, `integrate_*`, `resample`, `logpdf`.

use rayon::prelude::*;

use crate::numeric::pairwise_sum;
use crate::{Error, Result};

/// How the kernel bandwidth factor is chosen (scipy's `bw_method`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BandwidthMethod {
    /// Scott's rule, `neff ** (-1 / (D + 4))` (scipy's default).
    Scott,
    /// Silverman's rule, `(neff * (D + 2) / 4) ** (-1 / (D + 4))`.
    Silverman,
    /// A fixed factor that multiplies the data standard deviations.
    Factor(f64),
}

/// A fitted kernel density estimate over `D`-dimensional points.
#[derive(Debug, Clone)]
pub struct GaussianKde<const D: usize> {
    /// Points solved by the kernel's Cholesky factor.
    whitened: Vec<[f64; D]>,
    /// Lower Cholesky factor of the kernel covariance (`cho_cov`).
    cholesky: [[f64; D]; D],
    /// `1 / n`, the weight of every point.
    weight: f64,
    /// `(2 pi)^(-D/2) / prod L_kk`.
    norm: f64,
    /// The bandwidth factor (scipy's `kde.factor`).
    factor: f64,
}

impl<const D: usize> GaussianKde<D> {
    /// Fits the estimate to `points` (all finite). `path` names the points in errors.
    pub fn new(points: &[[f64; D]], bandwidth: BandwidthMethod, path: &str) -> Result<Self> {
        let n = points.len();
        if n < 2 || n <= D {
            let reason = if D == 1 {
                format!("a kernel density needs at least 2 finite values, got {n}")
            } else {
                format!("a {D}D kernel density needs more than {D} finite points, got {n}")
            };
            return Err(Error::spec(path, reason));
        }
        if let Some(index) = points
            .iter()
            .position(|point| point.iter().any(|value| !value.is_finite()))
        {
            return Err(Error::spec(
                path,
                format!("point {index} is not finite; drop non-finite rows first"),
            ));
        }
        // np.ones(n) / n
        let weight = 1.0 / n as f64;
        let weights = vec![weight; n];
        let squared: Vec<f64> = weights.iter().map(|w| w * w).collect();
        // 1 / np.vecdot(w, w): a BLAS ddot in scipy (summed in order here).
        let neff = 1.0 / weights.iter().fold(0.0, |sum, w| w.mul_add(*w, sum));
        let dimensions = D as f64;
        let factor = match bandwidth {
            BandwidthMethod::Scott => neff.powf(-1.0 / (dimensions + 4.0)),
            BandwidthMethod::Silverman => {
                (neff * (dimensions + 2.0) / 4.0).powf(-1.0 / (dimensions + 4.0))
            }
            BandwidthMethod::Factor(value) => {
                if !(value.is_finite() && value > 0.0) {
                    return Err(Error::spec(
                        "bandwidth",
                        format!(
                            "a bandwidth factor must be a positive finite number, got {value}; \
                             or pass 'scott' or 'silverman'"
                        ),
                    ));
                }
                value
            }
        };

        let covariance = covariance(points, &weights, &squared);
        if covariance.iter().flatten().any(|value| !value.is_finite()) {
            return Err(Error::spec(
                path,
                "the points' covariance overflows a float (coordinates too large); \
                 rescale the points",
            ));
        }
        let Some(data_cholesky) = cholesky(&covariance) else {
            let reason = if D == 1 {
                format!(
                    "the {n} values are all equal (their variance is zero, a singular \
                     covariance), so they have no density; pass values that spread"
                )
            } else {
                format!(
                    "the {n} points lie on a line or a point (their covariance is singular), \
                     so they have no {D}D density; pass points that spread in every dimension"
                )
            };
            return Err(Error::spec(path, reason));
        };
        let mut cholesky = data_cholesky;
        for row in cholesky.iter_mut() {
            for value in row.iter_mut() {
                *value *= factor;
            }
        }
        let mut norm = (2.0 * std::f64::consts::PI).powf(-dimensions / 2.0);
        for (index, row) in cholesky.iter().enumerate() {
            norm /= row[index];
        }
        if !(norm.is_finite() && norm > 0.0) {
            return Err(Error::spec(
                "bandwidth",
                format!(
                    "the kernel bandwidth (factor {factor} times the data spread) is too small \
                     or too large for a float; rescale the points or pass another bandwidth"
                ),
            ));
        }
        let whitened = points
            .iter()
            .map(|point| solve_lower(&cholesky, point))
            .collect();
        Ok(GaussianKde {
            whitened,
            cholesky,
            weight,
            norm,
            factor,
        })
    }

    /// The bandwidth factor (scipy's `kde.factor`).
    pub fn factor(&self) -> f64 {
        self.factor
    }

    /// The density at each query point (scipy's `kde(points)`), in parallel over
    /// queries; each sum runs over the points in order, as scipy.
    pub fn evaluate(&self, queries: &[[f64; D]]) -> Vec<f64> {
        let evaluate_one = |query: &[f64; D]| {
            let query = solve_lower(&self.cholesky, query);
            let mut estimate = 0.0;
            for point in &self.whitened {
                let mut argument = 0.0;
                for (a, b) in point.iter().zip(&query) {
                    let residual = a - b;
                    argument = residual.mul_add(residual, argument);
                }
                let kernel = (-argument / 2.0).exp() * self.norm;
                estimate = self.weight.mul_add(kernel, estimate);
            }
            estimate
        };
        if queries.len() * self.whitened.len() < 50_000 {
            queries.iter().map(evaluate_one).collect()
        } else {
            queries
                .par_iter()
                .with_min_len(16)
                .map(evaluate_one)
                .collect()
        }
    }
}

/// `np.cov(points.T, bias=False, aweights=weights)`.
fn covariance<const D: usize>(
    points: &[[f64; D]],
    weights: &[f64],
    squared_weights: &[f64],
) -> [[f64; D]; D] {
    let weight_sum = pairwise_sum(weights);
    let mut means = [0.0; D];
    let mut column = vec![0.0; points.len()];
    for (dimension, mean) in means.iter_mut().enumerate() {
        for ((value, point), weight) in column.iter_mut().zip(points).zip(weights) {
            *value = point[dimension] * weight;
        }
        *mean = pairwise_sum(&column) / weight_sum;
    }
    // fact = w_sum - ddof * sum(w * aweights) / w_sum
    let fact = weight_sum - pairwise_sum(squared_weights) / weight_sum;
    let scale = 1.0 / fact;
    let mut result = [[0.0; D]; D];
    for i in 0..D {
        for j in 0..D {
            let mut sum = 0.0;
            for (point, weight) in points.iter().zip(weights) {
                let a = point[i] - means[i];
                let b = (point[j] - means[j]) * weight;
                sum = a.mul_add(b, sum);
            }
            result[i][j] = sum * scale;
        }
    }
    result
}

/// Lower Cholesky factor (`scipy.linalg.cholesky(lower=True)`); `None` when the
/// matrix is not positive definite.
fn cholesky<const D: usize>(matrix: &[[f64; D]; D]) -> Option<[[f64; D]; D]> {
    let mut lower = [[0.0_f64; D]; D];
    for j in 0..D {
        let mut diagonal = matrix[j][j];
        for value in &lower[j][..j] {
            diagonal = (-value).mul_add(*value, diagonal);
        }
        // Also rejects NaN.
        if diagonal.is_nan() || diagonal <= 0.0 {
            return None;
        }
        let diagonal = diagonal.sqrt();
        lower[j][j] = diagonal;
        let inverse = 1.0 / diagonal;
        for i in j + 1..D {
            let mut value = matrix[i][j];
            let (above, below) = lower.split_at_mut(i);
            for (a, b) in below[0][..j].iter().zip(&above[j][..j]) {
                value = (-a).mul_add(*b, value);
            }
            lower[i][j] = value * inverse;
        }
    }
    Some(lower)
}

/// `scipy.linalg.solve_triangular(lower, vector, lower=True)`.
fn solve_lower<const D: usize>(lower: &[[f64; D]; D], vector: &[f64; D]) -> [f64; D] {
    let mut solution = [0.0; D];
    for (k, row) in lower.iter().enumerate() {
        let mut value = vector[k];
        for (factor, solved) in row[..k].iter().zip(&solution[..k]) {
            value -= factor * solved;
        }
        solution[k] = value * (1.0 / row[k]);
    }
    solution
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_dimensional_density_integrates_to_one() {
        let points: Vec<[f64; 1]> = (0..50).map(|i| [(i as f64 * 0.37).sin()]).collect();
        let kde = GaussianKde::new(&points, BandwidthMethod::Scott, "points").unwrap();
        let step = 0.01;
        let queries: Vec<[f64; 1]> = (0..800).map(|i| [-4.0 + i as f64 * step]).collect();
        let total: f64 = kde.evaluate(&queries).iter().sum::<f64>() * step;
        assert!((total - 1.0).abs() < 1e-6, "{total}");
        assert!((kde.factor() - 50f64.powf(-0.2)).abs() < 1e-15);
    }

    #[test]
    fn cholesky_of_a_known_matrix() {
        let lower = cholesky(&[[4.0, 2.0], [2.0, 5.0]]).unwrap();
        assert_eq!(lower, [[2.0, 0.0], [1.0, 2.0]]);
        assert!(cholesky(&[[1.0, 1.0], [1.0, 1.0]]).is_none());
        assert_eq!(solve_lower(&lower, &[2.0, 5.0]), [1.0, 2.0]);
    }

    #[test]
    fn collinear_and_too_few_points_are_errors() {
        let line: Vec<[f64; 2]> = (0..10).map(|i| [i as f64, 2.0 * i as f64]).collect();
        let error = GaussianKde::new(&line, BandwidthMethod::Scott, "points").unwrap_err();
        assert!(error.to_string().contains("singular"), "{error}");
        let error = GaussianKde::new(&[[0.0, 1.0]], BandwidthMethod::Scott, "points").unwrap_err();
        assert!(error.to_string().contains("more than 2"), "{error}");
        let points = [[0.0, 1.0], [1.0, 0.0], [2.0, 2.0]];
        let error = GaussianKde::new(&points, BandwidthMethod::Factor(-1.0), "points").unwrap_err();
        assert!(error.to_string().contains("bandwidth"), "{error}");
    }
}
