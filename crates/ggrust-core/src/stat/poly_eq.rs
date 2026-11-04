//! `stat="poly_eq"`: a least squares polynomial's equation and fit statistics
//! per group, as labels of the R package ggpmisc's `stat_poly_eq`.
//!
//! `formula`: `"y ~ x"` (default), `"y ~ poly(x, k, raw = TRUE)"` or
//! `"y ~ x + I(x^2) + ... + I(x^k)"`. The fit is R's `lm` (Householder QR of the
//! raw polynomial design). `label`: one or a list of `"eq"`
//! (`"y = 37.3 - 5.34x"`, 3 significant digits per coefficient), `"R2"`
//! (`"R² = 0.75"`, 2 decimals, the default as ggpmisc), `"adj.R2"`, `"P"` (the
//! F test, `"P < 0.001"` or 3 decimals) and `"n"`, joined with `", "`
//! (`label_sep`). Digits: `coef_digits` (3), `rr_digits` (2). Places as
//! `stat_cor` (`stat/pubr_labels.rs`): top left of the data range, groups
//! stacked; ggpmisc's npc placement is a recorded deviation.
//!
//! Groups with no more distinct x than the degree, or a constant y, get no label.
//!
//! What it does not do: other model formulas (no intercept, `log(x)`), weights,
//! `lm` methods other than least squares, plotmath output, AIC/BIC labels.

use super::distribution::f_upper;
use super::pubr_labels::{labels_per_group, significant};
use super::{Stat, StatContext};
use crate::data::LayerData;
use crate::parameters::Parameters;
use crate::spec::{Aesthetic, Value};
use crate::{Error, Result};

/// Polynomial equation labels.
pub struct PolyEqStat;

/// The parts a label can show (ggpmisc `use_label` names).
const PARTS: [&str; 5] = ["eq", "R2", "adj.R2", "P", "n"];

impl Stat for PolyEqStat {
    fn name(&self) -> &'static str {
        "poly_eq"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &[
            "formula",
            "label",
            "label_sep",
            "coef_digits",
            "rr_digits",
            "label_x",
            "label_y",
            "label_x_npc",
            "label_y_npc",
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
        let degree = match parameters.string("formula")? {
            None => 1,
            Some(formula) => parse_formula(formula).ok_or_else(|| {
                Error::spec(
                    parameters.path_of("formula"),
                    format!(
                        "unsupported formula '{formula}'; use 'y ~ x', \
                         'y ~ poly(x, 2, raw = TRUE)' or 'y ~ x + I(x^2)'"
                    ),
                )
            })?,
        };
        let parts = label_parts(parameters)?;
        let digits = |name: &str, default: usize| -> Result<usize> {
            match parameters.number(name)? {
                None => Ok(default),
                Some(value) if value >= 1.0 && value.fract() == 0.0 && value <= 15.0 => {
                    Ok(value as usize)
                }
                Some(value) => Err(Error::spec(
                    parameters.path_of(name),
                    format!("expected a whole number of digits from 1 to 15, got {value}"),
                )),
            }
        };
        let (coef_digits, rr_digits) = (digits("coef_digits", 3)?, digits("rr_digits", 2)?);
        let separator = parameters.string("label_sep")?.unwrap_or(", ");
        let (Some(x), Some(y)) = (data.continuous(Aesthetic::X), data.continuous(Aesthetic::Y))
        else {
            return Ok(LayerData::new(0));
        };
        labels_per_group(&data, parameters, context, |rows| {
            let (xs, ys): (Vec<f64>, Vec<f64>) = rows
                .iter()
                .filter_map(|&row| Some((x.get(row)?, y.get(row)?)))
                .filter(|(a, b)| a.is_finite() && b.is_finite())
                .unzip();
            let Some(fit) = PolynomialFit::new(&xs, &ys, degree) else {
                return Ok(None);
            };
            let texts: Vec<String> = parts
                .iter()
                .map(|part| match *part {
                    "eq" => fit.equation(coef_digits),
                    "R2" => format!("R² = {:.*}", rr_digits, fit.r_squared),
                    "adj.R2" => format!("R²adj = {:.*}", rr_digits, fit.adjusted_r_squared()),
                    "P" => match fit.p_value() {
                        p if p < 0.001 => "P < 0.001".to_owned(),
                        p => format!("P = {p:.3}"),
                    },
                    _ => format!("n = {}", fit.n),
                })
                .collect();
            Ok(Some(texts.join(separator)))
        })
    }
}

/// `label` as a list of part names.
fn label_parts(parameters: &Parameters) -> Result<Vec<&'static str>> {
    let names: Vec<&str> = match parameters.value("label") {
        None | Some(Value::Null) => return Ok(vec!["R2"]),
        Some(Value::String(name)) => vec![name.as_str()],
        Some(Value::Array(items)) if !items.is_empty() => items
            .iter()
            .map(|item| {
                item.as_str().ok_or_else(|| {
                    Error::spec(
                        parameters.path_of("label"),
                        format!("expected label part names, got {item}"),
                    )
                })
            })
            .collect::<Result<_>>()?,
        Some(other) => {
            return Err(Error::spec(
                parameters.path_of("label"),
                format!("expected a part name or a list of them, e.g. ['eq', 'R2'], got {other}"),
            ));
        }
    };
    names
        .into_iter()
        .map(|name| {
            PARTS
                .iter()
                .copied()
                .find(|part| *part == name)
                .ok_or_else(|| {
                    Error::spec(
                        parameters.path_of("label"),
                        format!(
                            "unknown label part '{name}'{}; use one of: {}",
                            crate::suggest::did_you_mean(name, PARTS),
                            PARTS.join(", ")
                        ),
                    )
                })
        })
        .collect()
}

/// The polynomial degree of a supported formula.
fn parse_formula(formula: &str) -> Option<usize> {
    let compact: String = formula.chars().filter(|c| !c.is_whitespace()).collect();
    let right = compact.strip_prefix("y~")?;
    if right == "x" {
        return Some(1);
    }
    if let Some(arguments) = right
        .strip_prefix("poly(x,")
        .and_then(|r| r.strip_suffix(')'))
    {
        let (degree, raw) = arguments.split_once(',')?;
        let raw = matches!(raw, "raw=TRUE" | "raw=T" | "TRUE" | "T");
        let degree: usize = degree.parse().ok()?;
        return (raw && degree >= 1).then_some(degree);
    }
    let terms: Vec<&str> = right.split('+').collect();
    if terms.first() != Some(&"x") {
        return None;
    }
    for (index, term) in terms.iter().enumerate().skip(1) {
        if *term != format!("I(x^{})", index + 1) {
            return None;
        }
    }
    Some(terms.len())
}

/// A least squares polynomial fit.
struct PolynomialFit {
    /// Intercept first.
    coefficients: Vec<f64>,
    r_squared: f64,
    n: usize,
}

impl PolynomialFit {
    /// `None` without more distinct x than the degree.
    fn new(xs: &[f64], ys: &[f64], degree: usize) -> Option<Self> {
        let mut distinct = xs.to_vec();
        distinct.sort_by(f64::total_cmp);
        distinct.dedup();
        if distinct.len() <= degree {
            return None;
        }
        let (n, columns) = (xs.len(), degree + 1);
        let mut design: Vec<Vec<f64>> = (0..columns)
            .map(|power| xs.iter().map(|x| x.powi(power as i32)).collect())
            .collect();
        let mut target = ys.to_vec();
        // Householder QR, column by column; the reflections also apply to y.
        for k in 0..columns {
            let norm = design[k][k..].iter().map(|v| v * v).sum::<f64>().sqrt();
            if norm == 0.0 {
                return None;
            }
            let alpha = if design[k][k] > 0.0 { -norm } else { norm };
            let mut v: Vec<f64> = design[k][k..].to_vec();
            v[0] -= alpha;
            let v_norm = v.iter().map(|a| a * a).sum::<f64>();
            if v_norm == 0.0 {
                continue;
            }
            let reflect = |column: &mut [f64]| {
                let dot: f64 = column[k..].iter().zip(&v).map(|(a, b)| a * b).sum();
                let factor = 2.0 * dot / v_norm;
                for (value, vi) in column[k..].iter_mut().zip(&v) {
                    *value -= factor * vi;
                }
            };
            for column in design.iter_mut().skip(k) {
                reflect(column);
            }
            reflect(&mut target);
        }
        let mut coefficients = vec![0.0; columns];
        for k in (0..columns).rev() {
            let sum: f64 = (k + 1..columns)
                .map(|j| design[j][k] * coefficients[j])
                .sum();
            coefficients[k] = (target[k] - sum) / design[k][k];
        }
        let residual: f64 = target[columns..].iter().map(|v| v * v).sum();
        let mean = ys.iter().sum::<f64>() / n as f64;
        let total: f64 = ys.iter().map(|y| (y - mean).powi(2)).sum();
        if total == 0.0 {
            // Constant y: R^2 is undefined (no label rather than "-inf").
            return None;
        }
        Some(PolynomialFit {
            coefficients,
            r_squared: 1.0 - residual / total,
            n,
        })
    }

    fn degree(&self) -> usize {
        self.coefficients.len() - 1
    }

    fn adjusted_r_squared(&self) -> f64 {
        let (n, k) = (self.n as f64, self.degree() as f64);
        1.0 - (1.0 - self.r_squared) * (n - 1.0) / (n - k - 1.0)
    }

    /// The overall F test's p-value.
    fn p_value(&self) -> f64 {
        let (n, k) = (self.n as f64, self.degree() as f64);
        let f = (self.r_squared / k) / ((1.0 - self.r_squared) / (n - k - 1.0));
        f_upper(f, k, n - k - 1.0)
    }

    /// `y = 37.3 - 5.34x + 1.17x²`.
    fn equation(&self, digits: usize) -> String {
        let mut text = format!("y = {}", significant(self.coefficients[0], digits));
        for (power, coefficient) in self.coefficients.iter().enumerate().skip(1) {
            let sign = if *coefficient < 0.0 { '-' } else { '+' };
            text.push_str(&format!(
                " {sign} {}x{}",
                significant(coefficient.abs(), digits),
                superscript(power)
            ));
        }
        text
    }
}

/// A power as superscript digits (`²`); powers of 10 and above as `^10` (the
/// embedded font has no superscript 0).
fn superscript(power: usize) -> String {
    const DIGITS: [&str; 10] = ["", "¹", "²", "³", "⁴", "⁵", "⁶", "⁷", "⁸", "⁹"];
    match power {
        1 => String::new(),
        2..=9 => DIGITS[power].to_owned(),
        _ => format!("^{power}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formulas() {
        assert_eq!(parse_formula("y ~ x"), Some(1));
        assert_eq!(parse_formula("y ~ poly(x, 3, raw = TRUE)"), Some(3));
        assert_eq!(parse_formula("y ~ x + I(x^2)"), Some(2));
        assert_eq!(parse_formula("y ~ poly(x, 2)"), None);
        assert_eq!(parse_formula("y ~ x + I(x^3)"), None);
        assert_eq!(parse_formula("log(y) ~ x"), None);
    }

    #[test]
    fn fits_match_r() {
        let reference: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/pubr_reference.json"
        ))
        .unwrap();
        let numbers = |value: &serde_json::Value| -> Vec<f64> {
            value
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect()
        };
        let close = |a: f64, b: f64| (a - b).abs() <= 1e-6 * b.abs() + 1e-12;
        for case in reference["polynomials"].as_array().unwrap() {
            let degree = case["degree"].as_u64().unwrap() as usize;
            let fit =
                PolynomialFit::new(&numbers(&case["x"]), &numbers(&case["y"]), degree).unwrap();
            for (a, b) in fit.coefficients.iter().zip(numbers(&case["coefficients"])) {
                assert!(close(*a, b), "degree {degree}: {a} vs R {b}");
            }
            assert!(close(fit.r_squared, case["r_squared"].as_f64().unwrap()));
            assert!(close(
                fit.adjusted_r_squared(),
                case["adjusted_r_squared"].as_f64().unwrap()
            ));
            assert!(
                close(fit.p_value(), case["p"].as_f64().unwrap()),
                "p {degree}"
            );
        }
    }

    #[test]
    fn equation_text() {
        let fit = PolynomialFit {
            coefficients: vec![49.93, -13.38, 1.171],
            r_squared: 0.8191,
            n: 32,
        };
        assert_eq!(fit.equation(3), "y = 49.9 - 13.4x + 1.17x²");
    }
}
