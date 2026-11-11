//! `stat="smooth"`: a fitted curve with a confidence band per group, as
//! lets-plot's `SmoothStat`.
//!
//! Methods: `lm` (least squares line; `deg` >= 2 fits a polynomial through
//! Forsythe orthogonal polynomials) and `loess`/`lowess` (Apache Commons Math
//! `LoessInterpolator` with 4 robustness iterations, then a natural cubic spline
//! through the fitted values). Polynomials and loess first average y over equal
//! x. Groups with more than `max_n` rows (1000) are sampled for loess with
//! Kotlin's `Random(seed)` (seed 37), as lets-plot. The curve is evaluated at `n`
//! (80) evenly spaced x over the group's range; with `se` (the default) the band
//! is `y -/+ t * se` at `level` (0.95).
//!
//! `se` is each model's own standard error, as ggplot2 (R's `predict(..., se =
//! TRUE)`): lines and polynomials `s * sqrt(x0' (X'X)^-1 x0)` with `n - deg - 1`
//! degrees of freedom; loess, a linear smoother `y_hat = L y`, `s * |l(x0)|` with
//! `s^2 = RSS / delta1` and `delta1^2 / delta2` degrees of freedom (`delta1 =
//! tr(M'M)`, `delta2 = tr((M'M)^2)`, `M = I - L`; above 500 points `delta1`).
//! A recorded deviation: lets-plot uses the straight-line formula for every
//! method, so its polynomial and loess bands differ.
//!
//! ggplot2 options: `fullrange` evaluates the curve over the plot's whole x range
//! (lines and polynomials extrapolate; loess, as R, stops at its data);
//! `formula` is `y ~ x` or `y ~ poly(x, k)` (the same as `deg=k`); `method_args`
//! passes `span` to loess.
//!
//! What it does not do: `glm`, `gam`, `rlm` (lets-plot neither), `orientation`,
//! other formula terms.

use std::collections::HashSet;

use super::groups::per_group;
use super::{Stat, StatContext};
use crate::data::{Computed, ContinuousColumn, LayerData, Variable};
use crate::parameters::Parameters;
use crate::position::random::KotlinRandom;
use crate::spec::Aesthetic;
use crate::{Error, Result};

/// Fitted curves.
pub struct SmoothStat;

/// lets-plot `SmoothStat` defaults.
const DEFAULT_N: usize = 80;
const DEFAULT_LEVEL: f64 = 0.95;
const DEFAULT_SPAN: f64 = 0.5;
const DEFAULT_MAX_N: usize = 1000;
const DEFAULT_SEED: i64 = 37;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Method {
    Linear,
    Loess,
}

struct SmoothOptions {
    method: Method,
    n: usize,
    se: bool,
    level: f64,
    span: f64,
    degree: usize,
    max_n: usize,
    seed: i64,
    full_range: bool,
}

impl SmoothOptions {
    fn from_parameters(parameters: &Parameters) -> Result<Self> {
        let method = match parameters.value("method") {
            None => Method::Linear,
            Some(value) => match value.as_str().map(str::to_lowercase).as_deref() {
                Some("lm") => Method::Linear,
                Some("loess" | "lowess") => Method::Loess,
                Some(name @ ("glm" | "gam" | "rlm")) => {
                    return Err(Error::spec(
                        parameters.path_of("method"),
                        format!("method '{name}' is not supported; use 'lm' or 'loess'"),
                    ));
                }
                _ => {
                    let given = value.as_str().unwrap_or_default();
                    return Err(Error::spec(
                        parameters.path_of("method"),
                        format!(
                            "unknown smoothing method {value}; expected 'lm', 'loess' or 'lowess'{}",
                            crate::suggest::did_you_mean(given, ["lm", "loess", "lowess"])
                        ),
                    ));
                }
            },
        };
        let whole = |name: &str, default: usize, least: usize| -> Result<usize> {
            match parameters.number(name)? {
                None => Ok(default),
                Some(value) if value >= least as f64 && value.fract() == 0.0 => Ok(value as usize),
                Some(value) => Err(Error::spec(
                    parameters.path_of(name),
                    format!("expected a whole number of at least {least}, got {value}"),
                )),
            }
        };
        let level = parameters.number("level")?.unwrap_or(DEFAULT_LEVEL);
        if !(0.01..=0.99).contains(&level) {
            return Err(Error::spec(
                parameters.path_of("level"),
                format!("expected a confidence level in [0.01, 0.99], got {level}"),
            ));
        }
        let span = match method_span(parameters, method)? {
            Some(span) if parameters.value("span").is_some() => {
                return Err(Error::spec(
                    parameters.path_of("method_args"),
                    format!("span is given twice (span= and method_args); keep one, got {span}"),
                ));
            }
            Some(span) => span,
            None => parameters.number("span")?.unwrap_or(DEFAULT_SPAN),
        };
        if !(span > 0.0 && span <= 1.0) {
            return Err(Error::spec(
                parameters.path_of("span"),
                format!("expected a span in (0, 1], got {span}"),
            ));
        }
        let seed = match parameters.number("seed")? {
            None => DEFAULT_SEED,
            Some(value) if value.fract() == 0.0 && value.abs() < 9.0e15 => value as i64,
            Some(value) => {
                return Err(Error::spec(
                    parameters.path_of("seed"),
                    format!("expected a whole number, got {value}"),
                ));
            }
        };
        Ok(SmoothOptions {
            method,
            n: whole("n", DEFAULT_N, 2)?,
            se: parameters.boolean("se")?.unwrap_or(true),
            level,
            span,
            degree: match formula_degree(parameters, method)? {
                Some(degree)
                    if parameters
                        .value("deg")
                        .is_some_and(|deg| deg.as_f64() != Some(degree as f64)) =>
                {
                    return Err(Error::spec(
                        parameters.path_of("formula"),
                        format!("the formula sets degree {degree} but deg= sets another; keep one"),
                    ));
                }
                Some(degree) => degree,
                None => whole("deg", 1, 1)?,
            },
            max_n: whole("max_n", DEFAULT_MAX_N, 1)?,
            seed,
            full_range: parameters.boolean("fullrange")?.unwrap_or(false),
        })
    }
}

/// ggplot2 `formula`: `y ~ x` (degree 1) or `y ~ poly(x, k)` (degree k, lm only).
fn formula_degree(parameters: &Parameters, method: Method) -> Result<Option<usize>> {
    let Some(formula) = parameters.string("formula")? else {
        return Ok(None);
    };
    let compact: String = formula.chars().filter(|c| !c.is_whitespace()).collect();
    let unsupported = || {
        Error::spec(
            parameters.path_of("formula"),
            format!(
                "unsupported formula '{formula}'; supported: 'y ~ x', and 'y ~ poly(x, k)' \
                 with method='lm'"
            ),
        )
    };
    let Some(right) = compact.strip_prefix("y~") else {
        return Err(unsupported());
    };
    if right == "x" {
        return Ok(Some(1));
    }
    let degree = right
        .strip_prefix("poly(x,")
        .and_then(|rest| rest.strip_suffix(')'))
        .map(|degree| degree.strip_prefix("degree=").unwrap_or(degree))
        .and_then(|degree| degree.parse::<usize>().ok())
        .filter(|degree| *degree >= 1)
        .ok_or_else(unsupported)?;
    if method != Method::Linear {
        return Err(unsupported());
    }
    Ok(Some(degree))
}

/// ggplot2 `method.args`: `{"span": s}` for loess, nothing for lm.
fn method_span(parameters: &Parameters, method: Method) -> Result<Option<f64>> {
    let Some(value) = parameters.value("method_args") else {
        return Ok(None);
    };
    let allowed: &[&str] = match method {
        Method::Loess => &["span"],
        Method::Linear => &[],
    };
    let crate::spec::Value::Object(arguments) = value else {
        return Err(Error::spec(
            parameters.path_of("method_args"),
            format!("expected a dict like {{'span': 0.3}}, got {value}"),
        ));
    };
    if let Some(key) = arguments
        .keys()
        .find(|key| !allowed.contains(&key.as_str()))
    {
        let accepted = if allowed.is_empty() {
            "none for method 'lm'".to_string()
        } else {
            allowed.join(", ")
        };
        return Err(Error::spec(
            parameters.path_of("method_args"),
            format!("unknown method argument '{key}'; accepted: {accepted}"),
        ));
    }
    match arguments.get("span") {
        None => Ok(None),
        Some(span) => match span.as_f64().filter(|span| *span > 0.0 && span.is_finite()) {
            Some(span) => Ok(Some(span)),
            None => Err(Error::spec(
                format!("{}.span", parameters.path_of("method_args")),
                format!("span must be a positive number, got {span}"),
            )),
        },
    }
}

impl Stat for SmoothStat {
    fn name(&self) -> &'static str {
        "smooth"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &[
            "method",
            "n",
            "se",
            "level",
            "span",
            "deg",
            "seed",
            "max_n",
            "fullrange",
            "formula",
            "method_args",
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
        let options = SmoothOptions::from_parameters(parameters)?;
        let full_range = context.x_range.filter(|_| options.full_range);
        let empty = ContinuousColumn::from_values(Vec::new());
        let x = data.continuous(Aesthetic::X).unwrap_or(&empty);
        let y = data.continuous(Aesthetic::Y).unwrap_or(&empty);
        per_group(&data, |rows| {
            let rows = match options.method == Method::Loess && rows.len() > options.max_n {
                true => sample_rows(rows, options.max_n, options.seed),
                false => rows.to_vec(),
            };
            let curve = smooth(&rows, x, y, &options, full_range);
            let mut columns = vec![
                (Variable::Aesthetic(Aesthetic::X), curve.x),
                (Variable::Aesthetic(Aesthetic::Y), curve.y),
            ];
            if options.se {
                columns.extend([
                    (Variable::Aesthetic(Aesthetic::Ymin), curve.low),
                    (Variable::Aesthetic(Aesthetic::Ymax), curve.high),
                    (Variable::Computed(Computed::Se), curve.se),
                ]);
            }
            Ok(columns)
        })
    }
}

/// lets-plot `SamplingUtil.sampleWithoutReplacement`: draw distinct indices with
/// Kotlin `nextInt(size)` until `size` of them are picked (or, past half the rows,
/// dropped). Rows keep their order.
fn sample_rows(rows: &[usize], size: usize, seed: i64) -> Vec<usize> {
    let mut random = KotlinRandom::new(seed);
    let pick = size <= rows.len() / 2;
    let count = if pick { size } else { rows.len() - size };
    let mut chosen = HashSet::with_capacity(count);
    while chosen.len() < count {
        chosen.insert(random.next_int_until(rows.len() as i32) as usize);
    }
    (0..rows.len())
        .filter(|index| chosen.contains(index) == pick)
        .map(|index| rows[index])
        .collect()
}

#[derive(Default)]
struct Curve {
    x: Vec<f64>,
    y: Vec<f64>,
    low: Vec<f64>,
    high: Vec<f64>,
    se: Vec<f64>,
}

/// The fitted curve of `rows`; empty when it cannot be fitted (fewer than 3
/// finite rows, too few distinct x for the method).
fn smooth(
    rows: &[usize],
    x: &ContinuousColumn,
    y: &ContinuousColumn,
    options: &SmoothOptions,
    full_range: Option<(f64, f64)>,
) -> Curve {
    let (mut xs, mut ys) = (Vec::new(), Vec::new());
    for &row in rows {
        if let (Some(x), Some(y)) = (x.get(row), y.get(row))
            && x.is_finite()
            && y.is_finite()
        {
            xs.push(x);
            ys.push(y);
        }
    }
    // lets-plot needs 3 y values (missing included) before fitting.
    if rows.len() < 3 || xs.is_empty() {
        return Curve::default();
    }
    let fit = match (options.method, options.degree) {
        (Method::Linear, 1) => Fit::linear(xs, ys),
        (Method::Linear, degree) => {
            let (xs, ys) = average_by_x(&xs, &ys);
            Fit::polynomial(xs, ys, degree)
        }
        (Method::Loess, _) => {
            let (xs, ys) = average_by_x(&xs, &ys);
            Fit::loess(xs, ys, options.span)
        }
    };
    let Some(fit) = fit else {
        return Curve::default();
    };
    let (low, high) = fit
        .xs
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), &x| {
            (low.min(x), high.max(x))
        });
    // Lines and polynomials extrapolate over the full range; loess (R `predict`
    // gives NA) keeps the points within its data.
    let extrapolate = !matches!(fit.model, Model::Spline { .. });
    let (from, to) = full_range.map_or((low, high), |(from, to)| (from.min(low), to.max(high)));
    let step = (to - from) / (options.n - 1) as f64;
    let points: Vec<f64> = (0..options.n)
        .map(|index| from + index as f64 * step)
        .filter(|at| extrapolate || (low..=high).contains(at))
        .collect();
    let (errors, t_critical) = match options.se {
        true => fit.standard_errors(&points, options.level, extrapolate),
        false => (vec![f64::NAN; points.len()], f64::NAN),
    };
    let mut curve = Curve::default();
    for (&at, se) in points.iter().zip(errors) {
        let value = fit.value(if extrapolate { at } else { at.clamp(low, high) });
        let half = t_critical * se;
        curve.x.push(at);
        curve.y.push(value);
        curve.low.push(value - half);
        curve.high.push(value + half);
        curve.se.push(se);
    }
    curve
}

/// lets-plot `averageByX`: finite pairs sorted by x, y averaged over equal x.
fn average_by_x(xs: &[f64], ys: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let mut pairs: Vec<(f64, f64)> = xs.iter().copied().zip(ys.iter().copied()).collect();
    pairs.sort_by(|left, right| left.0.total_cmp(&right.0));
    let (mut out_x, mut out_y): (Vec<f64>, Vec<f64>) = (Vec::new(), Vec::new());
    let mut count = 0.0;
    for (x, y) in pairs {
        if out_x.last() == Some(&x) {
            let last = out_y.len() - 1;
            out_y[last] += y;
            count += 1.0;
        } else {
            if let Some(last) = out_y.last_mut() {
                *last /= count;
            }
            out_x.push(x);
            out_y.push(y);
            count = 1.0;
        }
    }
    if let Some(last) = out_y.last_mut() {
        *last /= count;
    }
    (out_x, out_y)
}

/// A fitted model over the points it was fitted to.
struct Fit {
    xs: Vec<f64>,
    ys: Vec<f64>,
    model: Model,
}

enum Model {
    Line {
        slope: f64,
        intercept: f64,
    },
    Orthogonal(OrthogonalPolynomial),
    /// The spline through the loess fit and the smoother matrix `L` (row-major,
    /// fitted values `L y`).
    Spline {
        spline: Spline,
        smoother: Vec<f64>,
    },
}

impl Fit {
    /// lets-plot `LinearRegression.fit`.
    fn linear(xs: Vec<f64>, ys: Vec<f64>) -> Option<Fit> {
        if xs.len() <= 1 {
            return None;
        }
        let mean_x = mean(&xs);
        let mean_y = mean(&ys);
        let sum_xx: f64 = xs.iter().map(|x| (x - mean_x).powi(2)).sum();
        let sum_xy: f64 = xs
            .iter()
            .zip(&ys)
            .map(|(x, y)| (x - mean_x) * (y - mean_y))
            .sum();
        let slope = sum_xy / sum_xx;
        let model = Model::Line {
            slope,
            intercept: mean_y - slope * mean_x,
        };
        Some(Fit { xs, ys, model })
    }

    /// lets-plot `PolynomialRegression.fit`.
    fn polynomial(xs: Vec<f64>, ys: Vec<f64>, degree: usize) -> Option<Fit> {
        if xs.len() <= degree {
            return None;
        }
        let model = Model::Orthogonal(OrthogonalPolynomial::fit(&xs, &ys, degree));
        Some(Fit { xs, ys, model })
    }

    /// lets-plot `LocalPolynomialRegression.fit`.
    fn loess(xs: Vec<f64>, ys: Vec<f64>, span: f64) -> Option<Fit> {
        let n = xs.len();
        if n < 3 || span * (n as f64) < 2.0 {
            return None;
        }
        let (fitted, smoother) = loess_smooth(&xs, &ys, span, 4);
        let model = Model::Spline {
            spline: Spline::natural(&xs, &fitted),
            smoother,
        };
        Some(Fit { xs, ys, model })
    }

    fn value(&self, x: f64) -> f64 {
        match &self.model {
            Model::Line { slope, intercept } => slope * x + intercept,
            Model::Orthogonal(polynomial) => polynomial.value(x),
            Model::Spline { spline, .. } => spline.value(x),
        }
    }

    /// Standard errors of the fitted mean at `points` (clamped to the data
    /// range) and the two-sided t quantile at `level`.
    fn standard_errors(&self, points: &[f64], level: f64, extrapolate: bool) -> (Vec<f64>, f64) {
        let n = self.xs.len();
        let (low, high) = (
            self.xs[0].min(self.xs[n - 1]),
            self.xs[0].max(self.xs[n - 1]),
        );
        let (low, high) = self
            .xs
            .iter()
            .fold((low, high), |(a, b), &x| (a.min(x), b.max(x)));
        let points: Vec<f64> = points
            .iter()
            .map(|&x| if extrapolate { x } else { x.clamp(low, high) })
            .collect();
        let residuals: f64 = self
            .xs
            .iter()
            .zip(&self.ys)
            .map(|(&x, y)| (y - self.value(x)).powi(2))
            .sum();
        // Per point: the squared norm of the row that maps y to the fit there.
        let (leverages, freedom, scale_freedom): (Vec<f64>, f64, f64) = match &self.model {
            Model::Line { .. } => {
                let mean_x = mean(&self.xs);
                let sum_xx: f64 = self.xs.iter().map(|x| (x - mean_x).powi(2)).sum();
                let leverages = points
                    .iter()
                    .map(|x| 1.0 / n as f64 + (x - mean_x).powi(2) / sum_xx)
                    .collect();
                let freedom = n as f64 - 2.0;
                (leverages, freedom, freedom)
            }
            Model::Orthogonal(polynomial) => {
                let leverages = points.iter().map(|&x| polynomial.leverage(x)).collect();
                let freedom = (n - polynomial.coefficients.len()) as f64;
                (leverages, freedom, freedom)
            }
            Model::Spline { smoother, .. } => {
                let (delta1, delta2) = smoother_deltas(smoother, n);
                let freedom = if delta2 > 0.0 {
                    delta1 * delta1 / delta2
                } else {
                    delta1
                };
                (
                    smoother_leverages(&self.xs, smoother, &points),
                    freedom,
                    delta1,
                )
            }
        };
        let scale = (residuals / scale_freedom).sqrt();
        let t_critical = if freedom > 0.0 {
            t_quantile(1.0 - (1.0 - level) / 2.0, freedom)
        } else {
            f64::NAN
        };
        let errors = leverages
            .iter()
            .map(|leverage| scale * leverage.sqrt())
            .collect();
        (errors, t_critical)
    }
}

/// `|l(x0)|^2` at each point, where `l(x0)` maps y to the spline through `L y`
/// at `x0`: the spline is linear in its values, so column `j` of `L` gives `l_j`.
fn smoother_leverages(xs: &[f64], smoother: &[f64], points: &[f64]) -> Vec<f64> {
    let n = xs.len();
    let mut leverages = vec![0.0; points.len()];
    let mut column = vec![0.0; n];
    for j in 0..n {
        for (i, value) in column.iter_mut().enumerate() {
            *value = smoother[i * n + j];
        }
        let spline = Spline::natural(xs, &column);
        for (leverage, &x) in leverages.iter_mut().zip(points) {
            *leverage += spline.value(x).powi(2);
        }
    }
    leverages
}

/// Points up to which `delta2` is computed exactly (O(n^3)); above, `delta1`.
const EXACT_DELTA2_POINTS: usize = 500;

/// R loess `one.delta = tr(M'M)` and `two.delta = tr((M'M)^2)`, `M = I - L`
/// (`two.delta` is 0 above [`EXACT_DELTA2_POINTS`]).
fn smoother_deltas(smoother: &[f64], n: usize) -> (f64, f64) {
    let residual_operator = |i: usize, j: usize| f64::from(u8::from(i == j)) - smoother[i * n + j];
    let delta1 = (0..n * n)
        .map(|index| residual_operator(index / n, index % n).powi(2))
        .sum();
    if n > EXACT_DELTA2_POINTS {
        return (delta1, 0.0);
    }
    // A = M'M is symmetric: tr(A^2) is the sum of its squared entries.
    let columns: Vec<Vec<f64>> = (0..n)
        .map(|j| (0..n).map(|i| residual_operator(i, j)).collect())
        .collect();
    let mut delta2 = 0.0;
    for a in 0..n {
        for b in a..n {
            let entry: f64 = columns[a].iter().zip(&columns[b]).map(|(p, q)| p * q).sum();
            delta2 += if a == b {
                entry * entry
            } else {
                2.0 * entry * entry
            };
        }
    }
    (delta1, delta2)
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

/// A least-squares polynomial as a sum of Forsythe orthogonal polynomials
/// (lets-plot `ForsythePolynomialGenerator`): `p0 = 1`, `p1 = x - mean`,
/// `p(k+1) = (x - alpha) p(k) - beta p(k-1)`.
struct OrthogonalPolynomial {
    /// (alpha, beta) of each step past `p1`.
    recurrence: Vec<(f64, f64)>,
    mean: f64,
    coefficients: Vec<f64>,
    /// `sum p_k^2` over the fitted points: `X'X` of the orthogonal basis.
    norms: Vec<f64>,
}

impl OrthogonalPolynomial {
    fn fit(xs: &[f64], ys: &[f64], degree: usize) -> Self {
        let mut polynomial = OrthogonalPolynomial {
            recurrence: Vec::new(),
            mean: mean(xs),
            coefficients: Vec::new(),
            norms: Vec::new(),
        };
        // Values of p(k-1) and p(k) at the knots.
        let mut previous = vec![1.0; xs.len()];
        let mut current: Vec<f64> = xs.iter().map(|x| x - polynomial.mean).collect();
        let add = |polynomial: &mut OrthogonalPolynomial, values: &[f64]| {
            let (weighted, squared) = values
                .iter()
                .zip(ys)
                .fold((0.0, 0.0), |(w, s), (p, y)| (w + y * p, s + p * p));
            polynomial.coefficients.push(weighted / squared);
            polynomial.norms.push(squared);
        };
        add(&mut polynomial, &previous);
        if degree >= 1 {
            add(&mut polynomial, &current);
        }
        for _ in 2..=degree {
            let (mut sum_xp, mut sum_p, mut sum_previous) = (0.0, 0.0, 0.0);
            for ((x, p), q) in xs.iter().zip(&current).zip(&previous) {
                sum_xp += x * p * p;
                sum_p += p * p;
                sum_previous += q * q;
            }
            let (alpha, beta) = (sum_xp / sum_p, sum_p / sum_previous);
            polynomial.recurrence.push((alpha, beta));
            let next: Vec<f64> = xs
                .iter()
                .zip(&current)
                .zip(&previous)
                .map(|((x, p), q)| (x - alpha) * p - beta * q)
                .collect();
            add(&mut polynomial, &next);
            previous = std::mem::replace(&mut current, next);
        }
        polynomial
    }

    /// `p_0(x), p_1(x), ...`, one per coefficient.
    fn basis(&self, x: f64) -> Vec<f64> {
        let mut values = vec![1.0, x - self.mean];
        for &(alpha, beta) in &self.recurrence {
            let (previous, current) = (values[values.len() - 2], values[values.len() - 1]);
            values.push((x - alpha) * current - beta * previous);
        }
        values.truncate(self.coefficients.len());
        values
    }

    fn value(&self, x: f64) -> f64 {
        self.basis(x)
            .iter()
            .zip(&self.coefficients)
            .map(|(p, coefficient)| p * coefficient)
            .sum()
    }

    /// `x0' (X'X)^-1 x0`: the basis is orthogonal, so `X'X` is diagonal.
    fn leverage(&self, x: f64) -> f64 {
        self.basis(x)
            .iter()
            .zip(&self.norms)
            .map(|(p, norm)| p * p / norm)
            .sum()
    }
}

/// Apache Commons Math `LoessInterpolator.smooth` with unit weights: fitted
/// values at every x (sorted, distinct), and the smoother matrix of the last
/// pass (row-major, `fitted = L y` with that pass's robustness weights fixed).
fn loess_smooth(
    xs: &[f64],
    ys: &[f64],
    span: f64,
    robustness_iterations: usize,
) -> (Vec<f64>, Vec<f64>) {
    const ACCURACY: f64 = 1e-12;
    let n = xs.len();
    if n <= 2 {
        let identity = (0..n * n).map(|index| f64::from(u8::from(index % (n + 1) == 0)));
        return (ys.to_vec(), identity.collect());
    }
    let window = (span * n as f64) as usize;
    let mut smoother = vec![0.0; n * n];
    let mut fitted = vec![0.0; n];
    let mut residuals = vec![0.0; n];
    let mut robustness = vec![1.0; n];
    for iteration in 0..=robustness_iterations {
        let (mut left, mut right) = (0, window - 1);
        for i in 0..n {
            let x = xs[i];
            if i > 0 && right + 1 < n && xs[right + 1] - xs[i] < xs[i] - xs[left] {
                left += 1;
                right += 1;
            }
            let edge = if xs[i] - xs[left] > xs[right] - xs[i] {
                left
            } else {
                right
            };
            let denominator = (1.0 / (xs[edge] - x)).abs();
            let (mut sum_w, mut sum_x, mut sum_xx, mut sum_y, mut sum_xy) =
                (0.0, 0.0, 0.0, 0.0, 0.0);
            for k in left..=right {
                let (xk, yk) = (xs[k], ys[k]);
                let distance = if k < i { x - xk } else { xk - x };
                let w = tricube(distance * denominator) * robustness[k];
                let xkw = xk * w;
                sum_w += w;
                sum_x += xkw;
                sum_xx += xk * xkw;
                sum_y += yk * w;
                sum_xy += yk * xkw;
            }
            let (mean_x, mean_y) = (sum_x / sum_w, sum_y / sum_w);
            let (mean_xy, mean_xx) = (sum_xy / sum_w, sum_xx / sum_w);
            let beta = if (mean_xx - mean_x * mean_x).abs().sqrt() < ACCURACY {
                0.0
            } else {
                (mean_xy - mean_x * mean_y) / (mean_xx - mean_x * mean_x)
            };
            fitted[i] = beta * x + mean_y - beta * mean_x;
            // The same fit as weights on y: w_k / W * (1 + beta's share).
            let variance = mean_xx - mean_x * mean_x;
            for k in left..=right {
                let distance = if k < i { x - xs[k] } else { xs[k] - x };
                let w = tricube(distance * denominator) * robustness[k];
                let slope_share = if variance.abs().sqrt() < ACCURACY {
                    0.0
                } else {
                    (x - mean_x) * (xs[k] - mean_x) / variance
                };
                smoother[i * n + k] = w / sum_w * (1.0 + slope_share);
            }
            residuals[i] = (ys[i] - fitted[i]).abs();
        }
        if iteration == robustness_iterations {
            break;
        }
        let mut sorted = residuals.clone();
        sorted.sort_by(f64::total_cmp);
        let median = sorted[n / 2];
        if median.abs() < ACCURACY {
            break;
        }
        for (weight, residual) in robustness.iter_mut().zip(&residuals) {
            let argument = residual / (6.0 * median);
            *weight = if argument >= 1.0 {
                0.0
            } else {
                (1.0 - argument * argument).powi(2)
            };
        }
    }
    (fitted, smoother)
}

fn tricube(x: f64) -> f64 {
    let x = x.abs();
    if x >= 1.0 {
        0.0
    } else {
        (1.0 - x * x * x).powi(3)
    }
}

/// A natural cubic spline (Apache Commons Math `SplineInterpolator`).
struct Spline {
    knots: Vec<f64>,
    /// (a, b, c, d) of `a + b t + c t^2 + d t^3` per interval, `t = x - knot`.
    pieces: Vec<[f64; 4]>,
}

impl Spline {
    /// Needs at least 3 sorted, distinct knots.
    fn natural(x: &[f64], y: &[f64]) -> Spline {
        let n = x.len() - 1;
        let h: Vec<f64> = (0..n).map(|i| x[i + 1] - x[i]).collect();
        let mut mu = vec![0.0; n];
        let mut z = vec![0.0; n + 1];
        for i in 1..n {
            let g = 2.0 * (x[i + 1] - x[i - 1]) - h[i - 1] * mu[i - 1];
            mu[i] = h[i] / g;
            z[i] = (3.0 * (y[i + 1] * h[i - 1] - y[i] * (x[i + 1] - x[i - 1]) + y[i - 1] * h[i])
                / (h[i - 1] * h[i])
                - h[i - 1] * z[i - 1])
                / g;
        }
        let mut c = vec![0.0; n + 1];
        let mut pieces = vec![[0.0; 4]; n];
        for j in (0..n).rev() {
            c[j] = z[j] - mu[j] * c[j + 1];
            let b = (y[j + 1] - y[j]) / h[j] - h[j] * (c[j + 1] + 2.0 * c[j]) / 3.0;
            let d = (c[j + 1] - c[j]) / (3.0 * h[j]);
            pieces[j] = [y[j], b, c[j], d];
        }
        Spline {
            knots: x.to_vec(),
            pieces,
        }
    }

    /// Value inside the knot range (callers clamp x to it).
    fn value(&self, x: f64) -> f64 {
        let index = self
            .knots
            .partition_point(|knot| *knot <= x)
            .saturating_sub(1)
            .min(self.pieces.len() - 1);
        let [a, b, c, d] = self.pieces[index];
        let t = x - self.knots[index];
        a + t * (b + t * (c + t * d))
    }
}

/// Quantile `p` (> 0.5) of Student's t with `freedom` degrees of freedom, by
/// bisection on the CDF (lets-plot uses Apache `TDistribution`, same result).
pub(super) fn t_quantile(p: f64, freedom: f64) -> f64 {
    let cdf = |t: f64| {
        let tail = 0.5 * regularized_beta(freedom / (freedom + t * t), freedom / 2.0, 0.5);
        if t >= 0.0 { 1.0 - tail } else { tail }
    };
    let (mut low, mut high) = (0.0, 1.0);
    while cdf(high) < p && high < 1e300 {
        low = high;
        high *= 2.0;
    }
    for _ in 0..200 {
        let middle = 0.5 * (low + high);
        if cdf(middle) < p {
            low = middle;
        } else {
            high = middle;
        }
        if high - low <= 1e-14 * high.max(1.0) {
            break;
        }
    }
    0.5 * (low + high)
}

/// The regularized incomplete beta function `I_x(a, b)` (continued fraction,
/// modified Lentz).
pub(super) fn regularized_beta(x: f64, a: f64, b: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    if x > (a + 1.0) / (a + b + 2.0) {
        return 1.0 - regularized_beta(1.0 - x, b, a);
    }
    let front = (a * x.ln() + b * (1.0 - x).ln() - ln_beta(a, b)).exp() / a;
    const TINY: f64 = 1e-300;
    let (mut c, mut d) = (1.0, 1.0 - (a + b) * x / (a + 1.0));
    if d.abs() < TINY {
        d = TINY;
    }
    d = 1.0 / d;
    let mut result = d;
    for m in 1..500 {
        let m = m as f64;
        let even = m * (b - m) * x / ((a + 2.0 * m - 1.0) * (a + 2.0 * m));
        let odd = -(a + m) * (a + b + m) * x / ((a + 2.0 * m) * (a + 2.0 * m + 1.0));
        for coefficient in [even, odd] {
            d = 1.0 + coefficient * d;
            if d.abs() < TINY {
                d = TINY;
            }
            c = 1.0 + coefficient / c;
            if c.abs() < TINY {
                c = TINY;
            }
            d = 1.0 / d;
            result *= d * c;
        }
        if (d * c - 1.0).abs() < 1e-16 {
            break;
        }
    }
    front * result
}

fn ln_beta(a: f64, b: f64) -> f64 {
    ln_gamma(a) + ln_gamma(b) - ln_gamma(a + b)
}

/// Lanczos approximation (g = 7, 9 terms), accurate to about 1e-15.
pub(super) fn ln_gamma(x: f64) -> f64 {
    const COEFFICIENTS: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        let pi = std::f64::consts::PI;
        return (pi / (pi * x).sin()).ln() - ln_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let mut sum = COEFFICIENTS[0];
    for (index, coefficient) in COEFFICIENTS.iter().enumerate().skip(1) {
        sum += coefficient / (x + index as f64);
    }
    let t = x + 7.5;
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + sum.ln()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_quantiles_match_r() {
        // qt(0.975, df) in R.
        for (freedom, expected) in [(1.0, 12.706204736), (5.0, 2.570581836), (30.0, 2.042272456)] {
            assert!(
                (t_quantile(0.975, freedom) - expected).abs() < 1e-8,
                "df {freedom}"
            );
        }
    }

    #[test]
    fn polynomial_fit_recovers_a_quadratic() {
        let xs: Vec<f64> = (0..10).map(f64::from).collect();
        let ys: Vec<f64> = xs.iter().map(|x| 1.0 - 2.0 * x + 0.5 * x * x).collect();
        let fit = Fit::polynomial(xs, ys, 2).unwrap();
        for x in [0.0, 2.5, 9.0] {
            assert!((fit.value(x) - (1.0 - 2.0 * x + 0.5 * x * x)).abs() < 1e-9);
        }
    }

    #[test]
    fn line_and_its_band() {
        let xs = vec![1.0, 2.0, 3.0, 4.0];
        let ys = vec![1.0, 3.0, 2.0, 4.0];
        let fit = Fit::linear(xs, ys).unwrap();
        // lm(y ~ x): slope 0.8, intercept 0.5, residual s.e. sqrt(0.9).
        assert!((fit.value(0.0) - 0.5).abs() < 1e-12);
        let (errors, t_critical) = fit.standard_errors(&[2.5], 0.95, false);
        let expected = 0.9_f64.sqrt() * (0.25_f64 + 0.0).sqrt();
        assert!((errors[0] - expected).abs() < 1e-12);
        assert!((t_critical - 4.302_652_729_911_275).abs() < 1e-6);
    }

    #[test]
    fn polynomial_band_matches_r() {
        // predict(lm(y ~ poly(x, 2)), se.fit = TRUE) at 0, 2.5 and 9 (df 7).
        let xs: Vec<f64> = (0..10).map(f64::from).collect();
        let ys = vec![1.2, 0.1, -0.4, 0.3, -0.2, 1.1, 2.9, 4.2, 7.0, 9.9];
        let fit = Fit::polynomial(xs, ys, 2).unwrap();
        let (errors, t_critical) = fit.standard_errors(&[0.0, 2.5, 9.0], 0.95, false);
        for (error, r) in
            errors
                .iter()
                .zip([0.254262675333384, 0.138225062149778, 0.254262675333384])
        {
            assert!((error - r).abs() < 1e-12, "{error} vs {r}");
        }
        assert!((t_critical - 2.36462425159278).abs() < 1e-6);
    }

    #[test]
    fn loess_is_exact_on_a_line_and_spline_interpolates() {
        let xs: Vec<f64> = (0..20).map(f64::from).collect();
        let ys: Vec<f64> = xs.iter().map(|x| 3.0 * x + 1.0).collect();
        let (fitted, _) = loess_smooth(&xs, &ys, 0.5, 4);
        for (value, y) in fitted.iter().zip(&ys) {
            assert!((value - y).abs() < 1e-9);
        }
        // The smoother matrix reproduces the robust fit of noisy data.
        let noisy: Vec<f64> = xs
            .iter()
            .map(|x| (x * 0.7).sin() + (x * 3.1).cos() * 0.3)
            .collect();
        let (fitted, smoother) = loess_smooth(&xs, &noisy, 0.4, 4);
        for (i, value) in fitted.iter().enumerate() {
            let applied: f64 = (0..20).map(|k| smoother[i * 20 + k] * noisy[k]).sum();
            assert!((applied - value).abs() < 1e-9, "{applied} vs {value}");
        }
        let spline = Spline::natural(&xs, &ys);
        assert!((spline.value(4.5) - 14.5).abs() < 1e-9);
    }

    #[test]
    fn averages_equal_x_and_samples_distinct_rows() {
        let (xs, ys) = average_by_x(&[2.0, 1.0, 2.0], &[4.0, 1.0, 2.0]);
        assert_eq!((xs, ys), (vec![1.0, 2.0], vec![1.0, 3.0]));
        let rows: Vec<usize> = (0..50).collect();
        let picked = sample_rows(&rows, 10, 37);
        assert_eq!(picked.len(), 10);
        assert!(picked.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(sample_rows(&rows, 40, 37).len(), 40);
    }
}
