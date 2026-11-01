//! `stat="summary"` and `stat="summary_bin"`: y aggregated per x, or per x bin,
//! as lets-plot's `SummaryStat` / `SummaryBinStat`.
//!
//! Per group: `y`, `ymin`, `ymax` from `fun` (default mean), `fun_min` (default
//! min) and `fun_max` (default max), each one of `count`, `sum`, `mean`,
//! `median`, `min`, `max`, `lq`, `mq`, `uq` (the three `quantiles`, default
//! 0.25, 0.5, 0.75, with lets-plot's quantile rule). `summary` keeps the distinct
//! finite x values in order of first appearance; `summary_bin` bins x over the
//! plot's x range as `stat="bin"` and returns every bin, empty ones as missing.
//!
//! ggplot2's `fun.data` shortcuts (lets-plot #1507) as `fun_data`: `mean_se`
//! (mean +- `mult` standard errors, default 1) and `mean_sdl` (mean +- `mult`
//! standard deviations, default 2), with `fun_args={"mult": k}`.
//!
//! What it does not do: user functions (Python callables), `breaks` for
//! `summary_bin` (lets-plot has none either).

use std::collections::HashMap;

use super::bin::{BinOptions, binning_parameters, finite_range};
use super::groups::per_group;
use super::{Stat, StatContext};
use crate::data::{ContinuousColumn, LayerData, Variable};
use crate::parameters::Parameters;
use crate::spec::{Aesthetic, Value};
use crate::{Error, Result};

/// y aggregated per distinct x.
pub struct SummaryStat;

/// y aggregated per x bin.
pub struct SummaryBinStat;

/// Names `fun`, `fun_min` and `fun_max` accept (lets-plot `Summary.Functions`).
const FUNCTION_NAMES: [&str; 9] = [
    "count", "sum", "mean", "median", "min", "max", "lq", "mq", "uq",
];

/// Names `fun_data` accepts (ggplot2 `fun.data`).
const DATA_FUNCTION_NAMES: [&str; 2] = ["mean_se", "mean_sdl"];

/// lets-plot `SummaryStat.DEF_QUANTILES`.
const DEFAULT_QUANTILES: [f64; 3] = [0.25, 0.5, 0.75];

const SUMMARY_PARAMETERS: &[&str] = &[
    "fun",
    "fun_min",
    "fun_max",
    "fun_data",
    "fun_args",
    "quantiles",
];

impl Stat for SummaryStat {
    fn name(&self) -> &'static str {
        "summary"
    }

    fn parameters(&self) -> &'static [&'static str] {
        SUMMARY_PARAMETERS
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
        let summary = Summary::from_parameters(parameters)?;
        let empty = ContinuousColumn::from_values(Vec::new());
        let x = data.continuous(Aesthetic::X);
        let y = data.continuous(Aesthetic::Y).unwrap_or(&empty);
        per_group(&data, |rows| {
            let mut locations: Vec<f64> = Vec::new();
            let mut values: Vec<Vec<f64>> = Vec::new();
            let mut slot_by_location: HashMap<u64, usize> = HashMap::new();
            for &row in rows {
                // No x: one summary at 0 (lets-plot).
                let location = x.map_or(Some(0.0), |x| x.get(row));
                let (Some(location), Some(value)) = (location, y.get(row)) else {
                    continue;
                };
                if !location.is_finite() || !value.is_finite() {
                    continue;
                }
                let slot = *slot_by_location
                    .entry(location.to_bits())
                    .or_insert_with(|| {
                        locations.push(location);
                        values.push(Vec::new());
                        locations.len() - 1
                    });
                values[slot].push(value);
            }
            Ok(summary.columns(locations, values))
        })
    }
}

impl Stat for SummaryBinStat {
    fn name(&self) -> &'static str {
        "summarybin"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &[
            "fun",
            "fun_min",
            "fun_max",
            "fun_data",
            "fun_args",
            "quantiles",
            "bins",
            "binwidth",
            "center",
            "boundary",
        ]
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
        context: &StatContext,
    ) -> Result<LayerData> {
        let summary = Summary::from_parameters(parameters)?;
        let options = BinOptions::from_parameters(parameters)?;
        let empty = ContinuousColumn::from_values(Vec::new());
        let x = data.continuous(Aesthetic::X).unwrap_or(&empty);
        let y = data.continuous(Aesthetic::Y).unwrap_or(&empty);
        let range = context.x_range.or_else(|| finite_range(x));
        per_group(&data, |rows| {
            let Some(range) = range.filter(|_| data.row_count() > 0) else {
                return Ok(summary.columns(Vec::new(), Vec::new()));
            };
            let (count, width, start) = binning_parameters(range, &options);
            let mut values: Vec<Vec<f64>> = vec![Vec::new(); count];
            for &row in rows {
                let (Some(location), Some(value)) = (x.get(row), y.get(row)) else {
                    continue;
                };
                if !location.is_finite() || !value.is_finite() {
                    continue;
                }
                // Kotlin `floor(NaN).toInt()` is 0: one bin when the range is a point.
                let index = ((location - start) / width).floor();
                let index = if index.is_nan() { 0.0 } else { index };
                if index >= 0.0 && (index as usize) < count {
                    values[index as usize].push(value);
                }
            }
            let locations = (0..count)
                .map(|index| start + width / 2.0 + index as f64 * width)
                .collect();
            Ok(summary.columns(locations, values))
        })
    }
}

/// One aggregate over the sorted values of a location.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Aggregate {
    Count,
    Sum,
    Mean,
    Quantile(f64),
    Min,
    Max,
    /// mean + `mult` x standard error (ggplot2 `mean_se`).
    MeanStandardError(f64),
    /// mean + `mult` x standard deviation (Hmisc `smean.sdl`).
    MeanStandardDeviation(f64),
}

impl Aggregate {
    fn apply(self, sorted: &[f64]) -> f64 {
        let count = sorted.len() as f64;
        let sum = || {
            if sorted.is_empty() {
                f64::NAN
            } else {
                sorted.iter().sum()
            }
        };
        let mean = || sum() / count;
        // Sample variance (n - 1), as R `var`; NaN below two values.
        let variance = || {
            let mean = mean();
            sorted
                .iter()
                .map(|value| (value - mean).powi(2))
                .sum::<f64>()
                / (count - 1.0)
        };
        match self {
            Aggregate::Count => count,
            Aggregate::Sum => sum(),
            Aggregate::Mean => mean(),
            Aggregate::Quantile(probability) => quantile(sorted, probability),
            Aggregate::Min => sorted.first().copied().unwrap_or(f64::NAN),
            Aggregate::Max => sorted.last().copied().unwrap_or(f64::NAN),
            Aggregate::MeanStandardError(mult) if sorted.len() >= 2 => {
                mean() + mult * (variance() / count).sqrt()
            }
            Aggregate::MeanStandardDeviation(mult) if sorted.len() >= 2 => {
                mean() + mult * variance().sqrt()
            }
            Aggregate::MeanStandardError(_) | Aggregate::MeanStandardDeviation(_) => f64::NAN,
        }
    }
}

/// lets-plot `AggregateFunctions.quantile`: the value at `p (n - 1)`, or the
/// mean of its two neighbours when that is not a whole index.
fn quantile(sorted: &[f64], probability: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let place = probability * (sorted.len() - 1) as f64;
    // Kotlin `round` rounds half to even; only equality with `place` matters.
    if place.round_ties_even() == place {
        sorted[place as usize]
    } else {
        (sorted[place.ceil() as usize] + sorted[place.floor() as usize]) / 2.0
    }
}

/// The three aggregates of a summary layer.
struct Summary {
    y: Aggregate,
    ymin: Aggregate,
    ymax: Aggregate,
}

impl Summary {
    fn from_parameters(parameters: &Parameters) -> Result<Summary> {
        let quantiles = quantiles(parameters)?;
        let named = |name: &str, default: Aggregate| -> Result<Aggregate> {
            let Some(given) = parameters.string(name)? else {
                return Ok(default);
            };
            Ok(match given {
                "count" => Aggregate::Count,
                "sum" => Aggregate::Sum,
                "mean" => Aggregate::Mean,
                "median" => Aggregate::Quantile(0.5),
                "min" => Aggregate::Min,
                "max" => Aggregate::Max,
                "lq" => Aggregate::Quantile(quantiles[0]),
                "mq" => Aggregate::Quantile(quantiles[1]),
                "uq" => Aggregate::Quantile(quantiles[2]),
                _ => {
                    let hint = if DATA_FUNCTION_NAMES.contains(&given) {
                        format!("; '{given}' sets y, ymin and ymax: pass fun_data='{given}'")
                    } else {
                        crate::suggest::did_you_mean(given, FUNCTION_NAMES)
                    };
                    return Err(Error::spec(
                        parameters.path_of(name),
                        format!(
                            "unknown summary function '{given}'; expected one of {}{hint}",
                            FUNCTION_NAMES.join(", ")
                        ),
                    ));
                }
            })
        };
        if let Some(data_function) = parameters.string("fun_data")? {
            if let Some(name) = ["fun", "fun_min", "fun_max"]
                .into_iter()
                .find(|name| parameters.string(name).ok().flatten().is_some())
            {
                return Err(Error::spec(
                    parameters.path_of(name),
                    format!(
                        "'{name}' cannot be combined with fun_data='{data_function}', which \
                         sets y, ymin and ymax; drop one of them"
                    ),
                ));
            }
            let (default_mult, spread): (f64, fn(f64) -> Aggregate) = match data_function {
                "mean_se" => (1.0, Aggregate::MeanStandardError),
                "mean_sdl" => (2.0, Aggregate::MeanStandardDeviation),
                _ => {
                    return Err(Error::spec(
                        parameters.path_of("fun_data"),
                        format!(
                            "unknown summary data function '{data_function}'; expected 'mean_se' \
                             or 'mean_sdl'{}",
                            crate::suggest::did_you_mean(data_function, DATA_FUNCTION_NAMES)
                        ),
                    ));
                }
            };
            let mult = mult(parameters)?.unwrap_or(default_mult);
            return Ok(Summary {
                y: Aggregate::Mean,
                ymin: spread(-mult),
                ymax: spread(mult),
            });
        }
        if parameters.contains("fun_args") && mult(parameters)?.is_some() {
            return Err(Error::spec(
                parameters.path_of("fun_args"),
                "fun_args applies to fun_data ('mean_se', 'mean_sdl'); pass fun_data too or \
                 drop fun_args",
            ));
        }
        Ok(Summary {
            y: named("fun", Aggregate::Mean)?,
            ymin: named("fun_min", Aggregate::Min)?,
            ymax: named("fun_max", Aggregate::Max)?,
        })
    }

    /// x, y, ymin, ymax columns of one group.
    fn columns(&self, locations: Vec<f64>, values: Vec<Vec<f64>>) -> Vec<(Variable, Vec<f64>)> {
        let mut y = Vec::with_capacity(values.len());
        let mut ymin = Vec::with_capacity(values.len());
        let mut ymax = Vec::with_capacity(values.len());
        for mut bin in values {
            bin.sort_by(f64::total_cmp);
            y.push(self.y.apply(&bin));
            ymin.push(self.ymin.apply(&bin));
            ymax.push(self.ymax.apply(&bin));
        }
        vec![
            (Variable::Aesthetic(Aesthetic::X), locations),
            (Variable::Aesthetic(Aesthetic::Y), y),
            (Variable::Aesthetic(Aesthetic::Ymin), ymin),
            (Variable::Aesthetic(Aesthetic::Ymax), ymax),
        ]
    }
}

/// `quantiles`: three probabilities in 0..1, sorted (lets-plot).
fn quantiles(parameters: &Parameters) -> Result<[f64; 3]> {
    let Some(value) = parameters
        .value("quantiles")
        .filter(|value| !value.is_null())
    else {
        return Ok(DEFAULT_QUANTILES);
    };
    let invalid = || {
        Error::spec(
            parameters.path_of("quantiles"),
            format!("expected three numbers between 0 and 1, got {value}"),
        )
    };
    let Value::Array(items) = value else {
        return Err(invalid());
    };
    let mut probabilities: Vec<f64> = items
        .iter()
        .map(|item| item.as_f64().filter(|p| (0.0..=1.0).contains(p)))
        .collect::<Option<_>>()
        .ok_or_else(invalid)?;
    if probabilities.len() != 3 {
        return Err(invalid());
    }
    probabilities.sort_by(f64::total_cmp);
    Ok([probabilities[0], probabilities[1], probabilities[2]])
}

/// `fun_args={"mult": k}`.
fn mult(parameters: &Parameters) -> Result<Option<f64>> {
    let Some(value) = parameters
        .value("fun_args")
        .filter(|value| !value.is_null())
    else {
        return Ok(None);
    };
    let invalid = || {
        Error::spec(
            parameters.path_of("fun_args"),
            format!("expected a dict like {{'mult': 2}}, got {value}"),
        )
    };
    let Value::Object(entries) = value else {
        return Err(invalid());
    };
    if let Some(key) = entries.keys().find(|key| *key != "mult") {
        return Err(Error::spec(
            parameters.path_of("fun_args"),
            format!("unknown argument '{key}'; fun_args takes only 'mult'"),
        ));
    }
    match entries.get("mult") {
        None => Ok(None),
        Some(mult) => mult
            .as_f64()
            .filter(|mult| mult.is_finite() && *mult >= 0.0)
            .map(Some)
            .ok_or_else(invalid),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantile_follows_lets_plot() {
        let values = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(quantile(&values, 0.5), 2.5);
        // p (n - 1) = 0.75: mean of the neighbours (R type 7 gives 1.75).
        assert_eq!(quantile(&values, 0.25), 1.5);
        assert_eq!(quantile(&values, 0.0), 1.0);
        assert_eq!(quantile(&values, 1.0), 4.0);
        assert_eq!(quantile(&[1.0, 2.0, 3.0], 0.5), 2.0);
        assert!(quantile(&[], 0.5).is_nan());
    }

    #[test]
    fn mean_se_and_sdl_as_r() {
        // R: x <- c(1, 2, 4, 9); mean_se(x); mean_sdl(x)
        let values = [1.0, 2.0, 4.0, 9.0];
        let close = |left: f64, right: f64| (left - right).abs() < 1e-12;
        assert!(close(
            Aggregate::MeanStandardError(-1.0).apply(&values),
            2.2204869579947815
        ));
        assert!(close(
            Aggregate::MeanStandardError(1.0).apply(&values),
            5.7795130420052185
        ));
        assert!(close(
            Aggregate::MeanStandardDeviation(2.0).apply(&values),
            11.118052168020874
        ));
        assert!(Aggregate::MeanStandardError(1.0).apply(&[3.0]).is_nan());
    }

    #[test]
    fn empty_bins_are_missing() {
        assert!(Aggregate::Mean.apply(&[]).is_nan());
        assert!(Aggregate::Sum.apply(&[]).is_nan());
        assert_eq!(Aggregate::Count.apply(&[]), 0.0);
    }
}
