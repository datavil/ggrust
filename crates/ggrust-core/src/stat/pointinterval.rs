//! `stat="pointinterval"` and `stat="interval"`: a point summary and quantile
//! intervals of y per group and x, as the R package ggdist's `stat_pointinterval`
//! / `stat_interval` compute them for sample data.
//!
//! Per group and distinct x (first appearance), one row per interval level in
//! `widths` (widest first, so narrower intervals draw on top): `y` the point
//! (median, mean or mode, by `point_interval`), `ymin`/`ymax` the equal-tailed
//! quantile interval holding that share of the values (R type 7 quantiles),
//! `...width..` the level and `..level..` the level as a discrete value (widest
//! first, as ggdist's ordered factor). `interval` colors by `..level..`; the
//! `pointinterval` geom sizes lines by `...width..`.
//!
//! Options: `point_interval` (`"median_qi"`, default, `"mean_qi"`, `"mode_qi"`),
//! `widths` (default 0.66 and 0.95; 0.5, 0.8 and 0.95 for `interval`).
//!
//! What it does not do: highest-density intervals (`hdi`, `hdci`), weights,
//! distribution aesthetics.

use super::boxplot::quantile;
use super::ggdist_density::mode;
use super::groups::per_group;
use super::slab::values_by_x;
use super::{Stat, StatContext};
use crate::data::{Column, Computed, ContinuousColumn, DiscreteColumn, LayerData, Variable};
use crate::parameters::Parameters;
use crate::spec::{Aesthetic, Value};
use crate::{Error, Result};

/// ggdist `stat_pointinterval` / `stat_halfeye` `.width`.
const POINTINTERVAL_WIDTHS: &[f64] = &[0.66, 0.95];
/// ggdist `stat_interval` `.width`.
const INTERVAL_WIDTHS: &[f64] = &[0.5, 0.8, 0.95];

/// Point summary plus intervals (`stat_pointinterval`).
pub struct PointIntervalStat;

/// Intervals colored by level (`stat_interval`).
pub struct IntervalStat;

impl Stat for PointIntervalStat {
    fn name(&self) -> &'static str {
        "pointinterval"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["point_interval", "widths"]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Y]
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        _context: &StatContext,
    ) -> Result<LayerData> {
        let widths = widths(parameters, POINTINTERVAL_WIDTHS)?;
        intervals(&data, parameters, &widths)
    }
}

impl Stat for IntervalStat {
    fn name(&self) -> &'static str {
        "interval"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["point_interval", "widths"]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Y]
    }

    fn default_mapping(&self) -> &'static [(Aesthetic, Computed)] {
        &[(Aesthetic::Color, Computed::Level)]
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        _context: &StatContext,
    ) -> Result<LayerData> {
        let widths = widths(parameters, INTERVAL_WIDTHS)?;
        intervals(&data, parameters, &widths)
    }
}

/// The point summary of `point_interval`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PointSummary {
    Median,
    Mean,
    Mode,
}

impl PointSummary {
    const NAMES: [&'static str; 3] = ["median_qi", "mean_qi", "mode_qi"];

    fn from_parameters(parameters: &Parameters) -> Result<Self> {
        match parameters.string("point_interval")? {
            None | Some("median_qi") => Ok(PointSummary::Median),
            Some("mean_qi") => Ok(PointSummary::Mean),
            Some("mode_qi") => Ok(PointSummary::Mode),
            Some(other) => Err(Error::spec(
                parameters.path_of("point_interval"),
                format!(
                    "unsupported point_interval '{other}'{}; use one of: {}",
                    crate::suggest::did_you_mean(other, PointSummary::NAMES),
                    PointSummary::NAMES.join(", ")
                ),
            )),
        }
    }

    /// The point of sorted values (`data_order`: the same values unsorted).
    fn of(self, sorted: &[f64], data_order: &[f64]) -> f64 {
        match self {
            PointSummary::Median => quantile(sorted, 0.5),
            PointSummary::Mean => sorted.iter().sum::<f64>() / sorted.len() as f64,
            PointSummary::Mode => mode(sorted, data_order),
        }
    }
}

/// Rows of every group, x and level, plus the discrete `..level..`.
fn intervals(data: &LayerData, parameters: &Parameters, widths: &[f64]) -> Result<LayerData> {
    let summary = PointSummary::from_parameters(parameters)?;
    let empty = ContinuousColumn::from_values(Vec::new());
    let y = data.continuous(Aesthetic::Y).unwrap_or(&empty);
    let x = data.continuous(Aesthetic::X);
    let mut result = per_group(data, |rows| {
        let mut columns: [Vec<f64>; 5] = Default::default();
        for (location, values) in values_by_x(rows, x, y) {
            let mut sorted = values.clone();
            sorted.sort_by(f64::total_cmp);
            let point = summary.of(&sorted, &values);
            for &width in widths {
                let tail = (1.0 - width) / 2.0;
                let row = [
                    location,
                    point,
                    quantile(&sorted, tail),
                    quantile(&sorted, 1.0 - tail),
                    width,
                ];
                for (column, value) in columns.iter_mut().zip(row) {
                    column.push(value);
                }
            }
        }
        let [x, y, ymin, ymax, width] = columns;
        Ok(vec![
            (Variable::Aesthetic(Aesthetic::X), x),
            (Variable::Aesthetic(Aesthetic::Y), y),
            (Variable::Aesthetic(Aesthetic::Ymin), ymin),
            (Variable::Aesthetic(Aesthetic::Ymax), ymax),
            (Variable::Computed(Computed::IntervalWidth), width),
        ])
    })?;
    let levels: Vec<String> = match result.row_count() {
        0 => Vec::new(),
        _ => widths.iter().map(|width| format!("{width}")).collect(),
    };
    let codes: Vec<Option<u32>> = result
        .continuous(Computed::IntervalWidth)
        .map(|column| {
            (0..column.len())
                .map(|row| {
                    let width = column.get(row)?;
                    widths
                        .iter()
                        .position(|candidate| *candidate == width)
                        .map(|index| index as u32)
                })
                .collect()
        })
        .unwrap_or_default();
    result.insert(
        Computed::Level,
        Column::Discrete(DiscreteColumn::from_parts(levels, codes.into())),
    )?;
    Ok(result)
}

/// `widths`, sorted widest first (duplicates dropped); each in (0, 1].
fn widths(parameters: &Parameters, default: &[f64]) -> Result<Vec<f64>> {
    let mut widths = match parameters.value("widths") {
        None => default.to_vec(),
        Some(value) => {
            let invalid = || {
                Error::spec(
                    parameters.path_of("widths"),
                    format!(
                        "expected a non-empty list of interval widths in (0, 1], e.g. \
                         [0.66, 0.95], got {value}"
                    ),
                )
            };
            let items = match value {
                Value::Array(items) if !items.is_empty() => items.clone(),
                Value::Number(_) => vec![value.clone()],
                _ => return Err(invalid()),
            };
            items
                .iter()
                .map(|item| {
                    item.as_f64()
                        .filter(|width| *width > 0.0 && *width <= 1.0)
                        .ok_or_else(invalid)
                })
                .collect::<Result<Vec<f64>>>()?
        }
    };
    widths.sort_by(|left, right| right.total_cmp(left));
    widths.dedup();
    Ok(widths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Options;

    fn compute(stat: &dyn Stat, y: &[f64], options: &str) -> Result<LayerData> {
        let mut data = LayerData::new(y.len());
        let column = Column::Continuous(ContinuousColumn::from_values(y.to_vec()));
        data.insert(Aesthetic::Y, column).unwrap();
        let options: Options = serde_json::from_str(options).unwrap();
        stat.compute(
            data,
            &Parameters::new(&options, "layers[0]"),
            &StatContext::default(),
        )
    }

    fn values(data: &LayerData, variable: impl Into<Variable>) -> Vec<f64> {
        let column = data.continuous(variable).unwrap();
        (0..column.len())
            .map(|row| column.get(row).unwrap())
            .collect()
    }

    #[test]
    fn median_and_type_7_intervals_widest_first() {
        let y: Vec<f64> = (1..=11).map(f64::from).collect();
        let data = compute(&PointIntervalStat, &y, "{}").unwrap();
        assert_eq!(values(&data, Aesthetic::Y), [6.0, 6.0]);
        assert_eq!(values(&data, Computed::IntervalWidth), [0.95, 0.66]);
        let ymin = values(&data, Aesthetic::Ymin);
        assert!((ymin[0] - 1.25).abs() < 1e-12 && (ymin[1] - 2.7).abs() < 1e-12);
        let Some(Column::Discrete(level)) = data.get(Computed::Level) else {
            panic!("level is discrete");
        };
        assert_eq!(level.levels(), ["0.95", "0.66"]);
    }

    #[test]
    fn interval_stat_defaults_to_three_levels() {
        let data = compute(
            &IntervalStat,
            &[1.0, 2.0, 4.0],
            r#"{"point_interval": "mean_qi"}"#,
        )
        .unwrap();
        assert_eq!(values(&data, Computed::IntervalWidth), [0.95, 0.8, 0.5]);
        assert!((values(&data, Aesthetic::Y)[0] - 7.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn options_are_checked() {
        let error = compute(&IntervalStat, &[1.0], r#"{"point_interval": "median_hdi"}"#)
            .unwrap_err()
            .to_string();
        assert!(error.contains("layers[0].point_interval") && error.contains("median_qi"));
        let error = compute(&IntervalStat, &[1.0], r#"{"widths": [0.5, 1.5]}"#)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("layers[0].widths") && error.contains("(0, 1]"),
            "{error}"
        );
    }

    #[test]
    fn points_and_intervals_match_ggdist() {
        let reference: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ggdist_reference.json"
        ))
        .unwrap();
        let numbers = |value: &serde_json::Value| -> Vec<f64> {
            let items = value.as_array().unwrap();
            items.iter().map(|item| item.as_f64().unwrap()).collect()
        };
        for (name, sample) in reference["samples"].as_object().unwrap() {
            let y = numbers(&sample["values"]);
            for summary in PointSummary::NAMES {
                let options =
                    format!(r#"{{"point_interval": "{summary}", "widths": [0.5, 0.66, 0.95]}}"#);
                let data = compute(&PointIntervalStat, &y, &options).unwrap();
                let expected = &sample[summary];
                // Rows run widest first; R's run in the order given.
                for (key, aesthetic) in [
                    ("point", Aesthetic::Y),
                    ("lower", Aesthetic::Ymin),
                    ("upper", Aesthetic::Ymax),
                ] {
                    let mut actual = values(&data, aesthetic);
                    actual.reverse();
                    for (got, want) in actual.iter().zip(numbers(&expected[key])) {
                        assert!(
                            (got - want).abs() < 1e-9,
                            "{name} {summary} {key}: {got} vs {want}"
                        );
                    }
                }
            }
        }
    }
}
