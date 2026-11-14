//! Easing (gganimate `ease_aes()`, tweenr's easing functions): how the progress
//! of a transition, 0 to 1, maps to the share of the way a value has moved.
//! The formulas are tweenr's `easing.c` (AHEasing, WTFPL). [`Easing`] holds the
//! default ease and the per-aesthetic ones of `animation.ease`. It does not
//! interpolate values ([`super::tween`]).

use std::collections::BTreeMap;
use std::f64::consts::{FRAC_PI_2, PI};

use serde_json::Value;

use super::spec::{PATH, check_keys, shown};
use crate::data::Computed;
use crate::spec::Aesthetic;
use crate::suggest::did_you_mean;
use crate::{Error, Result};

/// One easing function by gganimate name (`"cubic-in-out"`).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Ease {
    name: &'static str,
    function: Curve,
}

impl PartialEq for Ease {
    fn eq(&self, other: &Ease) -> bool {
        self.name == other.name
    }
}

/// An easing curve: progress 0..1 to the share of the way.
type Curve = fn(f64) -> f64;

/// Every ease name tweenr knows (`validEase`), with its function.
const EASES: &[(&str, Curve)] = &[
    ("linear", |p| p),
    ("quadratic-in", |p| p * p),
    ("quadratic-out", |p| -(p * (p - 2.0))),
    ("quadratic-in-out", |p| {
        if p < 0.5 {
            2.0 * p * p
        } else {
            (-2.0 * p * p) + (4.0 * p) - 1.0
        }
    }),
    ("cubic-in", |p| p * p * p),
    ("cubic-out", |p| {
        let f = p - 1.0;
        f * f * f + 1.0
    }),
    ("cubic-in-out", |p| {
        if p < 0.5 {
            4.0 * p * p * p
        } else {
            let f = (2.0 * p) - 2.0;
            0.5 * f * f * f + 1.0
        }
    }),
    ("quartic-in", |p| p * p * p * p),
    ("quartic-out", |p| {
        let f = p - 1.0;
        f * f * f * (1.0 - p) + 1.0
    }),
    ("quartic-in-out", |p| {
        if p < 0.5 {
            8.0 * p * p * p * p
        } else {
            let f = p - 1.0;
            -8.0 * f * f * f * f + 1.0
        }
    }),
    ("quintic-in", |p| p * p * p * p * p),
    ("quintic-out", |p| {
        let f = p - 1.0;
        f * f * f * f * f + 1.0
    }),
    ("quintic-in-out", |p| {
        if p < 0.5 {
            16.0 * p * p * p * p * p
        } else {
            let f = (2.0 * p) - 2.0;
            0.5 * f * f * f * f * f + 1.0
        }
    }),
    ("sine-in", |p| ((p - 1.0) * FRAC_PI_2).sin() + 1.0),
    ("sine-out", |p| (p * FRAC_PI_2).sin()),
    ("sine-in-out", |p| 0.5 * (1.0 - (p * PI).cos())),
    ("circular-in", |p| 1.0 - (1.0 - (p * p)).sqrt()),
    ("circular-out", |p| ((2.0 - p) * p).sqrt()),
    ("circular-in-out", |p| {
        if p < 0.5 {
            0.5 * (1.0 - (1.0 - 4.0 * (p * p)).sqrt())
        } else {
            0.5 * ((-((2.0 * p) - 3.0) * ((2.0 * p) - 1.0)).sqrt() + 1.0)
        }
    }),
    ("exponential-in", |p| {
        if p == 0.0 {
            p
        } else {
            2f64.powf(10.0 * (p - 1.0))
        }
    }),
    ("exponential-out", |p| {
        if p == 1.0 {
            p
        } else {
            1.0 - 2f64.powf(-10.0 * p)
        }
    }),
    ("exponential-in-out", |p| {
        if p == 0.0 || p == 1.0 {
            p
        } else if p < 0.5 {
            0.5 * 2f64.powf((20.0 * p) - 10.0)
        } else {
            -0.5 * 2f64.powf((-20.0 * p) + 10.0) + 1.0
        }
    }),
    ("elastic-in", |p| {
        (13.0 * FRAC_PI_2 * p).sin() * 2f64.powf(10.0 * (p - 1.0))
    }),
    ("elastic-out", |p| {
        (-13.0 * FRAC_PI_2 * (p + 1.0)).sin() * 2f64.powf(-10.0 * p) + 1.0
    }),
    ("elastic-in-out", |p| {
        if p < 0.5 {
            0.5 * (13.0 * FRAC_PI_2 * (2.0 * p)).sin() * 2f64.powf(10.0 * ((2.0 * p) - 1.0))
        } else {
            0.5 * ((-13.0 * FRAC_PI_2 * ((2.0 * p - 1.0) + 1.0)).sin()
                * 2f64.powf(-10.0 * (2.0 * p - 1.0))
                + 2.0)
        }
    }),
    ("back-in", |p| p * p * p - p * (p * PI).sin()),
    ("back-out", |p| {
        let f = 1.0 - p;
        1.0 - (f * f * f - f * (f * PI).sin())
    }),
    ("back-in-out", |p| {
        if p < 0.5 {
            let f = 2.0 * p;
            0.5 * (f * f * f - f * (f * PI).sin())
        } else {
            let f = 1.0 - (2.0 * p - 1.0);
            0.5 * (1.0 - (f * f * f - f * (f * PI).sin())) + 0.5
        }
    }),
    ("bounce-in", |p| 1.0 - bounce_out(1.0 - p)),
    ("bounce-out", bounce_out),
    ("bounce-in-out", |p| {
        if p < 0.5 {
            0.5 * (1.0 - bounce_out(1.0 - p * 2.0))
        } else {
            0.5 * bounce_out(p * 2.0 - 1.0) + 0.5
        }
    }),
];

fn bounce_out(p: f64) -> f64 {
    if p < 4.0 / 11.0 {
        (121.0 * p * p) / 16.0
    } else if p < 8.0 / 11.0 {
        (363.0 / 40.0 * p * p) - (99.0 / 10.0 * p) + 17.0 / 5.0
    } else if p < 9.0 / 10.0 {
        (4356.0 / 361.0 * p * p) - (35442.0 / 1805.0 * p) + 16061.0 / 1805.0
    } else {
        (54.0 / 5.0 * p * p) - (513.0 / 25.0 * p) + 268.0 / 25.0
    }
}

impl Ease {
    /// `linear`.
    pub(crate) const LINEAR: Ease = Ease {
        name: "linear",
        function: |p| p,
    };

    /// The ease named `name`.
    ///
    /// # Errors
    ///
    /// A spec error at `path` for an unknown name, with a close match.
    pub(crate) fn parse(name: &str, path: &str) -> Result<Ease> {
        match EASES.iter().find(|(known, _)| *known == name) {
            Some((name, function)) => Ok(Ease {
                name,
                function: *function,
            }),
            None => Err(Error::spec(
                path,
                format!(
                    "unknown ease '{name}'{}; accepted: linear, or quadratic, cubic, quartic, \
                     quintic, sine, circular, exponential, elastic, back or bounce followed by \
                     -in, -out or -in-out (e.g. 'cubic-in-out')",
                    did_you_mean(name, EASES.iter().map(|(known, _)| *known))
                ),
            )),
        }
    }

    /// The eased share of the way at progress `p` (0 to 1). Some eases leave
    /// 0..1 on the way (elastic, back).
    pub(crate) fn apply(self, p: f64) -> f64 {
        (self.function)(p)
    }
}

/// gganimate's x-like and y-like aesthetics: `ease_aes(x=...)` sets those not
/// named themselves (ggplot2 `x_aes`, `y_aes`).
const X_AESTHETICS: &[&str] = &[
    "x",
    "xmin",
    "xmax",
    "xend",
    "xintercept",
    "xmin_final",
    "xmax_final",
    "xlower",
    "xmiddle",
    "xupper",
    "x0",
];
const Y_AESTHETICS: &[&str] = &[
    "y",
    "ymin",
    "ymax",
    "yend",
    "yintercept",
    "ymin_final",
    "ymax_final",
    "lower",
    "middle",
    "upper",
    "y0",
];

/// `ease_aes(default, **aesthetics)`: the ease of every interpolated column.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Easing {
    default: Ease,
    /// By ggrust column name (`"y"`, `"color"`, `"count"`).
    by_name: BTreeMap<String, Ease>,
}

impl Default for Easing {
    fn default() -> Self {
        Easing {
            default: Ease::LINEAR,
            by_name: BTreeMap::new(),
        }
    }
}

const EASE_KEYS: &[&str] = &["default", "aesthetics"];

impl Easing {
    /// Parse `animation.ease`: `{"default": "linear", "aesthetics": {"y": "bounce-out"}}`.
    ///
    /// # Errors
    ///
    /// A spec error at `animation.ease.*` for an unknown key, aesthetic or ease.
    pub(crate) fn parse(value: Option<&Value>) -> Result<Easing> {
        let path = format!("{PATH}.ease");
        let object = match value {
            None | Some(Value::Null) => return Ok(Easing::default()),
            Some(Value::Object(object)) => object,
            other => {
                return Err(Error::spec(
                    path,
                    format!("expected an object, got {}", shown(other)),
                ));
            }
        };
        check_keys(object, &path, EASE_KEYS)?;
        let name = |value: Option<&Value>, at: &str| -> Result<Option<Ease>> {
            match value {
                None | Some(Value::Null) => Ok(None),
                Some(Value::String(name)) => Ease::parse(name, at).map(Some),
                other => Err(Error::spec(
                    at,
                    format!("expected an ease name as a string, got {}", shown(other)),
                )),
            }
        };
        let default =
            name(object.get("default"), &format!("{path}.default"))?.unwrap_or(Ease::LINEAR);
        let mut given: Vec<(String, Ease)> = Vec::new();
        match object.get("aesthetics") {
            None | Some(Value::Null) => {}
            Some(Value::Object(aesthetics)) => {
                for (key, value) in aesthetics {
                    let at = format!("{path}.aesthetics.{key}");
                    let column = column_name(key, &at)?;
                    if let Some(ease) = name(Some(value), &at)? {
                        given.push((column, ease));
                    }
                }
            }
            other => {
                return Err(Error::spec(
                    format!("{path}.aesthetics"),
                    format!("expected an object, got {}", shown(other)),
                ));
            }
        }
        let mut by_name: BTreeMap<String, Ease> = given.iter().cloned().collect();
        // gganimate: `x` (`y`) eases the x-like (y-like) aesthetics not named.
        for (axis, family) in [("x", X_AESTHETICS), ("y", Y_AESTHETICS)] {
            if let Some(ease) = by_name.get(axis).copied() {
                for name in family {
                    by_name.entry((*name).to_owned()).or_insert(ease);
                }
            }
        }
        Ok(Easing { default, by_name })
    }

    /// The ease of the column or style named `name` (ggrust spelling).
    pub(crate) fn ease(&self, name: &str) -> Ease {
        self.by_name.get(name).copied().unwrap_or(self.default)
    }

    /// Parse helper for [`Map`] values (tests).
    #[cfg(test)]
    pub(crate) fn from_map(object: &serde_json::Map<String, Value>) -> Result<Easing> {
        Easing::parse(Some(&Value::Object(object.clone())))
    }
}

/// The ggrust column name of an `ease_aes()` keyword: an aesthetic (aliases
/// such as `colour` normalised) or a stat variable (`count`).
fn column_name(key: &str, path: &str) -> Result<String> {
    if let Ok(aesthetic) = key.parse::<Aesthetic>() {
        return Ok(aesthetic.as_str().to_owned());
    }
    if let Some(computed) =
        Computed::from_mapping(key).or_else(|| Computed::from_mapping(&format!("..{key}..")))
    {
        return Ok(computed.as_str().to_owned());
    }
    if X_AESTHETICS.contains(&key) || Y_AESTHETICS.contains(&key) {
        return Ok(key.to_owned());
    }
    let known = Aesthetic::ALL.iter().map(|aesthetic| aesthetic.as_str());
    Err(Error::spec(
        path,
        format!(
            "unknown aesthetic '{key}' in ease_aes(){}; name an aesthetic such as x, y, \
             color, fill, size or alpha, or a stat variable such as count",
            did_you_mean(key, known)
        ),
    ))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn ease(name: &str) -> Ease {
        Ease::parse(name, "animation.ease.default").unwrap()
    }

    #[test]
    fn every_ease_starts_at_0_and_ends_at_1() {
        for (name, _) in EASES {
            let ease = ease(name);
            assert!(ease.apply(0.0).abs() < 1e-9, "{name} at 0");
            assert!((ease.apply(1.0) - 1.0).abs() < 1e-9, "{name} at 1");
        }
        assert_eq!(EASES.len(), 31);
    }

    #[test]
    fn values_at_known_progress_match_tweenr() {
        // Hand-evaluated from tweenr's easing.c formulas.
        let cases: &[(&str, f64, f64)] = &[
            ("linear", 0.3, 0.3),
            ("quadratic-in", 0.5, 0.25),
            ("quadratic-out", 0.5, 0.75),
            ("cubic-in", 0.5, 0.125),
            ("cubic-out", 0.5, 0.875),
            ("cubic-in-out", 0.25, 0.0625),
            ("cubic-in-out", 0.75, 0.9375),
            ("quartic-in", 0.5, 0.0625),
            ("quartic-out", 0.5, 0.9375),
            ("quintic-in", 0.5, 0.03125),
            ("quintic-in-out", 0.25, 0.015625),
            ("sine-in", 0.5, 1.0 - std::f64::consts::FRAC_1_SQRT_2),
            ("sine-out", 0.5, std::f64::consts::FRAC_1_SQRT_2),
            ("sine-in-out", 0.5, 0.5),
            ("circular-in", 0.6, 0.2),
            ("circular-out", 0.4, 0.8),
            ("exponential-in", 0.5, 0.03125),
            ("exponential-out", 0.5, 0.96875),
            ("exponential-in-out", 0.25, 0.015625),
            ("back-in", 0.5, -0.375),
            ("back-out", 0.5, 1.375),
            ("bounce-out", 0.5, 0.71875),
            ("bounce-in", 0.5, 0.28125),
            ("bounce-in-out", 0.25, 0.140625),
            ("elastic-in", 0.5, (13.0 * FRAC_PI_2 * 0.5).sin() / 32.0),
            (
                "elastic-out",
                0.5,
                1.0 + (-13.0 * FRAC_PI_2 * 1.5).sin() / 32.0,
            ),
        ];
        for (name, p, expected) in cases {
            let value = ease(name).apply(*p);
            assert!(
                (value - expected).abs() < 1e-12,
                "{name}({p}) = {value}, expected {expected}"
            );
        }
        for name in [
            "quadratic",
            "cubic",
            "quartic",
            "quintic",
            "sine",
            "circular",
        ] {
            let in_out = ease(&format!("{name}-in-out"));
            assert!((in_out.apply(0.5) - 0.5).abs() < 1e-12, "{name}-in-out");
        }
    }

    #[test]
    fn unknown_names_are_spec_errors_with_a_close_match() {
        let error = Ease::parse("cubic-inout", "animation.ease.default")
            .unwrap_err()
            .to_string();
        assert!(error.contains("animation.ease.default"), "{error}");
        assert!(error.contains("did you mean 'cubic-in-out'"), "{error}");
        let error = Ease::parse("cubic", "p").unwrap_err().to_string();
        assert!(error.contains("-in-out"), "{error}");
    }

    #[test]
    fn per_aesthetic_eases_and_x_y_families() {
        let object = json!({"default": "cubic-in", "aesthetics": {"y": "bounce-out", "ymax": "linear", "colour": "sine-in"}});
        let easing = Easing::from_map(object.as_object().unwrap()).unwrap();
        assert_eq!(easing.ease("y"), ease("bounce-out"));
        assert_eq!(easing.ease("ymin"), ease("bounce-out"));
        assert_eq!(easing.ease("ymax"), ease("linear"));
        assert_eq!(easing.ease("color"), ease("sine-in"));
        assert_eq!(easing.ease("x"), ease("cubic-in"));
        let object = json!({"aesthetics": {"colr": "linear"}});
        let error = Easing::from_map(object.as_object().unwrap())
            .unwrap_err()
            .to_string();
        assert!(error.contains("animation.ease.aesthetics.colr"), "{error}");
        assert!(error.contains("did you mean 'color'"), "{error}");
        let object = json!({"aesthetics": {"y": 3}});
        assert!(Easing::from_map(object.as_object().unwrap()).is_err());
        assert_eq!(Easing::parse(None).unwrap(), Easing::default());
    }
}
