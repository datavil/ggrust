//! `stat="densityridges"`: the density of x per y (ridgelines), as lets-plot's
//! `DensityRidgesStat`: [`super::ydensity`] with x and y swapped.
//!
//! Per group and distinct y: a kernel density of x, with `..density..`,
//! `..count..`, `..scaled..`, `..quantile..`, and over the whole layer
//! `..height..` (density / largest density). Not trimmed by default; without
//! `tails_cutoff` the densities span every layer's x range.
//!
//! What it does not do: `orientation`.

use super::density::{DensityOptions, collect_bins, estimate_bins};
use super::groups::per_group;
use super::ydensity::quantiles;
use super::{Stat, StatContext};
use crate::Result;
use crate::data::{Column, Computed, ContinuousColumn, LayerData, Variable};
use crate::parameters::Parameters;
use crate::spec::Aesthetic;

/// Densities of x per y.
pub struct DensityRidgesStat;

impl Stat for DensityRidgesStat {
    fn name(&self) -> &'static str {
        "densityridges"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &[
            "trim",
            "tails_cutoff",
            "bw",
            "adjust",
            "kernel",
            "n",
            "fs_max",
            "quantiles",
        ]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y, Aesthetic::Weight]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X]
    }

    fn default_mapping(&self) -> &'static [(Aesthetic, Computed)] {
        &[
            (Aesthetic::Height, Computed::Height),
            (Aesthetic::Quantile, Computed::Quantile),
        ]
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        context: &StatContext,
    ) -> Result<LayerData> {
        let options = DensityOptions::from_parameters(parameters, false, None)?;
        let quantiles = quantiles(parameters)?;
        let overall = context.x_range.unwrap_or((-0.5, 0.5));
        let empty = ContinuousColumn::from_values(Vec::new());
        let x = data.continuous(Aesthetic::X).unwrap_or(&empty);
        let y = data.continuous(Aesthetic::Y);
        let weight = data.continuous(Aesthetic::Weight);
        let mut result = per_group(&data, |rows| {
            let bins = collect_bins(rows, y, x, weight);
            let [y, x, density, count, scaled, quantile] =
                estimate_bins(bins, &options, &quantiles, overall);
            Ok(vec![
                (Variable::Aesthetic(Aesthetic::X), x),
                (Variable::Aesthetic(Aesthetic::Y), y),
                (Variable::Computed(Computed::Density), density),
                (Variable::Computed(Computed::Count), count),
                (Variable::Computed(Computed::Scaled), scaled),
                (Variable::Computed(Computed::Quantile), quantile),
            ])
        })?;
        let density: Vec<f64> =
            result
                .continuous(Computed::Density)
                .map_or_else(Vec::new, |column| {
                    (0..column.len())
                        .map(|row| column.get(row).unwrap_or(f64::NAN))
                        .collect()
                });
        let largest = density.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let heights = density.iter().map(|value| value / largest).collect();
        result.insert(
            Computed::Height,
            Column::Continuous(ContinuousColumn::from_values(heights)),
        )?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Options;

    #[test]
    fn densities_of_x_per_y_spanning_the_overall_range() {
        let mut data = LayerData::new(6);
        let column = |values: Vec<f64>| Column::Continuous(ContinuousColumn::from_values(values));
        data.insert(Aesthetic::X, column(vec![1.0, 2.0, 3.0, 5.0, 6.0, 9.0]))
            .unwrap();
        data.insert(Aesthetic::Y, column(vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0]))
            .unwrap();
        let options: Options = serde_json::from_str(r#"{"n": 8, "quantiles": []}"#).unwrap();
        let context = StatContext {
            x_range: Some((0.0, 10.0)),
            y_range: None,
        };
        let result = DensityRidgesStat
            .compute(data, &Parameters::new(&options, ""), &context)
            .unwrap();
        let x = result.continuous(Aesthetic::X).unwrap();
        let y = result.continuous(Aesthetic::Y).unwrap();
        assert_eq!(result.row_count(), 16);
        assert_eq!((x.get(0), x.get(7)), (Some(0.0), Some(10.0)));
        assert_eq!((y.get(0), y.get(8)), (Some(0.0), Some(1.0)));
        let height = result.continuous(Computed::Height).unwrap();
        let largest = (0..16)
            .filter_map(|row| height.get(row))
            .fold(0.0, f64::max);
        assert_eq!(largest, 1.0);
    }
}
