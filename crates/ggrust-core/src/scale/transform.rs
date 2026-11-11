//! Continuous scale transforms (lets-plot `Transforms`): data is transformed when
//! it is mapped, so stats, domains and expansion work in transformed units.
//! `symlog` is lets-plot's; `log`, `log1p`, `exp`, `logit`, `probit`, `reciprocal`,
//! `pseudo_log` and `boxcox(p)` follow ggplot2's `scales` package.

use crate::stat::distribution::{normal_lower, normal_quantile};
use crate::{Error, Result};

/// lets-plot `Transform.MIN_POSITIVE_DOUBLE`: the smallest value a log accepts.
const MIN_POSITIVE: f64 = 4.9e-324 * 1e5;
/// lets-plot `Transform.MAX_DOUBLE`.
const MAX_VALUE: f64 = f64::MAX / 1e5;

/// A continuous transform.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Transform {
    /// No change.
    #[default]
    Identity,
    /// `-x`: the axis runs the other way.
    Reverse,
    /// Square root (values >= 0).
    Sqrt,
    /// Base-10 logarithm (values > 0).
    Log10,
    /// Base-2 logarithm (values > 0).
    Log2,
    /// Natural logarithm (values > 0).
    Log,
    /// `ln(1 + x)` (values > -1).
    Log1p,
    /// `e^x`.
    Exp,
    /// Log odds (values strictly between 0 and 1).
    Logit,
    /// Normal quantile (values strictly between 0 and 1).
    Probit,
    /// `1 / x` (values > 0): the axis runs the other way.
    Reciprocal,
    /// `asinh(x / 2)`: linear around 0, logarithmic far from it.
    PseudoLog,
    /// lets-plot's symmetric log: linear inside -1..1, `1 + log10` outside.
    Symlog,
    /// Box-Cox power transform of `x + offset`.
    BoxCox {
        /// The power; 0 is a logarithm.
        power: f64,
        /// Added to the data before the power.
        offset: f64,
    },
}

/// `scales::boxcox_trans`: below this power the transform is a logarithm.
const BOX_COX_LOG_POWER: f64 = 1e-7;

impl Transform {
    const ALL: [(Transform, &'static str); 13] = [
        (Transform::Identity, "identity"),
        (Transform::Reverse, "reverse"),
        (Transform::Sqrt, "sqrt"),
        (Transform::Log10, "log10"),
        (Transform::Log2, "log2"),
        (Transform::Log, "log"),
        (Transform::Log1p, "log1p"),
        (Transform::Exp, "exp"),
        (Transform::Logit, "logit"),
        (Transform::Probit, "probit"),
        (Transform::Reciprocal, "reciprocal"),
        (Transform::PseudoLog, "pseudo_log"),
        (Transform::Symlog, "symlog"),
    ];

    /// Parse a lets-plot `trans` name; the error lists the supported names.
    pub fn parse(name: &str, path: &str) -> Result<Self> {
        if name == "boxcox" || name.starts_with("boxcox(") {
            return Transform::parse_box_cox(name, path);
        }
        Transform::ALL
            .iter()
            .find(|(_, candidate)| *candidate == name)
            .map(|(transform, _)| *transform)
            .ok_or_else(|| {
                let mut names: Vec<&str> = Transform::ALL.iter().map(|(_, name)| *name).collect();
                names.push("boxcox(p)");
                Error::spec(
                    path,
                    format!(
                        "unknown transform '{name}'; supported: {}{}",
                        names.join(", "),
                        crate::suggest::did_you_mean(name, names.iter().copied())
                    ),
                )
            })
    }

    /// `boxcox(p)` or `boxcox(p, offset)`: the power is required, as in `scales`.
    fn parse_box_cox(name: &str, path: &str) -> Result<Self> {
        let invalid = || {
            Error::spec(
                path,
                format!(
                    "invalid transform '{name}'; boxcox needs a power and takes an optional \
                     offset, both finite numbers: use boxcox_trans(0.5) or \
                     boxcox_trans(0.5, offset=1)"
                ),
            )
        };
        let arguments = name
            .strip_prefix("boxcox(")
            .and_then(|rest| rest.strip_suffix(')'))
            .ok_or_else(invalid)?;
        let numbers = arguments
            .split(',')
            .map(|number| number.trim().parse::<f64>().ok().filter(|n| n.is_finite()))
            .collect::<Option<Vec<f64>>>()
            .ok_or_else(invalid)?;
        match numbers[..] {
            [power] => Ok(Transform::BoxCox { power, offset: 0.0 }),
            [power, offset] => Ok(Transform::BoxCox { power, offset }),
            _ => Err(invalid()),
        }
    }

    /// The name in the spec, with Box-Cox parameters (the zoom runtime parses it).
    pub fn spec_name(self) -> String {
        match self {
            Transform::BoxCox { power, offset } => format!("boxcox({power},{offset})"),
            other => other.as_str().to_string(),
        }
    }

    /// Name without parameters.
    pub fn as_str(self) -> &'static str {
        if matches!(self, Transform::BoxCox { .. }) {
            return "boxcox";
        }
        Transform::ALL
            .iter()
            .find(|(transform, _)| *transform == self)
            .map_or("", |(_, name)| name)
    }

    /// Whether breaks are evenly spaced in data units (identity, reverse).
    pub fn is_linear(self) -> bool {
        matches!(self, Transform::Identity | Transform::Reverse)
    }

    /// Whether larger data values get smaller transformed values.
    pub fn is_decreasing(self) -> bool {
        matches!(self, Transform::Reverse | Transform::Reciprocal)
    }

    /// Whether `value` can be transformed (finite and in the transform's domain).
    pub fn is_in_domain(self, value: f64) -> bool {
        value.is_finite()
            && match self {
                Transform::Identity
                | Transform::Reverse
                | Transform::PseudoLog
                | Transform::Symlog => true,
                Transform::Sqrt => value >= 0.0,
                Transform::Log10 | Transform::Log2 | Transform::Log => {
                    (MIN_POSITIVE..=MAX_VALUE).contains(&value)
                }
                Transform::Log1p => value > -1.0,
                Transform::Exp => value.exp() <= MAX_VALUE,
                Transform::Logit | Transform::Probit => value > 0.0 && value < 1.0,
                Transform::Reciprocal => value > 0.0,
                Transform::BoxCox { power, offset } => {
                    let shifted = value + offset;
                    shifted > 0.0 || (shifted == 0.0 && power >= BOX_COX_LOG_POWER)
                }
            }
    }

    /// Transform a value; `None` outside the domain (lets-plot drops those values).
    pub fn apply(self, value: f64) -> Option<f64> {
        self.is_in_domain(value)
            .then(|| self.apply_unchecked(value))
    }

    /// lets-plot's `apply` without the domain check: NaN for negative values under
    /// sqrt and log; logs of 0 and huge values are clamped to the log limits.
    pub fn apply_unchecked(self, value: f64) -> f64 {
        // Kotlin's log(x, base) is ln(x) / ln(base); keep its rounding.
        let log = |base: f64| {
            let transformed = value.ln() / base.ln();
            if transformed.is_nan() {
                return transformed;
            }
            let low = MIN_POSITIVE.ln() / base.ln();
            let high = MAX_VALUE.ln() / base.ln();
            transformed.clamp(low, high)
        };
        match self {
            Transform::Identity => value,
            Transform::Reverse => -value,
            Transform::Sqrt => value.sqrt(),
            Transform::Log10 => log(10.0),
            Transform::Log2 => log(2.0),
            Transform::Log => log(std::f64::consts::E),
            other => {
                // Outside the domain (or at an infinite end) is NaN, never an infinity.
                let transformed = if other.is_in_domain(value) {
                    other.apply_extended(value)
                } else {
                    f64::NAN
                };
                if transformed.is_finite() {
                    transformed
                } else {
                    f64::NAN
                }
            }
        }
    }

    /// The transforms that are not lets-plot's clamped ones, for values in the domain.
    fn apply_extended(self, value: f64) -> f64 {
        match self {
            Transform::Log1p => value.ln_1p(),
            Transform::Exp => value.exp(),
            Transform::Logit => (value / (1.0 - value)).ln(),
            Transform::Probit => normal_quantile(value),
            Transform::Reciprocal => 1.0 / value,
            Transform::PseudoLog => (value / 2.0).asinh(),
            // lets-plot `SymlogTransform`.
            Transform::Symlog if value.abs() < 1.0 => value,
            Transform::Symlog => value.signum() * (1.0 + value.abs().ln() / 10_f64.ln()),
            Transform::BoxCox { power, offset } if power.abs() < BOX_COX_LOG_POWER => {
                (value + offset).ln()
            }
            Transform::BoxCox { power, offset } => ((value + offset).powf(power) - 1.0) / power,
            _ => value,
        }
    }

    /// Back to data units.
    pub fn inverse(self, value: f64) -> f64 {
        let log_limits = |base: f64| {
            let low = MIN_POSITIVE.ln() / base.ln();
            let high = MAX_VALUE.ln() / base.ln();
            base.powf(value.clamp(low, high))
        };
        match self {
            Transform::Identity => value,
            Transform::Reverse => -value,
            Transform::Sqrt => value * value,
            Transform::Log10 => log_limits(10.0),
            Transform::Log2 => log_limits(2.0),
            Transform::Log => log_limits(std::f64::consts::E),
            Transform::Log1p => value.exp_m1(),
            // A zero-based domain reaches 0, which no exponent gives.
            Transform::Exp => value.max(MIN_POSITIVE).ln(),
            Transform::Logit => 1.0 / (1.0 + (-value).exp()),
            Transform::Probit => normal_lower(value),
            Transform::Reciprocal => 1.0 / value,
            Transform::PseudoLog => 2.0 * value.sinh(),
            Transform::Symlog if value.abs() < 1.0 => value,
            Transform::Symlog => value.signum() * 10_f64.powf(value.abs() - 1.0),
            Transform::BoxCox { power, offset } if power.abs() < BOX_COX_LOG_POWER => {
                value.exp() - offset
            }
            Transform::BoxCox { power, offset } => (value * power + 1.0).powf(1.0 / power) - offset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip_and_errors_list_them() {
        for (transform, name) in Transform::ALL {
            assert_eq!(
                Transform::parse(name, "scales[0].trans").unwrap(),
                transform
            );
            assert_eq!(transform.as_str(), name);
        }
        let error = Transform::parse("lg", "scales[0].trans")
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("at 'scales[0].trans': unknown transform 'lg'; supported: identity"),
            "{error}"
        );
    }

    #[test]
    fn domains_and_round_trips() {
        assert_eq!(Transform::Log10.apply(0.0), None);
        assert_eq!(Transform::Log2.apply(-1.0), None);
        assert_eq!(Transform::Sqrt.apply(-0.1), None);
        assert_eq!(Transform::Sqrt.apply(0.0), Some(0.0));
        assert_eq!(Transform::Identity.apply(f64::NAN), None);
        assert_eq!(Transform::Reverse.apply(3.0), Some(-3.0));
        for transform in [Transform::Sqrt, Transform::Log10, Transform::Log2] {
            for value in [0.5, 1.0, 8.0, 1000.0] {
                let back = transform.inverse(transform.apply(value).unwrap());
                assert!(
                    (back - value).abs() <= value * 1e-12,
                    "{transform:?} {value}"
                );
            }
        }
        assert!(Transform::Log10.inverse(1e9).is_finite());
    }

    #[test]
    fn extended_transforms_match_r() {
        // Values from R: log(5), log1p(5), exp(2), qlogis(.2), qnorm(.2), 1/5,
        // asinh(5/2), scales::boxcox_trans(.5, 1)$transform(5).
        let box_cox = Transform::BoxCox {
            power: 0.5,
            offset: 1.0,
        };
        for (transform, value, expected) in [
            (Transform::Log, 5.0, 1.6094379124341003),
            (Transform::Log1p, 5.0, 1.791759469228055),
            (Transform::Exp, 2.0, 7.38905609893065),
            (Transform::Logit, 0.2, -1.3862943611198906),
            (Transform::Probit, 0.2, -0.8416212335729143),
            (Transform::Reciprocal, 5.0, 0.2),
            (Transform::PseudoLog, 5.0, 1.6472311463710958),
            (box_cox, 5.0, 2.8989794855663558),
            (Transform::Symlog, 100.0, 3.0),
            (Transform::Symlog, -100.0, -3.0),
            (Transform::Symlog, 0.5, 0.5),
        ] {
            let transformed = transform.apply(value).unwrap();
            assert!(
                (transformed - expected).abs() < 1e-12,
                "{transform:?} {value}: {transformed}"
            );
            let back = transform.inverse(transformed);
            assert!((back - value).abs() < 1e-9, "{transform:?} {value}: {back}");
        }
    }

    #[test]
    fn extended_domains_are_nan_outside() {
        let box_cox = |power| Transform::BoxCox { power, offset: 0.0 };
        for (transform, outside) in [
            (Transform::Log, 0.0),
            (Transform::Log1p, -1.0),
            (Transform::Exp, 1000.0),
            (Transform::Logit, 0.0),
            (Transform::Logit, 1.0),
            (Transform::Probit, 1.0),
            (Transform::Reciprocal, 0.0),
            (Transform::Reciprocal, -2.0),
            (box_cox(0.5), -0.1),
            (box_cox(0.0), 0.0),
            (box_cox(-1.0), 0.0),
        ] {
            assert_eq!(transform.apply(outside), None, "{transform:?} {outside}");
        }
        assert_eq!(box_cox(0.5).apply(0.0), Some(-2.0));
        for (transform, outside) in [
            (Transform::Logit, 0.0),
            (Transform::Probit, -1.0),
            (Transform::Reciprocal, -1.0),
            (Transform::Log1p, -2.0),
        ] {
            assert!(transform.apply_unchecked(outside).is_nan(), "{transform:?}");
        }
        assert!(Transform::Reciprocal.is_decreasing());
        assert!(!Transform::Log.is_decreasing());
    }

    #[test]
    fn box_cox_parses_parameters_and_requires_a_power() {
        let parsed = Transform::parse("boxcox(0.5, 1)", "trans").unwrap();
        assert_eq!(
            parsed,
            Transform::BoxCox {
                power: 0.5,
                offset: 1.0
            }
        );
        assert_eq!(
            Transform::parse(&parsed.spec_name(), "trans").unwrap(),
            parsed
        );
        for bad in [
            "boxcox",
            "boxcox()",
            "boxcox(a)",
            "boxcox(1,2,3)",
            "boxcox(inf)",
        ] {
            let error = Transform::parse(bad, "trans").unwrap_err().to_string();
            assert!(error.contains("boxcox_trans(0.5)"), "{error}");
        }
    }
}
