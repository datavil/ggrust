//! `stat="density"`: the kernel density of x per group, as lets-plot's
//! `DensityStat`.
//!
//! Per group: `n` grid points with `..density..` (y by default), `..count..`,
//! `..scaled..` and `..quantile..` (the band of each point; band ends repeated
//! so bands touch). Not trimmed by default: the grid spans every layer's x range.
//! Bandwidths follow R, see [`super::density`].
//!
//! What it does not do: `orientation="y"`.

use super::density::{DensityOptions, collect_bins, estimate_bins};
use super::groups::per_group;
use super::ydensity::quantiles;
use super::{Stat, StatContext};
use crate::Result;
use crate::data::{Computed, ContinuousColumn, LayerData, Variable};
use crate::parameters::Parameters;
use crate::spec::Aesthetic;

/// Densities of x.
pub struct DensityStat;

impl Stat for DensityStat {
    fn name(&self) -> &'static str {
        "density"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["trim", "bw", "adjust", "kernel", "n", "fs_max", "quantiles"]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Weight]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X]
    }

    fn default_mapping(&self) -> &'static [(Aesthetic, Computed)] {
        &[
            (Aesthetic::Y, Computed::Density),
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
        let weight = data.continuous(Aesthetic::Weight);
        per_group(&data, |rows| {
            let bins = collect_bins(rows, None, x, weight);
            let [_, x, density, count, scaled, quantile] =
                estimate_bins(bins, &options, &quantiles, overall);
            Ok(vec![
                (Variable::Aesthetic(Aesthetic::X), x),
                (Variable::Computed(Computed::Density), density),
                (Variable::Computed(Computed::Count), count),
                (Variable::Computed(Computed::Scaled), scaled),
                (Variable::Computed(Computed::Quantile), quantile),
            ])
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Column;
    use crate::spec::Options;

    #[test]
    fn density_integrates_to_one_over_the_overall_range() {
        let mut data = LayerData::new(5);
        let values = vec![1.0, 2.0, 2.5, 3.0, 5.0];
        data.insert(
            Aesthetic::X,
            Column::Continuous(ContinuousColumn::from_values(values)),
        )
        .unwrap();
        let options: Options = serde_json::from_str(r#"{"n": 1000}"#).unwrap();
        let context = StatContext {
            x_range: Some((-10.0, 16.0)),
            y_range: None,
            ..StatContext::default()
        };
        let result = DensityStat
            .compute(data, &Parameters::new(&options, ""), &context)
            .unwrap();
        let x = result.continuous(Aesthetic::X).unwrap();
        let density = result.continuous(Computed::Density).unwrap();
        assert_eq!((x.get(0), x.get(x.len() - 1)), (Some(-10.0), Some(16.0)));
        // Trapezoids over the grid (band ends are repeated: zero-width steps).
        let area: f64 = (1..x.len())
            .map(|row| {
                let width = x.get(row).unwrap() - x.get(row - 1).unwrap();
                width * (density.get(row).unwrap() + density.get(row - 1).unwrap()) / 2.0
            })
            .sum();
        assert!((area - 1.0).abs() < 1e-3, "area {area}");
        let scaled = result.continuous(Computed::Scaled).unwrap();
        let largest = (0..scaled.len())
            .filter_map(|row| scaled.get(row))
            .fold(0.0, f64::max);
        assert_eq!(largest, 1.0);
    }
}
