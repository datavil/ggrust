//! `stat="density"`: the kernel density of x per group, as lets-plot's
//! `DensityStat`.
//!
//! Per group: `n` grid points with `..density..` (y by default), `..count..`,
//! `..scaled..` and `..quantile..` (the band of each point; band ends repeated
//! so bands touch). Not trimmed by default: the grid spans every layer's x range.
//! Bandwidths follow R, see [`super::density`].
//!
//! `bounds=[low, high]` drops values outside and reflects the kernels at the
//! bounds (ggplot2). `cut`, `clip` and `gridsize` are accepted for plotnine and
//! change nothing, as in plotnine 0.15.
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
        &[
            "trim",
            "bw",
            "adjust",
            "kernel",
            "n",
            "fs_max",
            "quantiles",
            "bounds",
            // plotnine passes these to statsmodels, where they leave the plot unchanged.
            "cut",
            "clip",
            "gridsize",
        ]
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
            let mut bins = collect_bins(rows, None, x, weight);
            for (_, pairs) in &mut bins {
                pairs.retain(|(value, _)| options.within_bounds(*value));
            }
            bins.retain(|(_, pairs)| !pairs.is_empty());
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

    fn density_with(values: Vec<f64>, options: &str) -> (Vec<f64>, Vec<f64>) {
        let mut data = LayerData::new(values.len());
        data.insert(
            Aesthetic::X,
            Column::Continuous(ContinuousColumn::from_values(values)),
        )
        .unwrap();
        let options: Options = serde_json::from_str(options).unwrap();
        let context = StatContext {
            x_range: Some((-5.0, 10.0)),
            ..StatContext::default()
        };
        let result = DensityStat
            .compute(data, &Parameters::new(&options, ""), &context)
            .unwrap();
        let column = |variable: Variable| {
            let column = result.continuous(variable).unwrap();
            (0..column.len())
                .map(|row| column.get(row).unwrap())
                .collect()
        };
        (
            column(Variable::Aesthetic(Aesthetic::X)),
            column(Variable::Computed(Computed::Density)),
        )
    }

    #[test]
    fn bounds_reflect_the_kernels_and_drop_values_outside() {
        let values = vec![0.1, 0.4, 0.5, 1.2, 2.0, 3.5, -1.0];
        let (x, density) = density_with(values, r#"{"n": 1000, "bounds": [0, null], "bw": 0.5}"#);
        // The range widens by its width below the bound (ggplot2), then stops at it.
        assert_eq!(x[0], 0.0);
        assert_eq!(x[x.len() - 1], 10.0);
        let area: f64 = (1..x.len())
            .map(|row| (x[row] - x[row - 1]) * (density[row] + density[row - 1]) / 2.0)
            .sum();
        assert!((area - 1.0).abs() < 1e-3, "area {area}");
        // R: x <- c(.1, .4, .5, 1.2, 2, 3.5); d <- density(c(x, -x), bw = .5, n = 2^14,
        // from = 0, to = 1); 2 * approx(d$x, d$y, 0)$y = 0.630156 (6 values, not 7);
        // ggplot2's geom_density(bounds = c(0, Inf)) gives 0.6301268.
        assert!((density[0] - 0.630156).abs() < 1e-5, "{}", density[0]);
    }

    #[test]
    fn plotnine_statsmodels_options_change_nothing() {
        let values = vec![0.0, 1.0, 2.0, 3.0, 10.0];
        let plain = density_with(values.clone(), r#"{"n": 50}"#);
        let with = density_with(
            values,
            r#"{"n": 50, "cut": 0.5, "clip": [-1, 5], "gridsize": 20}"#,
        );
        assert_eq!(plain, with);
    }

    #[test]
    fn bounds_must_be_ordered_pairs() {
        for bad in [r#"[1, 0]"#, r#"[0]"#, r#""x""#, r#"[0, "a"]"#] {
            let options: Options =
                serde_json::from_str(&format!(r#"{{"bounds": {bad}}}"#)).unwrap();
            let error =
                DensityOptions::from_parameters(&Parameters::new(&options, "l"), false, None)
                    .unwrap_err()
                    .to_string();
            assert!(error.contains("expected [low, high]"), "{error}");
        }
    }

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
