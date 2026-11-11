//! `stat="qq"`, `"qq_line"`, `"qq2"`, `"qq2_line"`: quantile-quantile comparisons,
//! as lets-plot's `QQStat`, `QQLineStat`, `QQ2Stat`, `QQ2LineStat` and `QQStatUtil`.
//!
//! `qq` sorts the `sample` of each group and pairs its values with the theoretical
//! quantiles at `(i - 0.5) / n`; rows stay rows, so a row's color follows its value.
//! `qq_line` is the line through the `quantiles` pair (default 0.25, 0.75), drawn
//! between the outermost points. `qq2` and `qq2_line` compare the `x` and `y` samples
//! with each other, by R-8 quantile estimates.
//!
//! What it does not do: distributions other than `norm`, `uniform`, `t`, `gamma`,
//! `exp` and `chi2`, or `orientation="y"`.

use serde_json::Value;

use super::distribution::{gamma_quantile, normal_quantile};
use super::groups::{per_group, rows_by_group};
use super::smooth::t_quantile;
use super::{Stat, StatContext};
use crate::data::{Column, Computed, ContinuousColumn, LayerData, Variable};
use crate::parameters::Parameters;
use crate::spec::Aesthetic;
use crate::suggest::did_you_mean;
use crate::{Error, Result};

/// lets-plot `QQLineStat.DEF_LINE_QUANTILES`.
const DEFAULT_LINE_QUANTILES: (f64, f64) = (0.25, 0.75);

const DISTRIBUTIONS: [&str; 6] = ["norm", "uniform", "t", "gamma", "exp", "chi2"];

const THEORETICAL_MAPPING: &[(Aesthetic, Computed)] = &[
    (Aesthetic::X, Computed::Theoretical),
    (Aesthetic::Y, Computed::Sample),
];

/// A theoretical distribution with its parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Distribution {
    Normal { mean: f64, deviation: f64 },
    Uniform { low: f64, high: f64 },
    StudentT { freedom: f64 },
    Gamma { shape: f64, rate: f64 },
}

impl Distribution {
    /// `distribution=` and `dparams=`, with lets-plot's defaults.
    fn from_parameters(parameters: &Parameters) -> Result<Distribution> {
        let name = parameters.string("distribution")?.unwrap_or("norm");
        let given = numbers(parameters, "dparams")?.unwrap_or_default();
        let at = |index: usize, default: f64| given.get(index).copied().unwrap_or(default);
        let positive = |index: usize, default: f64, what: &str| {
            let value = at(index, default);
            match value > 0.0 && value.is_finite() {
                true => Ok(value),
                false => Err(Error::spec(
                    parameters.path_of("dparams"),
                    format!(
                        "distribution='{name}' needs a positive {what}, got {value}; \
                         pass e.g. dparams=[{default}]"
                    ),
                )),
            }
        };
        match name {
            "norm" => Ok(Distribution::Normal {
                mean: at(0, 0.0),
                deviation: positive(1, 1.0, "standard deviation")?,
            }),
            "uniform" => {
                let (low, high) = (at(0, 0.0), at(1, 1.0));
                if low >= high {
                    return Err(Error::spec(
                        parameters.path_of("dparams"),
                        format!(
                            "distribution='uniform' needs a < b in dparams=[a, b], got \
                             [{low}, {high}]; pass e.g. dparams=[0, 1]"
                        ),
                    ));
                }
                Ok(Distribution::Uniform { low, high })
            }
            "t" => Ok(Distribution::StudentT {
                freedom: positive(0, 1.0, "degrees of freedom")?,
            }),
            "gamma" => Ok(Distribution::Gamma {
                shape: positive(0, 1.0, "shape (alpha)")?,
                rate: positive(1, 1.0, "rate (beta)")?,
            }),
            "exp" => Ok(Distribution::Gamma {
                shape: 1.0,
                rate: positive(0, 1.0, "rate (lambda)")?,
            }),
            "chi2" => Ok(Distribution::Gamma {
                shape: positive(0, 1.0, "degrees of freedom")? / 2.0,
                rate: 0.5,
            }),
            other => Err(Error::spec(
                parameters.path_of("distribution"),
                format!(
                    "unknown distribution '{other}'{}; expected one of: {}",
                    did_you_mean(other, DISTRIBUTIONS),
                    DISTRIBUTIONS.join(", ")
                ),
            )),
        }
    }

    /// The quantile at `p`, `0 < p < 1`.
    fn quantile(self, p: f64) -> f64 {
        match self {
            Distribution::Normal { mean, deviation } => mean + deviation * normal_quantile(p),
            Distribution::Uniform { low, high } => low + p * (high - low),
            Distribution::StudentT { freedom } if p < 0.5 => -t_quantile(1.0 - p, freedom),
            Distribution::StudentT { freedom } => t_quantile(p, freedom),
            Distribution::Gamma { shape, rate } => gamma_quantile(p, shape) / rate,
        }
    }
}

/// A list of numbers. Null counts as absent.
fn numbers(parameters: &Parameters, name: &str) -> Result<Option<Vec<f64>>> {
    let invalid = |value: &Value| {
        Error::spec(
            parameters.path_of(name),
            format!("expected a list of numbers, got {value}"),
        )
    };
    match parameters.value(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| item.as_f64().ok_or_else(|| invalid(item)))
            .collect::<Result<Vec<f64>>>()
            .map(Some),
        Some(value) => Err(invalid(value)),
    }
}

/// `quantiles=`: a pair of probabilities.
fn line_quantiles(parameters: &Parameters) -> Result<(f64, f64)> {
    match numbers(parameters, "quantiles")?.as_deref() {
        None => Ok(DEFAULT_LINE_QUANTILES),
        Some(&[low, high])
            if (0.0..=1.0).contains(&low) && (0.0..=1.0).contains(&high) && low != high =>
        {
            Ok((low, high))
        }
        Some(other) => Err(Error::spec(
            parameters.path_of("quantiles"),
            format!(
                "expected two different probabilities between 0 and 1, got {other:?}; \
                 pass e.g. quantiles=[0.25, 0.75]"
            ),
        )),
    }
}

/// The finite values of `column` at `rows`, sorted.
fn sorted_values(column: &ContinuousColumn, rows: &[usize]) -> Vec<f64> {
    let mut values: Vec<f64> = rows
        .iter()
        .filter_map(|&row| column.get(row).filter(|value| value.is_finite()))
        .collect();
    values.sort_by(f64::total_cmp);
    values
}

/// lets-plot `QQStatUtil.getEstimatedQuantile`: the R-8 estimate from a sorted,
/// non-empty sample.
fn estimated_quantile(sorted: &[f64], p: f64) -> f64 {
    let n = sorted.len();
    let count = n as f64;
    if n == 1 {
        return sorted[0];
    }
    if p < 1.0 / count {
        return sorted[0] + (count + 1.0) / (3.0 * count) * (sorted[1] - sorted[0]);
    }
    if p > (count - 1.0) / count {
        return sorted[n - 2]
            + (2.0 * count - 1.0) / (3.0 * count) * (sorted[n - 1] - sorted[n - 2]);
    }
    let h = (count + 1.0 / 3.0) * p - 2.0 / 3.0;
    let (i, j) = (h.floor() as usize, h.ceil() as usize);
    sorted[i] + (h - i as f64) * (sorted[j] - sorted[i])
}

/// lets-plot `QQStatUtil.lineByPoints`: the line through two points, `x` different.
fn line_through(x: (f64, f64), y: (f64, f64)) -> impl Fn(f64) -> f64 {
    let slope = (y.1 - y.0) / (x.1 - x.0);
    let intercept = y.0 - slope * x.0;
    move |at| slope * at + intercept
}

/// Sample against theoretical quantiles.
pub struct QqStat;

impl Stat for QqStat {
    fn name(&self) -> &'static str {
        "qq"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["distribution", "dparams"]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Sample]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Sample]
    }

    fn default_mapping(&self) -> &'static [(Aesthetic, Computed)] {
        THEORETICAL_MAPPING
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        _context: &StatContext,
    ) -> Result<LayerData> {
        let distribution = Distribution::from_parameters(parameters)?;
        let empty = ContinuousColumn::from_values(Vec::new());
        let sample = data.continuous(Aesthetic::Sample).unwrap_or(&empty);
        let mut order: Vec<u32> = Vec::with_capacity(data.row_count());
        let mut theoretical: Vec<f64> = Vec::with_capacity(data.row_count());
        let mut sorted: Vec<f64> = Vec::with_capacity(data.row_count());
        for rows in rows_by_group(&data) {
            let mut finite: Vec<(f64, usize)> = rows
                .iter()
                .filter_map(|&row| Some((sample.get(row).filter(|v| v.is_finite())?, row)))
                .collect();
            finite.sort_by(|left, right| left.0.total_cmp(&right.0));
            let count = finite.len() as f64;
            for (index, (value, row)) in finite.into_iter().enumerate() {
                order.push(row as u32);
                sorted.push(value);
                theoretical.push(distribution.quantile((index as f64 + 0.5) / count));
            }
        }
        let mut result = data.take(&order);
        result.remove(Aesthetic::Sample);
        result.insert(
            Variable::Computed(Computed::Theoretical),
            Column::Continuous(ContinuousColumn::from_values(theoretical)),
        )?;
        result.insert(
            Variable::Computed(Computed::Sample),
            Column::Continuous(ContinuousColumn::from_values(sorted)),
        )?;
        Ok(result)
    }
}

/// The fitting line of a Q-Q plot.
pub struct QqLineStat;

impl Stat for QqLineStat {
    fn name(&self) -> &'static str {
        "qq_line"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["distribution", "dparams", "quantiles"]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Sample]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Sample]
    }

    fn default_mapping(&self) -> &'static [(Aesthetic, Computed)] {
        THEORETICAL_MAPPING
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        _context: &StatContext,
    ) -> Result<LayerData> {
        let distribution = Distribution::from_parameters(parameters)?;
        let quantiles = line_quantiles(parameters)?;
        let empty = ContinuousColumn::from_values(Vec::new());
        let sample = data.continuous(Aesthetic::Sample).unwrap_or(&empty);
        let mut result = per_group(&data, |rows| {
            let sorted = sorted_values(sample, rows);
            let (theoretical, values) = theoretical_line(&sorted, quantiles, distribution);
            Ok(vec![
                (Variable::Computed(Computed::Theoretical), theoretical),
                (Variable::Computed(Computed::Sample), values),
            ])
        })?;
        result.remove(Aesthetic::Sample);
        Ok(result)
    }
}

/// The line's two end points: `(theoretical, sample)`, empty without a sample.
fn theoretical_line(
    sorted: &[f64],
    quantiles: (f64, f64),
    distribution: Distribution,
) -> (Vec<f64>, Vec<f64>) {
    let (Some(&first), Some(&last)) = (sorted.first(), sorted.last()) else {
        return (Vec::new(), Vec::new());
    };
    let edge = 0.5 / sorted.len() as f64;
    let sample = (
        estimated_quantile(sorted, quantiles.0),
        estimated_quantile(sorted, quantiles.1),
    );
    // min / max keep the probabilities off 0 and 1, whose quantiles may be infinite.
    let theoretical = (
        distribution.quantile(quantiles.0.max(edge)),
        distribution.quantile(quantiles.1.min(1.0 - edge)),
    );
    if theoretical.0 == theoretical.1 {
        return (vec![theoretical.0, theoretical.1], vec![first, last]);
    }
    let line = line_through(theoretical, sample);
    let ends = vec![
        distribution.quantile(edge),
        distribution.quantile(1.0 - edge),
    ];
    let values = ends.iter().map(|&at| line(at)).collect();
    (ends, values)
}

/// One sample against another.
pub struct Qq2Stat;

impl Stat for Qq2Stat {
    fn name(&self) -> &'static str {
        "qq2"
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn compute(
        &self,
        data: LayerData,
        _parameters: &Parameters,
        _context: &StatContext,
    ) -> Result<LayerData> {
        per_samples(&data, |x, y| {
            let count = x.len() as f64;
            let probabilities = (1..=x.len()).map(|index| (index as f64 - 0.5) / count);
            probabilities
                .map(|p| (estimated_quantile(x, p), estimated_quantile(y, p)))
                .unzip()
        })
    }
}

/// The fitting line of a two-sample Q-Q plot.
pub struct Qq2LineStat;

impl Stat for Qq2LineStat {
    fn name(&self) -> &'static str {
        "qq2_line"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["quantiles"]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        _context: &StatContext,
    ) -> Result<LayerData> {
        let quantiles = line_quantiles(parameters)?;
        per_samples(&data, |x, y| {
            let at_x = (
                estimated_quantile(x, quantiles.0),
                estimated_quantile(x, quantiles.1),
            );
            let at_y = (
                estimated_quantile(y, quantiles.0),
                estimated_quantile(y, quantiles.1),
            );
            let (first, last) = (x[0], x[x.len() - 1]);
            if at_x.0 == at_x.1 {
                return (vec![at_x.0, at_x.1], vec![y[0], y[y.len() - 1]]);
            }
            let line = line_through(at_x, at_y);
            (vec![first, last], vec![line(first), line(last)])
        })
    }
}

/// Run `compute` on the sorted finite x and y of every group; a group without
/// finite values in either gives no rows.
fn per_samples(
    data: &LayerData,
    compute: impl Fn(&[f64], &[f64]) -> (Vec<f64>, Vec<f64>) + Sync,
) -> Result<LayerData> {
    let empty = ContinuousColumn::from_values(Vec::new());
    let x = data.continuous(Aesthetic::X).unwrap_or(&empty);
    let y = data.continuous(Aesthetic::Y).unwrap_or(&empty);
    per_group(data, |rows| {
        let (sorted_x, sorted_y) = (sorted_values(x, rows), sorted_values(y, rows));
        let (at_x, at_y) = match sorted_x.is_empty() || sorted_y.is_empty() {
            true => (Vec::new(), Vec::new()),
            false => compute(&sorted_x, &sorted_y),
        };
        Ok(vec![
            (Variable::Aesthetic(Aesthetic::X), at_x),
            (Variable::Aesthetic(Aesthetic::Y), at_y),
        ])
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const NORMAL: Distribution = Distribution::Normal {
        mean: 0.0,
        deviation: 1.0,
    };

    #[test]
    fn quantiles_match_r() {
        let cases = [
            (NORMAL, 0.975, 1.959963984540054),
            (NORMAL, 1e-10, -6.361340902404056),
            (
                Distribution::StudentT { freedom: 5.0 },
                0.1,
                -1.4758840488244815,
            ),
            (
                Distribution::Gamma {
                    shape: 2.5,
                    rate: 2.0,
                },
                0.3,
                0.7499770331899765,
            ),
            (
                Distribution::Gamma {
                    shape: 1.5,
                    rate: 0.5,
                },
                0.95,
                7.814727903251178,
            ),
            (
                Distribution::Uniform {
                    low: 2.0,
                    high: 4.0,
                },
                0.25,
                2.5,
            ),
        ];
        for (distribution, p, expected) in cases {
            let value = distribution.quantile(p);
            assert!(
                (value - expected).abs() < 1e-9,
                "{distribution:?} {p}: {value}"
            );
        }
    }

    #[test]
    fn estimated_quantile_follows_r8() {
        let sorted = [1.0, 2.0, 4.0, 8.0];
        // R: quantile(c(1,2,4,8), 0.5, type = 8) = 3.
        assert_eq!(estimated_quantile(&sorted, 0.5), 3.0);
        assert_eq!(estimated_quantile(&[5.0], 0.2), 5.0);
    }

    #[test]
    fn line_passes_through_the_quartiles() {
        let sorted: Vec<f64> = (1..=9).map(f64::from).collect();
        let (x, y) = theoretical_line(&sorted, (0.25, 0.75), NORMAL);
        assert_eq!(x.len(), 2);
        let slope = (y[1] - y[0]) / (x[1] - x[0]);
        let q1 = estimated_quantile(&sorted, 0.25);
        let at = y[0] + slope * (normal_quantile(0.25) - x[0]);
        assert!((at - q1).abs() < 1e-12);
    }
}
