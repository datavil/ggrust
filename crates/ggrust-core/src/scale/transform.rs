//! Continuous scale transforms (lets-plot `Transforms`): data is transformed when
//! it is mapped, so stats, domains and expansion work in transformed units.

use crate::{Error, Result};

/// lets-plot `Transform.MIN_POSITIVE_DOUBLE`: the smallest value a log accepts.
const MIN_POSITIVE: f64 = 4.9e-324 * 1e5;
/// lets-plot `Transform.MAX_DOUBLE`.
const MAX_VALUE: f64 = f64::MAX / 1e5;

/// A continuous transform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
}

impl Transform {
    const ALL: [(Transform, &'static str); 5] = [
        (Transform::Identity, "identity"),
        (Transform::Reverse, "reverse"),
        (Transform::Sqrt, "sqrt"),
        (Transform::Log10, "log10"),
        (Transform::Log2, "log2"),
    ];

    /// Parse a lets-plot `trans` name; the error lists the supported names.
    pub fn parse(name: &str, path: &str) -> Result<Self> {
        Transform::ALL
            .iter()
            .find(|(_, candidate)| *candidate == name)
            .map(|(transform, _)| *transform)
            .ok_or_else(|| {
                let names: Vec<&str> = Transform::ALL.iter().map(|(_, name)| *name).collect();
                Error::InvalidSpec(format!(
                    "at '{path}': unknown transform '{name}'; supported: {}",
                    names.join(", ")
                ))
            })
    }

    /// lets-plot name.
    pub fn as_str(self) -> &'static str {
        Transform::ALL
            .iter()
            .find(|(transform, _)| *transform == self)
            .map_or("", |(_, name)| name)
    }

    /// Whether breaks are evenly spaced in data units (identity, reverse).
    pub fn is_linear(self) -> bool {
        matches!(self, Transform::Identity | Transform::Reverse)
    }

    /// Whether `value` can be transformed (finite and in the transform's domain).
    pub fn is_in_domain(self, value: f64) -> bool {
        value.is_finite()
            && match self {
                Transform::Identity | Transform::Reverse => true,
                Transform::Sqrt => value >= 0.0,
                Transform::Log10 | Transform::Log2 => (MIN_POSITIVE..=MAX_VALUE).contains(&value),
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
        let error = Transform::parse("log", "scales[0].trans")
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("at 'scales[0].trans': unknown transform 'log'; supported: identity"),
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
}
