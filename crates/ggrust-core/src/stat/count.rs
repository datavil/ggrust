//! `stat="count"`: rows (or summed `weight`) per x location, as lets-plot's
//! `CountStat`.
//!
//! Per group: one row per distinct finite x, in order of first appearance, with
//! `..count..`. Over the whole layer: `..sum..` (count at the location over all
//! groups), `..prop..` / `..proppct..` (the group's share at the location) and
//! `..sumprop..` / `..sumpct..` (the location's share of the layer).
//!
//! What it does not do: `count2d`, or report rows with a missing x (they are
//! dropped, as lets-plot drops them).

use std::collections::HashMap;

use super::groups::per_group;
use super::{Stat, StatContext};
use crate::Result;
use crate::data::{Column, Computed, ContinuousColumn, LayerData, Variable};
use crate::parameters::Parameters;
use crate::spec::Aesthetic;

/// Counts rows per x.
pub struct CountStat;

impl Stat for CountStat {
    fn name(&self) -> &'static str {
        "count"
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Weight]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X]
    }

    fn default_mapping(&self) -> &'static [(Aesthetic, Computed)] {
        &[(Aesthetic::Y, Computed::Count)]
    }

    fn compute(
        &self,
        data: LayerData,
        _parameters: &Parameters,
        _context: &StatContext,
    ) -> Result<LayerData> {
        let empty = ContinuousColumn::from_values(Vec::new());
        let x = data.continuous(Aesthetic::X).unwrap_or(&empty);
        let weight = data.continuous(Aesthetic::Weight);
        let mut result = per_group(&data, |rows| {
            let mut locations: Vec<f64> = Vec::new();
            let mut counts: Vec<f64> = Vec::new();
            let mut slot_by_location: HashMap<u64, usize> = HashMap::new();
            for &row in rows {
                let Some(location) = x.get(row).filter(|value| value.is_finite()) else {
                    continue;
                };
                let slot = *slot_by_location
                    .entry(location_key(location))
                    .or_insert_with(|| {
                        locations.push(location);
                        counts.push(0.0);
                        locations.len() - 1
                    });
                counts[slot] += row_weight(weight, row);
            }
            Ok(vec![
                (Variable::Aesthetic(Aesthetic::X), locations),
                (Variable::Computed(Computed::Count), counts),
            ])
        })?;
        normalize(&mut result)?;
        Ok(result)
    }
}

/// A missing or non-finite weight counts as 0 (lets-plot `finiteOrNull ?: 0`).
pub(super) fn row_weight(weight: Option<&ContinuousColumn>, row: usize) -> f64 {
    match weight {
        Some(weight) => weight
            .get(row)
            .filter(|value| value.is_finite())
            .unwrap_or(0.0),
        None => 1.0,
    }
}

/// Locations compare as Kotlin's boxed doubles (`Double.equals`): by bits, so
/// -0.0 and 0.0 are two locations.
fn location_key(location: f64) -> u64 {
    location.to_bits()
}

/// lets-plot `AbstractCountStat.normalize`: layer-wide sums and proportions.
fn normalize(data: &mut LayerData) -> Result<()> {
    let (Some(x), Some(count)) = (
        data.continuous(Aesthetic::X),
        data.continuous(Computed::Count),
    ) else {
        return Ok(());
    };
    let mut total_by_location: HashMap<u64, f64> = HashMap::new();
    for row in 0..data.row_count() {
        let (Some(location), Some(count)) = (x.get(row), count.get(row)) else {
            continue;
        };
        *total_by_location
            .entry(location_key(location))
            .or_insert(0.0) += count;
    }
    let total: f64 = total_by_location.values().sum();
    let mut sums = Vec::with_capacity(data.row_count());
    let mut proportions = Vec::with_capacity(data.row_count());
    for row in 0..data.row_count() {
        let location_total = x
            .get(row)
            .and_then(|location| total_by_location.get(&location_key(location)))
            .copied()
            .unwrap_or(f64::NAN);
        sums.push(location_total);
        proportions.push(count.get(row).unwrap_or(f64::NAN) / location_total);
    }
    let percent = |values: &[f64]| values.iter().map(|value| value * 100.0).collect();
    let sum_proportions: Vec<f64> = sums.iter().map(|sum| sum / total).collect();
    let columns = [
        (Computed::ProportionPercent, percent(&proportions)),
        (Computed::Proportion, proportions),
        (Computed::SumPercent, percent(&sum_proportions)),
        (Computed::SumProportion, sum_proportions),
        (Computed::Sum, sums),
    ];
    for (variable, values) in columns {
        data.insert(
            variable,
            Column::Continuous(ContinuousColumn::from_values(values)),
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Options;

    fn column(values: &[f64]) -> Column {
        Column::Continuous(ContinuousColumn::from_values(values.to_vec()))
    }

    fn values(data: &LayerData, variable: impl Into<Variable>) -> Vec<f64> {
        let column = data.continuous(variable).unwrap();
        (0..column.len())
            .map(|row| column.get(row).unwrap())
            .collect()
    }

    #[test]
    fn counts_per_group_and_location_in_first_appearance() {
        // (x, group): (0,0) (1,0) (0,1) (2,1) (1,0) (0,1)
        let mut data = LayerData::new(6);
        data.insert(Aesthetic::X, column(&[0.0, 1.0, 0.0, 2.0, 1.0, 0.0]))
            .unwrap();
        data.set_group_index(Some(vec![0, 0, 1, 1, 0, 1])).unwrap();
        let options = Options::new();
        let result = CountStat
            .compute(
                data,
                &Parameters::new(&options, "layers[0]"),
                &StatContext::default(),
            )
            .unwrap();
        assert_eq!(values(&result, Aesthetic::X), [0.0, 1.0, 0.0, 2.0]);
        assert_eq!(values(&result, Computed::Count), [1.0, 2.0, 2.0, 1.0]);
        assert_eq!(values(&result, Computed::Sum), [3.0, 2.0, 3.0, 1.0]);
        let proportion = values(&result, Computed::Proportion);
        assert!((proportion[0] - 1.0 / 3.0).abs() < 1e-12);
        assert_eq!(values(&result, Computed::SumProportion)[3], 1.0 / 6.0);
        assert_eq!(result.group_index(), Some(&[0, 0, 1, 1][..]));
    }

    #[test]
    fn weights_sum_and_missing_x_is_dropped() {
        let mut data = LayerData::new(3);
        data.insert(
            Aesthetic::X,
            Column::Continuous(ContinuousColumn::from_options(vec![
                Some(1.0),
                None,
                Some(1.0),
            ])),
        )
        .unwrap();
        data.insert(
            Aesthetic::Weight,
            Column::Continuous(ContinuousColumn::from_options(vec![
                Some(2.5),
                Some(1.0),
                None,
            ])),
        )
        .unwrap();
        let options = Options::new();
        let result = CountStat
            .compute(
                data,
                &Parameters::new(&options, ""),
                &StatContext::default(),
            )
            .unwrap();
        assert_eq!(values(&result, Computed::Count), [2.5]);
    }
}
