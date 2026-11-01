//! `stat="ecdf"`: the empirical cumulative distribution of x, as lets-plot's
//! `ECDFStat`.
//!
//! Per group: y = the share of finite x values at or below each x. Without `n`
//! the distinct x values (in order of first appearance), with `n` that many
//! points evenly spaced from the smallest to the largest x. `pad` (default true)
//! adds (-inf, 0) and (inf, 1), which a `pad`ded `geom_step` draws to the panel
//! edges.
//!
//! What it does not do: `orientation="y"`.

use super::groups::per_group;
use super::{Stat, StatContext};
use crate::Result;
use crate::data::{ContinuousColumn, LayerData, Variable};
use crate::parameters::Parameters;
use crate::spec::Aesthetic;

/// Empirical cumulative distribution of x.
pub struct EcdfStat;

/// lets-plot `ECDFStat.DEF_PADDED`.
const DEFAULT_PAD: bool = true;

impl Stat for EcdfStat {
    fn name(&self) -> &'static str {
        "ecdf"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["n", "pad"]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X]
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        _context: &StatContext,
    ) -> Result<LayerData> {
        // lets-plot `getInteger`: truncated; below 1 gives no points.
        let count = parameters.number("n")?.map(|value| {
            if value >= 1.0 {
                value.trunc() as usize
            } else {
                0
            }
        });
        let pad = parameters.boolean("pad")?.unwrap_or(DEFAULT_PAD);
        let empty = ContinuousColumn::from_values(Vec::new());
        let x = data.continuous(Aesthetic::X).unwrap_or(&empty);
        per_group(&data, |rows| {
            let values: Vec<f64> = rows
                .iter()
                .filter_map(|&row| x.get(row).filter(|value| value.is_finite()))
                .collect();
            let (locations, shares) = ecdf(&values, count, pad);
            Ok(vec![
                (Variable::Aesthetic(Aesthetic::X), locations),
                (Variable::Aesthetic(Aesthetic::Y), shares),
            ])
        })
    }
}

/// (x, y) of one group's ECDF.
fn ecdf(values: &[f64], count: Option<usize>, pad: bool) -> (Vec<f64>, Vec<f64>) {
    if values.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let total = sorted.len() as f64;
    let share = |location: f64| sorted.partition_point(|value| *value <= location) as f64 / total;
    let mut locations: Vec<f64> = match count {
        None => {
            // Kotlin `distinct()`: first appearance, -0.0 and 0.0 apart.
            let mut seen = std::collections::HashSet::new();
            values
                .iter()
                .copied()
                .filter(|value| seen.insert(value.to_bits()))
                .collect()
        }
        Some(count) => linspace(sorted[0], sorted[sorted.len() - 1], count),
    };
    let mut shares: Vec<f64> = locations.iter().map(|&location| share(location)).collect();
    if pad {
        locations.extend([f64::NEG_INFINITY, f64::INFINITY]);
        shares.extend([0.0, 1.0]);
    }
    (locations, shares)
}

/// lets-plot `ECDFStat.linspace`.
fn linspace(start: f64, stop: f64, count: usize) -> Vec<f64> {
    match count {
        0 => Vec::new(),
        1 => vec![start],
        _ => {
            let step = (stop - start) / (count - 1) as f64;
            (0..count)
                .map(|index| start + index as f64 * step)
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shares_count_ties() {
        let (x, y) = ecdf(&[2.0, 1.0, 2.0, 4.0], None, false);
        assert_eq!(x, vec![2.0, 1.0, 4.0]);
        assert_eq!(y, vec![0.75, 0.25, 1.0]);
    }

    #[test]
    fn interpolated_and_padded() {
        let (x, y) = ecdf(&[0.0, 1.0, 2.0, 3.0, 4.0], Some(3), true);
        assert_eq!(x, vec![0.0, 2.0, 4.0, f64::NEG_INFINITY, f64::INFINITY]);
        assert_eq!(y, vec![0.2, 0.6, 1.0, 0.0, 1.0]);
    }

    #[test]
    fn empty_group_has_no_rows() {
        assert_eq!(ecdf(&[], None, true), (Vec::new(), Vec::new()));
    }
}
