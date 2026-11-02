//! `stat="sum"`: rows per distinct (x, y) location, as lets-plot's `SumStat`
//! (`geom_count`).
//!
//! Per group: one row per distinct finite (x, y), in order of first appearance,
//! with `..n..`, `..prop..` (`n` over the group's total) and `..proppct..`. The
//! size is mapped to `..n..` by default. A mapped `weight` is summed into `..n..`,
//! as ggplot2 (lets-plot ignores it; deviation).
//!
//! What it does not do: report rows with a missing x or y (they are dropped).

use std::collections::HashMap;

use super::count::row_weight;
use super::groups::per_group;
use super::{Stat, StatContext};
use crate::Result;
use crate::data::{Computed, ContinuousColumn, LayerData, Variable};
use crate::parameters::Parameters;
use crate::spec::Aesthetic;

/// Counts rows per (x, y).
pub struct SumStat;

impl Stat for SumStat {
    fn name(&self) -> &'static str {
        "sum"
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y, Aesthetic::Weight]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn default_mapping(&self) -> &'static [(Aesthetic, Computed)] {
        &[(Aesthetic::Size, Computed::N)]
    }

    fn compute(
        &self,
        data: LayerData,
        _parameters: &Parameters,
        _context: &StatContext,
    ) -> Result<LayerData> {
        let empty = ContinuousColumn::from_values(Vec::new());
        let x = data.continuous(Aesthetic::X).unwrap_or(&empty);
        let y = data.continuous(Aesthetic::Y).unwrap_or(&empty);
        let weight = data.continuous(Aesthetic::Weight);
        per_group(&data, |rows| {
            let mut xs: Vec<f64> = Vec::new();
            let mut ys: Vec<f64> = Vec::new();
            let mut counts: Vec<f64> = Vec::new();
            // Locations compare by bits, as Kotlin's boxed doubles (see count.rs).
            let mut slot_by_location: HashMap<(u64, u64), usize> = HashMap::new();
            for &row in rows {
                let (Some(x_value), Some(y_value)) = (
                    x.get(row).filter(|value| value.is_finite()),
                    y.get(row).filter(|value| value.is_finite()),
                ) else {
                    continue;
                };
                let slot = *slot_by_location
                    .entry((x_value.to_bits(), y_value.to_bits()))
                    .or_insert_with(|| {
                        xs.push(x_value);
                        ys.push(y_value);
                        counts.push(0.0);
                        counts.len() - 1
                    });
                counts[slot] += row_weight(weight, row);
            }
            let total: f64 = counts.iter().sum();
            let proportions: Vec<f64> = counts.iter().map(|count| count / total).collect();
            let percents = proportions.iter().map(|value| value * 100.0).collect();
            Ok(vec![
                (Variable::Aesthetic(Aesthetic::X), xs),
                (Variable::Aesthetic(Aesthetic::Y), ys),
                (Variable::Computed(Computed::N), counts),
                (Variable::Computed(Computed::Proportion), proportions),
                (Variable::Computed(Computed::ProportionPercent), percents),
            ])
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Column;
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

    fn compute(data: LayerData) -> LayerData {
        let options = Options::new();
        SumStat
            .compute(
                data,
                &Parameters::new(&options, "layers[0]"),
                &StatContext::default(),
            )
            .unwrap()
    }

    #[test]
    fn counts_locations_per_group_as_lets_plot() {
        // The measured lets-plot case: group "g" with a = 0, b = 1.
        let mut data = LayerData::new(8);
        data.insert(
            Aesthetic::X,
            column(&[0.0, 0.0, 0.0, 1.0, 1.0, 2.0, 0.0, 1.0]),
        )
        .unwrap();
        data.insert(
            Aesthetic::Y,
            column(&[0.0, 0.0, 1.0, 1.0, 1.0, 2.0, 0.0, 1.0]),
        )
        .unwrap();
        data.set_group_index(Some(vec![0, 0, 0, 1, 1, 1, 1, 0]))
            .unwrap();
        let result = compute(data);
        assert_eq!(
            values(&result, Aesthetic::X),
            [0.0, 0.0, 1.0, 1.0, 2.0, 0.0]
        );
        assert_eq!(
            values(&result, Aesthetic::Y),
            [0.0, 1.0, 1.0, 1.0, 2.0, 0.0]
        );
        assert_eq!(values(&result, Computed::N), [2.0, 1.0, 1.0, 2.0, 1.0, 1.0]);
        assert_eq!(
            values(&result, Computed::Proportion),
            [0.5, 0.25, 0.25, 0.5, 0.25, 0.25]
        );
        assert_eq!(values(&result, Computed::ProportionPercent)[0], 50.0);
        assert_eq!(result.group_index(), Some(&[0, 0, 0, 1, 1, 1][..]));
    }

    #[test]
    fn weights_sum_and_missing_positions_are_dropped() {
        let mut data = LayerData::new(4);
        data.insert(
            Aesthetic::X,
            Column::Continuous(ContinuousColumn::from_options(vec![
                Some(1.0),
                Some(1.0),
                None,
                Some(1.0),
            ])),
        )
        .unwrap();
        data.insert(
            Aesthetic::Y,
            Column::Continuous(ContinuousColumn::from_options(vec![
                Some(2.0),
                Some(2.0),
                Some(2.0),
                Some(f64::NAN),
            ])),
        )
        .unwrap();
        data.insert(Aesthetic::Weight, column(&[2.0, 3.0, 1.0, 1.0]))
            .unwrap();
        let result = compute(data);
        assert_eq!(values(&result, Computed::N), [5.0]);
        assert_eq!(values(&result, Computed::Proportion), [1.0]);
    }
}
