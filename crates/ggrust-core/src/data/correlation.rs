//! Pearson correlation matrix of a frame's numeric columns, for `corr_plot`.
//!
//! Follows lets-plot's bistro `CorrUtil` / `correlationPearson`: every numeric
//! (integer or float) column is a variable, in frame order; other columns are
//! skipped. Each pair uses the rows where both values are finite (pairwise
//! complete, as pandas `DataFrame.corr`); fewer than 2 such rows or a constant
//! column give NaN. The diagonal is 1. A frame that already is a correlation
//! matrix (square, float columns, every value in -1..1 or NaN, as lets_plot's
//! docs describe) is returned as it is, its columns naming the variables.
//!
//! What it does not do: other methods (Spearman, Kendall), p-values.

use super::{Column, ContinuousColumn, DataFrame};
use crate::Result;
use crate::parallel;

/// A square matrix of correlations between named variables.
#[derive(Debug, Clone, PartialEq)]
pub struct CorrelationMatrix {
    /// Variable names, in frame order.
    pub names: Vec<String>,
    /// Row-major `names.len()` x `names.len()` correlations: row `i`, column `j` at
    /// `i * names.len() + j`.
    pub values: Vec<f64>,
    /// Whether the frame was read as a correlation matrix (not computed).
    pub given: bool,
}

/// The correlations between the numeric columns of `frame`, or the frame itself
/// when it already is a correlation matrix.
///
/// Errors only when a column cannot be read.
pub fn correlation_matrix(frame: &DataFrame) -> Result<CorrelationMatrix> {
    let mut names = Vec::new();
    let mut columns = Vec::new();
    let mut all_float = true;
    for name in frame.column_names() {
        let data_type = frame.data_type(name)?;
        all_float &= data_type.is_floating();
        if !data_type.is_numeric() {
            continue;
        }
        if let Column::Continuous(column) = frame.column(name)? {
            names.push(name.to_owned());
            columns.push(column);
        }
    }
    if all_float && is_coefficient_matrix(&columns, frame.row_count()) {
        let size = columns.len();
        let mut values = vec![f64::NAN; size * size];
        for (column_index, column) in columns.iter().enumerate() {
            for row in 0..size {
                values[row * size + column_index] = column.get(row).unwrap_or(f64::NAN);
            }
        }
        return Ok(CorrelationMatrix {
            names,
            values,
            given: true,
        });
    }
    let size = columns.len();
    let pairs: Vec<(usize, usize)> = (0..size)
        .flat_map(|row| (row + 1..size).map(move |column| (row, column)))
        .collect();
    let parallel = parallel::is_worth(pairs.len() * frame.row_count());
    let correlations = parallel::ordered_map(pairs.clone(), parallel, |(first, second)| {
        pearson(&columns[first], &columns[second])
    });
    let mut values = vec![1.0; size * size];
    for ((row, column), correlation) in pairs.into_iter().zip(correlations) {
        values[row * size + column] = correlation;
        values[column * size + row] = correlation;
    }
    Ok(CorrelationMatrix {
        names,
        values,
        given: false,
    })
}

/// Whether float `columns` (every column of the frame) form a correlation matrix:
/// as many rows as columns, every value in -1..1 or missing.
fn is_coefficient_matrix(columns: &[ContinuousColumn], row_count: usize) -> bool {
    !columns.is_empty()
        && columns.len() == row_count
        && columns.iter().all(|column| {
            (0..column.len()).all(|row| {
                column
                    .get(row)
                    .is_none_or(|value| value.is_nan() || (-1.0..=1.0).contains(&value))
            })
        })
}

/// Pearson correlation over the rows where both values are finite; NaN with
/// fewer than 2 such rows or no variance. Clamped to -1..1 against rounding.
fn pearson(first: &ContinuousColumn, second: &ContinuousColumn) -> f64 {
    let pairs: Vec<(f64, f64)> = (0..first.len().min(second.len()))
        .filter_map(|row| Some((first.get(row)?, second.get(row)?)))
        .filter(|(x, y)| x.is_finite() && y.is_finite())
        .collect();
    if pairs.len() < 2 {
        return f64::NAN;
    }
    let count = pairs.len() as f64;
    let mean_x = pairs.iter().map(|(x, _)| x).sum::<f64>() / count;
    let mean_y = pairs.iter().map(|(_, y)| y).sum::<f64>() / count;
    let (mut covariance, mut variance_x, mut variance_y) = (0.0, 0.0, 0.0);
    for (x, y) in &pairs {
        let (delta_x, delta_y) = (x - mean_x, y - mean_y);
        covariance += delta_x * delta_y;
        variance_x += delta_x * delta_x;
        variance_y += delta_y * delta_y;
    }
    let correlation = covariance / (variance_x * variance_y).sqrt();
    if correlation.is_finite() {
        correlation.clamp(-1.0, 1.0)
    } else {
        f64::NAN
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow_array::{ArrayRef, Float64Array, Int64Array, RecordBatch, StringArray};
    use arrow_schema::{Field, Schema};

    use super::*;

    fn frame(columns: Vec<(&str, ArrayRef)>) -> DataFrame {
        let schema = Arc::new(Schema::new(
            columns
                .iter()
                .map(|(name, array)| Field::new(*name, array.data_type().clone(), true))
                .collect::<Vec<_>>(),
        ));
        let arrays = columns.into_iter().map(|(_, array)| array).collect();
        let batch = RecordBatch::try_new(schema.clone(), arrays).expect("batch");
        DataFrame::new(schema, vec![batch]).expect("frame")
    }

    fn floats(values: &[Option<f64>]) -> ArrayRef {
        Arc::new(Float64Array::from(values.to_vec()))
    }

    fn some(values: &[f64]) -> Vec<Option<f64>> {
        values.iter().copied().map(Some).collect()
    }

    #[test]
    fn pairwise_complete_pearson_over_numeric_columns() {
        let data = frame(vec![
            ("a", floats(&some(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]))),
            ("i", Arc::new(Int64Array::from(vec![1, 3, 2, 5, 4, 6]))),
            (
                "s",
                Arc::new(StringArray::from(vec!["u", "v", "w", "x", "y", "z"])),
            ),
            (
                "n",
                floats(&[
                    Some(1.0),
                    None,
                    Some(2.0),
                    Some(5.0),
                    Some(f64::INFINITY),
                    Some(9.0),
                ]),
            ),
        ]);
        let matrix = correlation_matrix(&data).expect("matrix");
        assert_eq!(matrix.names, ["a", "i", "n"]);
        assert!(!matrix.given);
        // numpy.corrcoef([1..6], [1,3,2,5,4,6]) and over rows 0, 2, 3, 5.
        let expected_a_i = 0.885_714_285_714_285_8;
        let expected_a_n = 0.957_923_726_611_343;
        assert!((matrix.values[1] - expected_a_i).abs() < 1e-12);
        assert!((matrix.values[3] - expected_a_i).abs() < 1e-12);
        assert!((matrix.values[2] - expected_a_n).abs() < 1e-12);
        assert_eq!(matrix.values[0], 1.0);
        assert_eq!(matrix.values[8], 1.0);
    }

    #[test]
    fn constant_or_short_columns_are_nan_off_the_diagonal() {
        let data = frame(vec![
            ("a", floats(&some(&[1.0, 2.0, 3.0]))),
            ("c", floats(&some(&[1.0, 1.0, 1.0]))),
            ("short", floats(&[Some(1.0), None, None])),
        ]);
        let matrix = correlation_matrix(&data).expect("matrix");
        assert!(matrix.values[1].is_nan());
        assert!(matrix.values[2].is_nan());
        assert_eq!(matrix.values[4], 1.0);
        assert_eq!(matrix.values[8], 1.0);
    }

    #[test]
    fn a_correlation_matrix_is_read_as_it_is() {
        let data = frame(vec![
            ("a", floats(&[Some(1.0), Some(0.5), None])),
            ("b", floats(&some(&[0.5, 1.0, -0.25]))),
            ("c", floats(&[Some(f64::NAN), Some(-0.25), Some(1.0)])),
        ]);
        let matrix = correlation_matrix(&data).expect("matrix");
        assert!(matrix.given);
        assert_eq!(matrix.values[1], 0.5);
        assert_eq!(matrix.values[5], -0.25);
        assert!(matrix.values[2].is_nan() && matrix.values[6].is_nan());
        // Out of range values: computed instead.
        let data = frame(vec![
            ("a", floats(&some(&[1.0, 2.0]))),
            ("b", floats(&some(&[0.5, 1.0]))),
        ]);
        assert!(!correlation_matrix(&data).expect("matrix").given);
    }

    #[test]
    fn integer_square_frames_are_data() {
        let data = frame(vec![
            ("a", Arc::new(Int64Array::from(vec![1, 0]))),
            ("b", Arc::new(Int64Array::from(vec![0, 1]))),
        ]);
        let matrix = correlation_matrix(&data).expect("matrix");
        assert!(!matrix.given);
        assert_eq!(matrix.values[1], -1.0);
    }
}
