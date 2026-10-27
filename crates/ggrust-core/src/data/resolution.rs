//! Data resolution: the smallest gap between distinct values of a position, as
//! lets-plot's `GeomContext.getResolution` (bar widths, dodging).

use super::ContinuousColumn;

/// lets-plot `SeriesUtil.TINY`.
const TINY: f64 = 1e-50;

/// Smallest positive gap between the finite values; 1 when there are fewer than
/// two distinct values.
pub fn resolution(column: &ContinuousColumn) -> f64 {
    let mut values: Vec<f64> = (0..column.len())
        .filter_map(|row| column.get(row).filter(|value| value.is_finite()))
        .collect();
    values.sort_by(f64::total_cmp);
    let smallest = values
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .filter(|gap| *gap > 0.0)
        .fold(f64::INFINITY, f64::min);
    if smallest.is_finite() && smallest > TINY {
        smallest
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smallest_gap_or_one() {
        let column = |values: Vec<Option<f64>>| ContinuousColumn::from_options(values);
        assert_eq!(
            resolution(&column(vec![
                Some(3.0),
                Some(1.0),
                None,
                Some(1.5),
                Some(3.0)
            ])),
            0.5
        );
        assert_eq!(resolution(&column(vec![Some(2.0), Some(2.0)])), 1.0);
        assert_eq!(resolution(&column(vec![])), 1.0);
    }
}
