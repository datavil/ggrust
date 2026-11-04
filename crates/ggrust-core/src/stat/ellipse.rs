//! `stat="ellipse"` (ggplot2 `stat_ellipse`): a confidence ellipse per group.
//!
//! Per group of finite (x, y) points, a location and a shape matrix are fitted,
//! then a circle of `segments + 1` points (first = last) is stretched by the
//! shape's Cholesky factor and scaled by a radius:
//!
//! - `type="t"` (default): a robust multivariate t fit (5 degrees of freedom),
//!   the iteratively reweighted EM of Kent, Tyler and Vardi (1994), with the
//!   same start, weights, stopping rule (`tol` 0.01 on the weights, at most 25
//!   steps) and final scaling as `MASS::cov.trob`, which ggplot2 calls.
//! - `type="norm"`: the mean and the unbiased sample covariance.
//! - `type="euclid"`: a circle; ggplot2 makes the shape `min(variance) * I` and
//!   divides `level` by its Cholesky factor, so the radius is `level` data units.
//!
//! For `t` and `norm` the radius is `sqrt(2 * qf(level, 2, n - 1))`; the F
//! quantile with 2 numerator degrees of freedom has a closed form.
//!
//! What it does not do: groups with fewer than 4 points get no ellipse (ggplot2
//! prints a message; ggrust drops them silently), and a degenerate shape (e.g.
//! constant x: R's `chol` fails) gives no ellipse instead of an error. Points
//! exactly on a sloped line keep a rounding-sized width, as in R (a sliver).
//! No `cov.trob` convergence warning.

use std::f64::consts::PI;

use super::groups::per_group;
use super::{Stat, StatContext};
use crate::data::{ContinuousColumn, LayerData, Variable};
use crate::parameters::Parameters;
use crate::spec::{Aesthetic, Value};
use crate::{Error, Result};

/// Confidence ellipses of x and y.
pub struct EllipseStat;

/// ggplot2 defaults.
const DEFAULT_LEVEL: f64 = 0.95;
const DEFAULT_SEGMENTS: usize = 51;

/// `MASS::cov.trob` defaults: t degrees of freedom, weight tolerance, steps.
const TROB_NU: f64 = 5.0;
const TROB_TOLERANCE: f64 = 0.01;
const TROB_MAX_STEPS: usize = 25;

/// Which fit shapes the ellipse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EllipseType {
    T,
    Norm,
    Euclid,
}

impl Stat for EllipseStat {
    fn name(&self) -> &'static str {
        "ellipse"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["type", "level", "segments"]
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
        let kind = ellipse_type(parameters)?;
        let level = parameters.number("level")?.unwrap_or(DEFAULT_LEVEL);
        let level_ok = match kind {
            EllipseType::Euclid => level > 0.0,
            EllipseType::T | EllipseType::Norm => level > 0.0 && level < 1.0,
        };
        if !level_ok {
            let accepted = match kind {
                EllipseType::Euclid => "a radius above 0 for type='euclid'",
                _ => "a probability between 0 and 1 (exclusive), e.g. 0.95",
            };
            return Err(Error::spec(
                parameters.path_of("level"),
                format!("level must be {accepted}, got {level}"),
            ));
        }
        let segments = match parameters.number("segments")? {
            None => DEFAULT_SEGMENTS,
            Some(value) if value >= 1.0 && value.fract() == 0.0 && value <= 1e6 => value as usize,
            Some(value) => {
                return Err(Error::spec(
                    parameters.path_of("segments"),
                    format!(
                        "segments must be a whole number of at least 1 (default 51), got {value}"
                    ),
                ));
            }
        };
        let empty = ContinuousColumn::from_values(Vec::new());
        let x = data.continuous(Aesthetic::X).unwrap_or(&empty);
        let y = data.continuous(Aesthetic::Y).unwrap_or(&empty);
        per_group(&data, |rows| {
            let points: Vec<(f64, f64)> = rows
                .iter()
                .filter_map(|&row| Some((x.get(row)?, y.get(row)?)))
                .filter(|(x, y)| x.is_finite() && y.is_finite())
                .collect();
            let (xs, ys) = ellipse(&points, kind, level, segments)
                .map(|outline| outline.into_iter().unzip())
                .unwrap_or_default();
            Ok(vec![
                (Variable::Aesthetic(Aesthetic::X), xs),
                (Variable::Aesthetic(Aesthetic::Y), ys),
            ])
        })
    }
}

/// `type=`: `"t"` (default), `"norm"` or `"euclid"`.
fn ellipse_type(parameters: &Parameters) -> Result<EllipseType> {
    match parameters.value("type") {
        None | Some(Value::Null) => Ok(EllipseType::T),
        Some(Value::String(text)) if text == "t" => Ok(EllipseType::T),
        Some(Value::String(text)) if text == "norm" => Ok(EllipseType::Norm),
        Some(Value::String(text)) if text == "euclid" => Ok(EllipseType::Euclid),
        Some(other) => Err(Error::spec(
            parameters.path_of("type"),
            format!(
                "unknown ellipse type {other}; use 't' (robust t fit, default), 'norm' \
                 (normal) or 'euclid' (a circle of radius level)"
            ),
        )),
    }
}

/// Location and 2x2 shape `[s11, s12, s22]` of the points.
type Fit = ((f64, f64), [f64; 3]);

/// The outline of one group: `segments + 1` points, or `None` with too few
/// points (fewer than 4, as ggplot2) or a degenerate shape.
fn ellipse(
    points: &[(f64, f64)],
    kind: EllipseType,
    level: f64,
    segments: usize,
) -> Option<Vec<(f64, f64)>> {
    let degrees = points.len() as f64 - 1.0;
    if degrees < 3.0 {
        return None;
    }
    let ((center_x, center_y), shape) = match kind {
        EllipseType::T => cov_trob(points).0,
        EllipseType::Norm => sample_covariance(points),
        EllipseType::Euclid => {
            let (center, [s11, _, s22]) = sample_covariance(points);
            let smallest = s11.min(s22);
            (center, [smallest, 0.0, smallest])
        }
    };
    let [r11, r12, r22] = cholesky(shape)?;
    let radius = match kind {
        EllipseType::Euclid => level / r11.max(r12).max(r22),
        _ => (2.0 * f_quantile_two(level, degrees)).sqrt(),
    };
    Some(
        (0..=segments)
            .map(|index| {
                let angle = index as f64 * 2.0 * PI / segments as f64;
                let (cos, sin) = (angle.cos(), angle.sin());
                (
                    center_x + radius * (cos * r11 + sin * 0.0),
                    center_y + radius * (cos * r12 + sin * r22),
                )
            })
            .collect(),
    )
}

/// Upper Cholesky factor `[r11, r12, r22]` of a 2x2 shape (R `chol`); `None`
/// when it is not positive definite.
fn cholesky([s11, s12, s22]: [f64; 3]) -> Option<[f64; 3]> {
    if s11.is_nan() || s11 <= 0.0 {
        return None;
    }
    let r11 = s11.sqrt();
    let r12 = s12 / r11;
    let rest = s22 - r12 * r12;
    if rest.is_nan() || rest <= 0.0 || rest.is_infinite() {
        return None;
    }
    Some([r11, r12, rest.sqrt()])
}

/// Quantile of the F distribution with 2 and `degrees` degrees of freedom:
/// its CDF is `1 - (1 + 2x/d)^(-d/2)`.
fn f_quantile_two(probability: f64, degrees: f64) -> f64 {
    degrees / 2.0 * ((-2.0 / degrees) * (-probability).ln_1p()).exp_m1()
}

/// Mean and unbiased covariance (R `cov.wt` with unit weights).
fn sample_covariance(points: &[(f64, f64)]) -> Fit {
    let n = points.len() as f64;
    let mean_x = points.iter().map(|point| point.0).sum::<f64>() / n;
    let mean_y = points.iter().map(|point| point.1).sum::<f64>() / n;
    let mut shape = [0.0; 3];
    for (x, y) in points {
        let (dx, dy) = (x - mean_x, y - mean_y);
        shape[0] += dx * dx;
        shape[1] += dx * dy;
        shape[2] += dy * dy;
    }
    ((mean_x, mean_y), shape.map(|value| value / (n - 1.0)))
}

/// Robust multivariate t location and scatter (as `MASS::cov.trob(x)` with
/// unit weights, `nu = 5`): start at the mean with weights `1 + p/nu`; each
/// step computes squared Mahalanobis distances `Q` of the centred points under
/// the weighted scatter, new weights `(nu + p) / (nu + Q)` and a weighted mean;
/// stop when no weight moved by `tol` or more. Also returns the number of
/// completed steps before the stopping one (`cov.trob`'s `iter`).
fn cov_trob(points: &[(f64, f64)]) -> (Fit, usize) {
    let n = points.len();
    let p = 2.0;
    let mut location = (
        points.iter().map(|point| point.0).sum::<f64>() / n as f64,
        points.iter().map(|point| point.1).sum::<f64>() / n as f64,
    );
    let mut weights = vec![1.0 + p / TROB_NU; n];
    let mut centred = vec![(0.0, 0.0); n];
    let mut steps = 0;
    for step in 1..=TROB_MAX_STEPS {
        for (centre, (x, y)) in centred.iter_mut().zip(points) {
            *centre = (x - location.0, y - location.1);
        }
        let total: f64 = weights.iter().sum();
        let mut scatter = [0.0; 3];
        for (weight, (dx, dy)) in weights.iter().zip(&centred) {
            let share = weight / total;
            scatter[0] += share * dx * dx;
            scatter[1] += share * dx * dy;
            scatter[2] += share * dy * dy;
        }
        let determinant = scatter[0] * scatter[2] - scatter[1] * scatter[1];
        let mut moved = false;
        for (weight, (dx, dy)) in weights.iter_mut().zip(&centred) {
            let distance = (scatter[2] * dx * dx - 2.0 * scatter[1] * dx * dy
                + scatter[0] * dy * dy)
                / determinant;
            let next = (TROB_NU + p) / (TROB_NU + distance);
            // NaN (degenerate scatter) never counts as settled, as R's `all()`.
            let change = (next - *weight).abs();
            if change.is_nan() || change >= TROB_TOLERANCE {
                moved = true;
            }
            *weight = next;
        }
        let total: f64 = weights.iter().sum();
        location = (
            weights
                .iter()
                .zip(points)
                .map(|(w, point)| w * point.0)
                .sum::<f64>()
                / total,
            weights
                .iter()
                .zip(points)
                .map(|(w, point)| w * point.1)
                .sum::<f64>()
                / total,
        );
        if !moved {
            break;
        }
        steps = step;
    }
    // Scatter of the last centred points under the last weights, over n.
    let mut shape = [0.0; 3];
    for (weight, (dx, dy)) in weights.iter().zip(&centred) {
        shape[0] += weight * dx * dx;
        shape[1] += weight * dx * dy;
        shape[2] += weight * dy * dy;
    }
    ((location, shape.map(|value| value / n as f64)), steps)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cloud() -> Vec<(f64, f64)> {
        vec![
            (0.0, 0.1),
            (1.0, 0.8),
            (2.0, 2.3),
            (3.0, 2.9),
            (4.0, 4.2),
            (2.5, 1.0),
            (1.5, 2.0),
            (9.0, -3.0),
        ]
    }

    #[test]
    fn f_quantile_inverts_the_cdf() {
        for (probability, degrees) in [(0.95, 3.0), (0.5, 10.0), (0.99, 59.0)] {
            let x = f_quantile_two(probability, degrees);
            let cdf = 1.0 - (1.0 + 2.0 * x / degrees).powf(-degrees / 2.0);
            assert!((cdf - probability).abs() < 1e-14, "{probability} {degrees}");
        }
        // R: qf(0.95, 2, 59) = 3.153123...
        assert!((f_quantile_two(0.95, 59.0) - 3.153_123_21).abs() < 1e-7);
    }

    #[test]
    fn closed_outline_with_segments_plus_one_points() {
        let outline = ellipse(&cloud(), EllipseType::Norm, 0.95, 10).unwrap();
        assert_eq!(outline.len(), 11);
        let (first, last) = (outline[0], outline[10]);
        assert!((first.0 - last.0).abs() < 1e-12 && (first.1 - last.1).abs() < 1e-12);
    }

    #[test]
    fn euclid_is_a_circle_of_radius_level() {
        let points = cloud();
        let outline = ellipse(&points, EllipseType::Euclid, 2.0, 40).unwrap();
        let ((cx, cy), _) = sample_covariance(&points);
        for (x, y) in outline {
            assert!((((x - cx).powi(2) + (y - cy).powi(2)).sqrt() - 2.0).abs() < 1e-12);
        }
    }

    #[test]
    fn too_few_or_collinear_points_give_nothing() {
        let three = [(0.0, 0.0), (1.0, 2.0), (2.0, 1.0)];
        assert!(ellipse(&three, EllipseType::T, 0.95, 51).is_none());
        let line: Vec<(f64, f64)> = (0..6).map(|i| (2.0, i as f64)).collect();
        assert!(ellipse(&line, EllipseType::Norm, 0.95, 51).is_none());
        assert!(ellipse(&line, EllipseType::T, 0.95, 51).is_none());
    }

    #[test]
    fn trob_downweights_the_outlier() {
        let ((center, shape), steps) = cov_trob(&cloud());
        let ((mean, covariance), _) = (sample_covariance(&cloud()), ());
        assert!(steps >= 1);
        // The outlier at (9, -3) pulls the robust centre less than the mean.
        assert!(center.0 < mean.0 && center.1 > mean.1);
        assert!(shape[0] < covariance[0]);
    }

    /// `cov.trob` fits of `tests/fixtures/ellipse_reference.json` (MASS 7.3).
    #[test]
    fn trob_matches_mass() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/ellipse_reference.json"
        );
        let text = std::fs::read_to_string(path).unwrap();
        let reference: serde_json::Value = serde_json::from_str(&text).unwrap();
        let rows = reference["data"].as_array().unwrap();
        for group in ["a", "b"] {
            let points: Vec<(f64, f64)> = rows
                .iter()
                .filter(|row| row["g"] == group)
                .map(|row| (row["x"].as_f64().unwrap(), row["y"].as_f64().unwrap()))
                .collect();
            let ((center, shape), steps) = cov_trob(&points);
            let fit = &reference["trob"][group];
            let number = |value: &serde_json::Value| value.as_f64().unwrap();
            assert_eq!(steps as u64, fit["iterations"].as_u64().unwrap(), "{group}");
            assert!((center.0 - number(&fit["center"][0])).abs() < 1e-12);
            assert!((center.1 - number(&fit["center"][1])).abs() < 1e-12);
            for (value, index) in shape.iter().zip([0, 1, 3]) {
                assert!(
                    (value - number(&fit["cov"][index])).abs() < 1e-12,
                    "{group}"
                );
            }
        }
    }
}
