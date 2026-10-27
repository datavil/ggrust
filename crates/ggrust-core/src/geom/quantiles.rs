//! Quantile bands of density geoms (violins, ridges), as lets-plot's
//! `QuantilesHelper`: split a density into its bands and find where quantile
//! lines go. `y` is the axis the density runs along.

use crate::data::ContinuousColumn;

/// Rows sorted by (band, axis value), split where the band changes (lets-plot
/// `QuantilesHelper.splitByQuantiles`); a missing band joins its run.
pub(super) fn split_by_band(
    rows: &[usize],
    band_of: impl Fn(usize) -> f64,
    y: &ContinuousColumn,
) -> Vec<Vec<usize>> {
    let mut sorted = rows.to_vec();
    sorted.sort_by(|left, right| {
        band_of(*left).total_cmp(&band_of(*right)).then(
            y.get(*left)
                .unwrap_or(0.0)
                .total_cmp(&y.get(*right).unwrap_or(0.0)),
        )
    });
    let mut runs: Vec<Vec<usize>> = Vec::new();
    for (index, &row) in sorted.iter().enumerate() {
        let joins = index > 0 && {
            let (previous, current) = (band_of(sorted[index - 1]), band_of(row));
            !(previous.is_finite() && current.is_finite()) || previous == current
        };
        match runs.last_mut() {
            Some(run) if joins => run.push(row),
            _ => runs.push(vec![row]),
        }
    }
    runs
}

/// lets-plot `QuantilesHelper.getQuantileLineElements`: from the end of the axis,
/// the first point of each quantile band (quantiles descending) carries its line.
pub(super) fn quantile_line_rows(
    rows: &[usize],
    band_of: impl Fn(usize) -> f64,
    y: &ContinuousColumn,
    quantiles: &[f64],
) -> Vec<usize> {
    let mut sorted = rows.to_vec();
    sorted.sort_by(|left, right| {
        band_of(*left).total_cmp(&band_of(*right)).then(
            y.get(*left)
                .unwrap_or(0.0)
                .total_cmp(&y.get(*right).unwrap_or(0.0)),
        )
    });
    sorted.reverse();
    let mut descending = quantiles.to_vec();
    descending.sort_by(|left, right| right.total_cmp(left));
    let mut found = Vec::new();
    let mut index = 0;
    for quantile in descending {
        while index < sorted.len() {
            let row = sorted[index];
            index += 1;
            if band_of(row) == quantile {
                found.push(row);
                break;
            }
        }
    }
    found
}
