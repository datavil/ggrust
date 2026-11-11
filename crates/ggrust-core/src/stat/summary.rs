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
//! standard deviations, default 2), with `fun_args={"mult": k}`. `mean_cl_normal`
//! (t interval), `mean_cl_boot` (bootstrap percentile interval) and `median_hilow`
//! (the central quantile range) are Hmisc's `smean.cl.normal`, `smean.cl.boot` and
//! `smedian.hilow`, with `fun_args` `conf_int` (default 0.95) and, for the bootstrap,
//! `B` (default 1000) and `seed` (fixed by default, so output is reproducible).
//!
//! What it does not do: user functions (Python callables), `breaks` for
//! `summary_bin` (lets-plot has none either).

use std::collections::HashMap;

use super::bin::{BinOptions, binning_parameters, finite_range};
use super::boxplot::quantile as type_7_quantile;
use super::groups::per_group;
use super::smooth::t_quantile;
use super::{Stat, StatContext};
use crate::data::{ContinuousColumn, LayerData, Variable};
use crate::parameters::Parameters;
use crate::position::random::KotlinRandom;
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
const DATA_FUNCTION_NAMES: [&str; 5] = [
    "mean_se",
    "mean_sdl",
    "mean_cl_normal",
    "mean_cl_boot",
    "median_hilow",
];

/// Hmisc's default confidence level.
const DEFAULT_CONFIDENCE: f64 = 0.95;
/// Hmisc `smean.cl.boot` default resample count.
const DEFAULT_BOOTSTRAP_SAMPLES: usize = 1000;
/// Default bootstrap seed: output must not change from run to run.
const DEFAULT_BOOTSTRAP_SEED: i64 = 1;

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
pub(super) enum Aggregate {
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
    /// mean + `t` x standard error, `t` signed (Hmisc `smean.cl.normal`).
    MeanConfidenceNormal {
        /// Quantile probability of Student's t: below 0.5 for the lower end.
        probability: f64,
    },
    /// A percentile of the bootstrapped means (Hmisc `smean.cl.boot`).
    MeanConfidenceBootstrap {
        probability: f64,
        samples: usize,
        seed: i64,
    },
    /// R's type 7 quantile (Hmisc `smedian.hilow`).
    QuantileType7(f64),
}

/// Names `stat_summary_2d`'s `fun` accepts.
pub(super) const PLAIN_FUNCTION_NAMES: [&str; 6] = ["count", "sum", "mean", "median", "min", "max"];

impl Aggregate {
    /// The aggregate of a plain name (no quantile options), as `stat_summary_2d`.
    pub(super) fn named(name: &str) -> Option<Aggregate> {
        Some(match name {
            "count" => Aggregate::Count,
            "sum" => Aggregate::Sum,
            "mean" => Aggregate::Mean,
            "median" => Aggregate::Quantile(0.5),
            "min" => Aggregate::Min,
            "max" => Aggregate::Max,
            _ => return None,
        })
    }

    /// The aggregate of values sorted ascending.
    pub(super) fn apply(self, sorted: &[f64]) -> f64 {
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
            Aggregate::MeanConfidenceNormal { probability } if sorted.len() >= 2 => {
                // t_quantile takes the upper half only.
                let freedom = count - 1.0;
                let t = if probability < 0.5 {
                    -t_quantile(1.0 - probability, freedom)
                } else {
                    t_quantile(probability, freedom)
                };
                mean() + t * (variance() / count).sqrt()
            }
            Aggregate::MeanConfidenceBootstrap {
                probability,
                samples,
                seed,
            } if sorted.len() >= 2 => bootstrap_mean_quantile(sorted, probability, samples, seed),
            Aggregate::MeanStandardError(_)
            | Aggregate::MeanStandardDeviation(_)
            | Aggregate::MeanConfidenceNormal { .. }
            | Aggregate::MeanConfidenceBootstrap { .. } => f64::NAN,
            Aggregate::QuantileType7(_) if sorted.is_empty() => f64::NAN,
            Aggregate::QuantileType7(probability) => type_7_quantile(sorted, probability),
        }
    }
}

/// A percentile of `samples` resampled means. Both ends use the same seed, so they
/// read one bootstrap distribution.
fn bootstrap_mean_quantile(sorted: &[f64], probability: f64, samples: usize, seed: i64) -> f64 {
    let mut random = KotlinRandom::new(seed);
    let count = sorted.len();
    let mut means: Vec<f64> = (0..samples)
        .map(|_| {
            let total: f64 = (0..count)
                .map(|_| sorted[random.next_int_until(count as i32) as usize])
                .sum();
            total / count as f64
        })
        .collect();
    means.sort_by(f64::total_cmp);
    type_7_quantile(&means, probability)
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
            return data_function_summary(data_function, parameters);
        }
        if !fun_args(parameters)?.is_none_or(|arguments| arguments.is_empty()) {
            return Err(Error::spec(
                parameters.path_of("fun_args"),
                format!(
                    "fun_args applies to fun_data ({}); pass fun_data too or drop fun_args",
                    DATA_FUNCTION_NAMES.join(", ")
                ),
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

/// The `fun_args` dict, if given.
fn fun_args<'a>(
    parameters: &'a Parameters<'_>,
) -> Result<Option<&'a serde_json::Map<String, Value>>> {
    let Some(value) = parameters
        .value("fun_args")
        .filter(|value| !value.is_null())
    else {
        return Ok(None);
    };
    match value {
        Value::Object(entries) => Ok(Some(entries)),
        _ => Err(Error::spec(
            parameters.path_of("fun_args"),
            format!("expected a dict like {{'mult': 2}}, got {value}"),
        )),
    }
}

/// A `fun_args` number that passes `valid`.
fn fun_arg(
    parameters: &Parameters,
    name: &str,
    expected: &str,
    valid: fn(f64) -> bool,
) -> Result<Option<f64>> {
    let Some(value) = fun_args(parameters)?.and_then(|arguments| arguments.get(name)) else {
        return Ok(None);
    };
    value
        .as_f64()
        .filter(|number| number.is_finite() && valid(*number))
        .map(Some)
        .ok_or_else(|| {
            Error::spec(
                parameters.path_of("fun_args"),
                format!("'{name}' expected {expected}, got {value}"),
            )
        })
}

/// y, ymin and ymax of a `fun_data` shortcut.
fn data_function_summary(name: &str, parameters: &Parameters) -> Result<Summary> {
    let allowed: &[&str] = match name {
        "mean_se" | "mean_sdl" => &["mult"],
        "mean_cl_normal" | "median_hilow" => &["conf_int"],
        "mean_cl_boot" => &["conf_int", "B", "seed"],
        _ => {
            return Err(Error::spec(
                parameters.path_of("fun_data"),
                format!(
                    "unknown summary data function '{name}'; expected one of {}{}",
                    DATA_FUNCTION_NAMES.join(", "),
                    crate::suggest::did_you_mean(name, DATA_FUNCTION_NAMES)
                ),
            ));
        }
    };
    if let Some(key) = fun_args(parameters)?
        .into_iter()
        .flat_map(|arguments| arguments.keys())
        .find(|key| !allowed.contains(&key.as_str()))
    {
        return Err(Error::spec(
            parameters.path_of("fun_args"),
            format!(
                "unknown argument '{key}' for fun_data='{name}'; it takes {}",
                allowed.join(", ")
            ),
        ));
    }
    let mult = fun_arg(parameters, "mult", "a number >= 0", |mult| mult >= 0.0)?;
    let confidence = fun_arg(
        parameters,
        "conf_int",
        "a number between 0 and 1",
        |level| level > 0.0 && level < 1.0,
    )?
    .unwrap_or(DEFAULT_CONFIDENCE);
    let (lower, upper) = ((1.0 - confidence) / 2.0, (1.0 + confidence) / 2.0);
    let spread = |make: fn(f64) -> Aggregate, default_mult: f64| {
        let mult = mult.unwrap_or(default_mult);
        Summary {
            y: Aggregate::Mean,
            ymin: make(-mult),
            ymax: make(mult),
        }
    };
    Ok(match name {
        "mean_se" => spread(Aggregate::MeanStandardError, 1.0),
        "mean_sdl" => spread(Aggregate::MeanStandardDeviation, 2.0),
        "mean_cl_normal" => Summary {
            y: Aggregate::Mean,
            ymin: Aggregate::MeanConfidenceNormal { probability: lower },
            ymax: Aggregate::MeanConfidenceNormal { probability: upper },
        },
        "median_hilow" => Summary {
            y: Aggregate::QuantileType7(0.5),
            ymin: Aggregate::QuantileType7(lower),
            ymax: Aggregate::QuantileType7(upper),
        },
        _ => {
            let whole = |number: f64| number.fract() == 0.0;
            let samples = fun_arg(parameters, "B", "a whole number from 1 to 100000", |n| {
                n.fract() == 0.0 && (1.0..=1e5).contains(&n)
            })?
            .map_or(DEFAULT_BOOTSTRAP_SAMPLES, |samples| samples as usize);
            let seed = fun_arg(parameters, "seed", "a whole number", whole)?
                .map_or(DEFAULT_BOOTSTRAP_SEED, |seed| seed as i64);
            let at = |probability| Aggregate::MeanConfidenceBootstrap {
                probability,
                samples,
                seed,
            };
            Summary {
                y: Aggregate::Mean,
                ymin: at(lower),
                ymax: at(upper),
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_shortcuts_match_hmisc() {
        // R: x <- c(1, 2, 4, 9); Hmisc::smean.cl.normal(x) (mean + qt(.975, 3) * se);
        // quantile(x, c(.5, .025, .975)).
        let sorted = [1.0, 2.0, 4.0, 9.0];
        let normal = |probability| Aggregate::MeanConfidenceNormal { probability }.apply(&sorted);
        assert!((normal(0.025) + 1.6632).abs() < 1e-3, "{}", normal(0.025));
        assert!((normal(0.975) - 9.6632).abs() < 1e-3, "{}", normal(0.975));
        assert!((Aggregate::QuantileType7(0.5).apply(&sorted) - 3.0).abs() < 1e-12);
        assert!((Aggregate::QuantileType7(0.025).apply(&sorted) - 1.075).abs() < 1e-12);
        assert!((Aggregate::QuantileType7(0.975).apply(&sorted) - 8.625).abs() < 1e-12);
        for aggregate in [
            Aggregate::MeanConfidenceNormal { probability: 0.025 },
            Aggregate::MeanConfidenceBootstrap {
                probability: 0.025,
                samples: 10,
                seed: 1,
            },
        ] {
            assert!(aggregate.apply(&[1.0]).is_nan());
        }
    }

    #[test]
    fn bootstrap_is_seeded_and_brackets_the_mean() {
        let sorted: Vec<f64> = (0..50).map(f64::from).collect();
        let at = |probability, seed| {
            Aggregate::MeanConfidenceBootstrap {
                probability,
                samples: 1000,
                seed,
            }
            .apply(&sorted)
        };
        assert_eq!(at(0.025, 3), at(0.025, 3));
        assert_ne!(at(0.025, 3), at(0.025, 4));
        // Standard error about 2.06: the interval is mean +- about 4.
        assert!((at(0.025, 1) - 20.5).abs() < 1.0, "{}", at(0.025, 1));
        assert!((at(0.975, 1) - 28.5).abs() < 1.0, "{}", at(0.975, 1));
    }

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
