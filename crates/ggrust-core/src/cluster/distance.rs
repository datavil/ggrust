//! Dissimilarities between the rows of a numeric matrix, for [`super::hclust`].
//!
//! `euclidean` is R's `dist()` (sum of squared differences in column order, then
//! the square root); `correlation` is `1 - r` with Pearson's `r` between rows, as
//! R's `as.dist(1 - cor(t(x)))` and scipy's `"correlation"` metric.
//!
//! What it does not do: missing values (R's `dist` rescales over the present
//! columns; ggrust rejects them), other metrics.

use super::condensed_index;
use crate::{Error, Result};

/// Dissimilarity between two rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Distance {
    /// Straight-line distance.
    Euclidean,
    /// One minus Pearson's correlation.
    Correlation,
}

impl Distance {
    /// Every distance with its name.
    pub const ALL: [(&'static str, Distance); 2] = [
        ("euclidean", Distance::Euclidean),
        ("correlation", Distance::Correlation),
    ];

    /// The distance with this name.
    pub fn from_name(name: &str) -> Option<Distance> {
        Distance::ALL
            .iter()
            .find(|(known, _)| *known == name)
            .map(|(_, distance)| *distance)
    }
}

/// Condensed dissimilarities between the rows of a row-major `labels.len() x
/// columns` matrix. Errors name the first row with a missing or infinite value
/// (or, under correlation, without variance).
pub fn dissimilarities(
    values: &[f64],
    columns: usize,
    labels: &[String],
    distance: Distance,
) -> Result<Vec<f64>> {
    let n = labels.len();
    let row = |index: usize| &values[index * columns..(index + 1) * columns];
    for (index, label) in labels.iter().enumerate() {
        if let Some(column) = row(index).iter().position(|value| !value.is_finite()) {
            return Err(Error::data(format!(
                "cannot cluster '{label}': value {} (column {}) is missing or infinite; fill or drop missing values first",
                row(index)[column],
                column + 1
            )));
        }
    }
    let rows: Vec<(Vec<f64>, f64)> = match distance {
        Distance::Euclidean => (0..n).map(|index| (row(index).to_vec(), 1.0)).collect(),
        Distance::Correlation => (0..n)
            .map(|index| centred(row(index)).ok_or(index))
            .collect::<std::result::Result<_, _>>()
            .map_err(|index| {
                Error::data(format!(
                    "cannot cluster '{}' by correlation: its values do not vary (or it has fewer than 2); use distance='euclidean' or drop the row",
                    labels[index]
                ))
            })?,
    };
    let mut condensed = vec![0.0; n * n.saturating_sub(1) / 2];
    for first in 0..n {
        for second in first + 1..n {
            let ((x, x_deviation), (y, y_deviation)) = (&rows[first], &rows[second]);
            condensed[condensed_index(n, first, second)] = match distance {
                Distance::Euclidean => x
                    .iter()
                    .zip(y)
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum::<f64>()
                    .sqrt(),
                Distance::Correlation => {
                    let covariance =
                        x.iter().zip(y).map(|(a, b)| a * b).sum::<f64>() / (columns as f64 - 1.0);
                    1.0 - (covariance / (x_deviation * y_deviation)).clamp(-1.0, 1.0)
                }
            };
        }
    }
    Ok(condensed)
}

/// The row minus its mean, and its standard deviation; None without variance.
///
/// The arithmetic is R's `cor()` (`cov.c`, complete cases): the mean is corrected
/// by the mean residual, sums of products are divided by `n - 1`, and the
/// correlation is the covariance over the product of standard deviations. Doing
/// the same steps matters for ties: integer data give exactly equal correlations
/// only when they are computed the same way.
fn centred(row: &[f64]) -> Option<(Vec<f64>, f64)> {
    if row.len() < 2 {
        return None;
    }
    let count = row.len() as f64;
    let mut mean = row.iter().sum::<f64>() / count;
    if mean.is_finite() {
        mean += row.iter().map(|value| value - mean).sum::<f64>() / count;
    }
    let centred: Vec<f64> = row.iter().map(|value| value - mean).collect();
    let variance = centred.iter().map(|value| value * value).sum::<f64>() / (count - 1.0);
    let deviation = variance.sqrt();
    (deviation > 0.0 && deviation.is_finite()).then_some((centred, deviation))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(n: usize) -> Vec<String> {
        (0..n).map(|index| format!("r{index}")).collect()
    }

    #[test]
    fn euclidean_and_correlation_distances() {
        let values = [0.0, 0.0, 3.0, 4.0, 6.0, 8.0];
        let euclidean = dissimilarities(&values, 2, &labels(3), Distance::Euclidean);
        assert_eq!(euclidean.expect("ok"), vec![5.0, 10.0, 5.0]);
        let values = [1.0, 2.0, 3.0, 2.0, 4.0, 6.0, 3.0, 2.0, 1.0];
        let correlation = dissimilarities(&values, 3, &labels(3), Distance::Correlation);
        let correlation = correlation.expect("ok");
        assert!(correlation[0].abs() < 1e-15);
        assert!((correlation[1] - 2.0).abs() < 1e-15);
    }

    #[test]
    fn missing_values_and_constant_rows_name_the_row() {
        let values = [1.0, f64::NAN];
        let error = dissimilarities(&values, 1, &labels(2), Distance::Euclidean);
        assert!(error.expect_err("nan").to_string().contains("'r1'"));
        let values = [1.0, 2.0, 5.0, 5.0];
        let error = dissimilarities(&values, 2, &labels(2), Distance::Correlation);
        let message = error.expect_err("constant").to_string();
        assert!(message.contains("'r1'") && message.contains("euclidean"));
    }
}
