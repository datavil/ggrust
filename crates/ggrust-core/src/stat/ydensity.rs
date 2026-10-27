//! `stat="ydensity"`: the density of y per x (violins), as lets-plot's
//! `YDensityStat`.
//!
//! Per group and distinct x: a kernel density of y on `n` points (see
//! [`super::density`]), with `..density..`, `..count..`, `..scaled..` and
//! `..quantile..` (the quantile band of each point; band edges are repeated so
//! bands touch). Over the whole layer, `..violinwidth..` by `scale`: `area`
//! (density / largest density, default), `count` (wider for more data) or `width`
//! (every violin as wide).
//!
//! Options: `scale`, `trim` (default true), `tails_cutoff` (3 bandwidths when not
//! trimmed), `bw`, `adjust`, `kernel`, `n`, `fs_max`, `quantiles`, `weight`.
//!
//! What it does not do: `orientation="y"`.

use super::density::{DensityOptions, collect_bins, estimate_bins};
use super::groups::per_group;
use super::{Stat, StatContext};
use crate::data::{Column, Computed, ContinuousColumn, LayerData, Variable};
use crate::parameters::Parameters;
use crate::spec::{Aesthetic, Value};
use crate::{Error, Result};

/// lets-plot `BaseYDensityStat.DEF_QUANTILES`.
pub const DEFAULT_QUANTILES: [f64; 3] = [0.25, 0.5, 0.75];

/// Densities of y per x.
pub struct YDensityStat;

impl Stat for YDensityStat {
    fn name(&self) -> &'static str {
        "ydensity"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &[
            "scale",
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
        &[Aesthetic::Y]
    }

    fn default_mapping(&self) -> &'static [(Aesthetic, Computed)] {
        &[
            (Aesthetic::Violinwidth, Computed::ViolinWidth),
            (Aesthetic::Quantile, Computed::Quantile),
        ]
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        context: &StatContext,
    ) -> Result<LayerData> {
        let scale = ViolinScale::from_parameters(parameters)?;
        let options = DensityOptions::from_parameters(parameters, true, Some(3.0))?;
        let quantiles = quantiles(parameters)?;
        // lets-plot `overallYRange`, or -0.5..0.5 without data.
        let overall = context.y_range.unwrap_or((-0.5, 0.5));
        let empty = ContinuousColumn::from_values(Vec::new());
        let y = data.continuous(Aesthetic::Y).unwrap_or(&empty);
        let x = data.continuous(Aesthetic::X);
        let weight = data.continuous(Aesthetic::Weight);
        let mut result = per_group(&data, |rows| {
            let bins = collect_bins(rows, x, y, weight);
            let [x, y, density, count, scaled, quantile] =
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
        let widths = scale.widths(&result);
        result.insert(
            Computed::ViolinWidth,
            Column::Continuous(ContinuousColumn::from_values(widths)),
        )?;
        Ok(result)
    }
}

/// `scale=`: how violin widths compare across the layer.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ViolinScale {
    Area,
    Count,
    Width,
}

impl ViolinScale {
    fn from_parameters(parameters: &Parameters) -> Result<Self> {
        match parameters
            .string("scale")?
            .map(str::to_lowercase)
            .as_deref()
        {
            None | Some("area") => Ok(ViolinScale::Area),
            Some("count") => Ok(ViolinScale::Count),
            Some("width") => Ok(ViolinScale::Width),
            Some(other) => Err(Error::InvalidSpec(format!(
                "at '{}': unsupported scale '{other}'; use one of: area, count, width",
                parameters.path_of("scale")
            ))),
        }
    }

    /// lets-plot `YDensityStat.normalize`.
    fn widths(self, data: &LayerData) -> Vec<f64> {
        let values = |variable: Computed| -> Vec<f64> {
            data.continuous(variable).map_or_else(Vec::new, |column| {
                (0..column.len())
                    .map(|row| column.get(row).unwrap_or(f64::NAN))
                    .collect()
            })
        };
        let density = values(Computed::Density);
        let largest = density.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        match self {
            ViolinScale::Area => density.iter().map(|value| value / largest).collect(),
            ViolinScale::Width => values(Computed::Scaled),
            ViolinScale::Count => {
                let count = values(Computed::Count);
                // The largest total weight of a violin (count / density per point).
                let weight = count
                    .iter()
                    .zip(&density)
                    .filter(|(_, density)| **density > 0.0)
                    .map(|(count, density)| count / density)
                    .fold(f64::NEG_INFINITY, f64::max);
                count
                    .iter()
                    .map(|value| value / (largest * weight))
                    .collect()
            }
        }
    }
}

/// `quantiles=[...]`, each within 0..1.
pub fn quantiles(parameters: &Parameters) -> Result<Vec<f64>> {
    let Some(value) = parameters.value("quantiles") else {
        return Ok(DEFAULT_QUANTILES.to_vec());
    };
    let invalid = || {
        Error::InvalidSpec(format!(
            "at '{}': expected a list of numbers between 0 and 1, got {value}",
            parameters.path_of("quantiles")
        ))
    };
    let Value::Array(items) = value else {
        return Err(invalid());
    };
    items
        .iter()
        .map(|item| {
            item.as_f64()
                .filter(|number| (0.0..=1.0).contains(number))
                .ok_or_else(invalid)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Options;

    fn compute(x: &[f64], y: &[f64], options: &str) -> LayerData {
        let mut data = LayerData::new(y.len());
        let column =
            |values: &[f64]| Column::Continuous(ContinuousColumn::from_values(values.to_vec()));
        data.insert(Aesthetic::X, column(x)).unwrap();
        data.insert(Aesthetic::Y, column(y)).unwrap();
        let options: Options = serde_json::from_str(options).unwrap();
        YDensityStat
            .compute(
                data,
                &Parameters::new(&options, "layers[0]"),
                &StatContext::default(),
            )
            .unwrap()
    }

    fn values(data: &LayerData, variable: impl Into<Variable>) -> Vec<f64> {
        let column = data.continuous(variable).unwrap();
        (0..column.len())
            .map(|row| column.get(row).unwrap())
            .collect()
    }

    #[test]
    fn trimmed_grid_per_x_with_shared_band_edges() {
        let x = [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let y = [1.0, 2.0, 3.0, 5.0, 2.0, 2.5, 4.0];
        let data = compute(&x, &y, r#"{"n": 16}"#);
        let x = values(&data, Aesthetic::X);
        let y = values(&data, Aesthetic::Y);
        // 16 grid points per violin plus one repeated point per band change (3).
        assert_eq!(x.iter().filter(|value| **value == 0.0).count(), 19);
        assert_eq!(y[0], 1.0);
        let widths = values(&data, Computed::ViolinWidth);
        assert_eq!(widths.iter().copied().fold(0.0, f64::max), 1.0);
        let bands = values(&data, Computed::Quantile);
        assert!(
            bands
                .windows(2)
                .all(|pair| pair[0] <= pair[1] || pair[1] == 0.25)
        );
    }

    #[test]
    fn width_scale_makes_every_violin_reach_one() {
        let x = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let y = [1.0, 2.0, 3.0, 2.0, 2.5, 4.0, 4.5, 9.0];
        let data = compute(&x, &y, r#"{"scale": "width", "n": 32}"#);
        let x = values(&data, Aesthetic::X);
        let widths = values(&data, Computed::ViolinWidth);
        for bin in [0.0, 1.0] {
            let largest = x
                .iter()
                .zip(&widths)
                .filter(|(location, _)| **location == bin)
                .map(|(_, width)| *width)
                .fold(0.0, f64::max);
            assert!((largest - 1.0).abs() < 1e-12);
        }
    }
}
