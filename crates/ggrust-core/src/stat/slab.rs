//! `stat="slab"`: the density of y per group and x, as the R package ggdist's
//! `stat_slab` computes it for sample data (the slab of `stat_halfeye`).
//!
//! Per group and distinct x (first appearance), with two or more distinct
//! finite y values: a density on `n` points (default 501) from
//! [`super::ggdist_density`], giving `..pdf..`, `..cdf..` (empirical),
//! `..thickness..` (the pdf, drawn by default) and `..n..` (the group's size).
//! Groups with fewer values draw nothing.
//!
//! Options: `density` (`"bounded"`, default, or `"unbounded"`), `bandwidth`
//! (`"dpi"`, default, `"nrd0"`, `"nrd"`, or a positive number), `adjust`
//! (default 1), `trim` (default true), `n`.
//!
//! What it does not do: weights, distribution (`dist`) aesthetics, `p_limits`,
//! histogram densities, ggdist's warning when `dpi` falls back to `nrd0`.

use std::collections::HashMap;

use super::ggdist_density::{BandwidthRule, DensityKind, sample_density};
use super::groups::per_group;
use super::{Stat, StatContext};
use crate::data::{Computed, ContinuousColumn, LayerData, Variable};
use crate::parameters::Parameters;
use crate::spec::{Aesthetic, Value};
use crate::{Error, Result};

/// ggdist's slab grid size.
const DEFAULT_N: usize = 501;

/// Slab densities per group.
pub struct SlabStat;

/// The layer's density options.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SlabOptions {
    kind: DensityKind,
    bandwidth: BandwidthRule,
    adjust: f64,
    trim: bool,
    n: usize,
}

impl SlabOptions {
    fn from_parameters(parameters: &Parameters) -> Result<Self> {
        let kind = match parameters.string("density")? {
            None | Some("bounded") => DensityKind::Bounded,
            Some("unbounded") => DensityKind::Unbounded,
            Some(other) => {
                return Err(Error::spec(
                    parameters.path_of("density"),
                    format!(
                        "unknown density '{other}'; use \"bounded\" (default) or \"unbounded\""
                    ),
                ));
            }
        };
        let bandwidth = match parameters.value("bandwidth") {
            None => BandwidthRule::Dpi,
            Some(Value::String(name)) => match name.as_str() {
                "dpi" => BandwidthRule::Dpi,
                "nrd0" => BandwidthRule::Nrd0,
                "nrd" => BandwidthRule::Nrd,
                other => {
                    let suggestion = crate::suggest::did_you_mean(other, ["dpi", "nrd0", "nrd"]);
                    return Err(Error::spec(
                        parameters.path_of("bandwidth"),
                        format!(
                            "unknown bandwidth '{other}'{suggestion}; use \"dpi\" (default), \
                             \"nrd0\", \"nrd\" or a positive number"
                        ),
                    ));
                }
            },
            Some(value) => match value.as_f64().filter(|number| *number > 0.0) {
                Some(number) if number.is_finite() => BandwidthRule::Fixed(number),
                _ => {
                    return Err(Error::spec(
                        parameters.path_of("bandwidth"),
                        format!(
                            "expected \"dpi\", \"nrd0\", \"nrd\" or a positive number, got {value}"
                        ),
                    ));
                }
            },
        };
        let adjust = parameters.number("adjust")?.unwrap_or(1.0);
        if adjust <= 0.0 {
            return Err(Error::spec(
                parameters.path_of("adjust"),
                format!("adjust must be positive, got {adjust}"),
            ));
        }
        let n = match parameters.number("n")? {
            None => DEFAULT_N,
            Some(value) if value >= 2.0 && value.fract() == 0.0 && value <= 1e6 => value as usize,
            Some(value) => {
                return Err(Error::spec(
                    parameters.path_of("n"),
                    format!("n must be a whole number from 2 to 1000000, got {value}"),
                ));
            }
        };
        Ok(SlabOptions {
            kind,
            bandwidth,
            adjust,
            trim: parameters.boolean("trim")?.unwrap_or(true),
            n,
        })
    }
}

impl Stat for SlabStat {
    fn name(&self) -> &'static str {
        "slab"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["density", "bandwidth", "adjust", "trim", "n"]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Y]
    }

    fn default_mapping(&self) -> &'static [(Aesthetic, Computed)] {
        &[(Aesthetic::Thickness, Computed::Thickness)]
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        _context: &StatContext,
    ) -> Result<LayerData> {
        let options = SlabOptions::from_parameters(parameters)?;
        let empty = ContinuousColumn::from_values(Vec::new());
        let y = data.continuous(Aesthetic::Y).unwrap_or(&empty);
        let x = data.continuous(Aesthetic::X);
        per_group(&data, |rows| {
            let mut columns: [Vec<f64>; 6] = Default::default();
            for (location, mut values) in values_by_x(rows, x, y) {
                values.sort_by(f64::total_cmp);
                if values.len() < 2 || values[0] == values[values.len() - 1] {
                    continue;
                }
                let (bandwidth, _fell_back) = options.bandwidth.of(&values);
                let estimate = sample_density(
                    &values,
                    options.kind,
                    bandwidth * options.adjust,
                    options.n,
                    options.trim,
                );
                let count = values.len() as f64;
                for index in 0..estimate.grid.len() {
                    let row = [
                        location,
                        estimate.grid[index],
                        estimate.density[index],
                        estimate.cdf[index],
                        estimate.density[index],
                        count,
                    ];
                    for (column, value) in columns.iter_mut().zip(row) {
                        column.push(value);
                    }
                }
            }
            let [x, y, pdf, cdf, thickness, n] = columns;
            Ok(vec![
                (Variable::Aesthetic(Aesthetic::X), x),
                (Variable::Aesthetic(Aesthetic::Y), y),
                (Variable::Computed(Computed::Pdf), pdf),
                (Variable::Computed(Computed::Cdf), cdf),
                (Variable::Computed(Computed::Thickness), thickness),
                (Variable::Computed(Computed::N), n),
            ])
        })
    }
}

/// Finite y values of `rows` by finite x (0 without an x), x in first appearance.
pub(super) fn values_by_x(
    rows: &[usize],
    x: Option<&ContinuousColumn>,
    y: &ContinuousColumn,
) -> Vec<(f64, Vec<f64>)> {
    let mut bins: Vec<(f64, Vec<f64>)> = Vec::new();
    let mut slot_of: HashMap<u64, usize> = HashMap::new();
    for &row in rows {
        let Some(value) = y.get(row).filter(|value| value.is_finite()) else {
            continue;
        };
        let location = match x {
            Some(x) => match x.get(row).filter(|value| value.is_finite()) {
                Some(location) => location,
                None => continue,
            },
            None => 0.0,
        };
        let slot = *slot_of.entry(location.to_bits()).or_insert_with(|| {
            bins.push((location, Vec::new()));
            bins.len() - 1
        });
        bins[slot].1.push(value);
    }
    bins
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Column;
    use crate::spec::Options;

    fn compute(x: &[f64], y: &[f64], options: &str) -> Result<LayerData> {
        let mut data = LayerData::new(y.len());
        let column =
            |values: &[f64]| Column::Continuous(ContinuousColumn::from_values(values.to_vec()));
        data.insert(Aesthetic::X, column(x)).unwrap();
        data.insert(Aesthetic::Y, column(y)).unwrap();
        let options: Options = serde_json::from_str(options).unwrap();
        SlabStat.compute(
            data,
            &Parameters::new(&options, "layers[0]"),
            &StatContext::default(),
        )
    }

    #[test]
    fn one_density_per_x_on_n_points() {
        let y = [1.0, 2.0, 2.5, 3.0, 5.0, 6.0, 6.5, 8.0];
        let data = compute(
            &[0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0],
            &y,
            r#"{"n": 11}"#,
        )
        .unwrap();
        assert_eq!(data.row_count(), 22);
        let x = data.continuous(Aesthetic::X).unwrap();
        assert_eq!((x.get(0), x.get(11)), (Some(0.0), Some(1.0)));
        let grid = data.continuous(Aesthetic::Y).unwrap();
        assert_eq!((grid.get(0), grid.get(10)), (Some(1.0), Some(3.0)));
        let cdf = data.continuous(Computed::Cdf).unwrap();
        assert_eq!((cdf.get(0), cdf.get(10)), (Some(0.25), Some(1.0)));
    }

    #[test]
    fn constant_groups_draw_nothing() {
        let data = compute(&[0.0, 0.0, 1.0], &[2.0, 2.0, 3.0], "{}").unwrap();
        assert_eq!(data.row_count(), 0);
    }

    #[test]
    fn options_are_checked() {
        let error = compute(&[0.0], &[1.0], r#"{"density": "bounds"}"#)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("layers[0].density") && error.contains("bounded"),
            "{error}"
        );
        let error = compute(&[0.0], &[1.0], r#"{"bandwidth": "dpj"}"#)
            .unwrap_err()
            .to_string();
        assert!(error.contains("did you mean 'dpi'"), "{error}");
        let error = compute(&[0.0], &[1.0], r#"{"bandwidth": -1}"#)
            .unwrap_err()
            .to_string();
        assert!(error.contains("positive number"), "{error}");
        let error = compute(&[0.0], &[1.0], r#"{"n": 1}"#)
            .unwrap_err()
            .to_string();
        assert!(error.contains("layers[0].n"), "{error}");
    }
}
