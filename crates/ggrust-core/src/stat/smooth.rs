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
//! As lets-plot, `se` uses the straight-line formula `s * sqrt(1/n + (x - mean)^2
//! / Sxx)` for every method, with `n - deg - 1` degrees of freedom (loess: `deg`
//! 1). ggplot2 uses each model's own standard errors, so polynomial and loess
//! bands differ from ggplot2.
//!
//! What it does not do: `glm`, `gam`, `rlm` (lets-plot neither), `orientation`.

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
        let span = parameters.number("span")?.unwrap_or(DEFAULT_SPAN);
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
            degree: whole("deg", 1, 1)?,
            max_n: whole("max_n", DEFAULT_MAX_N, 1)?,
            seed,
        })
    }
}

impl Stat for SmoothStat {
    fn name(&self) -> &'static str {
        "smooth"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["method", "n", "se", "level", "span", "deg", "seed", "max_n"]
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
        let options = SmoothOptions::from_parameters(parameters)?;
        let empty = ContinuousColumn::from_values(Vec::new());
        let x = data.continuous(Aesthetic::X).unwrap_or(&empty);
        let y = data.continuous(Aesthetic::Y).unwrap_or(&empty);
        per_group(&data, |rows| {
            let rows = match options.method == Method::Loess && rows.len() > options.max_n {
                true => sample_rows(rows, options.max_n, options.seed),
                false => rows.to_vec(),
            };
            let curve = smooth(&rows, x, y, &options);
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
    let evaluator = Evaluator::new(&fit, options.level);
    let step = (high - low) / (options.n - 1) as f64;
    let mut curve = Curve::default();
    for index in 0..options.n {
        let at = low + index as f64 * step;
        let value = fit.value(at.clamp(low, high));
        let se = evaluator.se(at.clamp(low, high));
        let half = evaluator.t_critical * se;
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
    /// Degree used for the degrees of freedom (`n - degree - 1`).
    degree: f64,
}

enum Model {
    Line { slope: f64, intercept: f64 },
    Orthogonal(OrthogonalPolynomial),
    Spline(Spline),
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
        Some(Fit {
            xs,
            ys,
            model,
            degree: 1.0,
        })
    }

    /// lets-plot `PolynomialRegression.fit`.
    fn polynomial(xs: Vec<f64>, ys: Vec<f64>, degree: usize) -> Option<Fit> {
        if xs.len() <= degree {
            return None;
        }
        let model = Model::Orthogonal(OrthogonalPolynomial::fit(&xs, &ys, degree));
        Some(Fit {
            xs,
            ys,
            model,
            degree: degree as f64,
        })
    }

    /// lets-plot `LocalPolynomialRegression.fit`.
    fn loess(xs: Vec<f64>, ys: Vec<f64>, span: f64) -> Option<Fit> {
        let n = xs.len();
        if n < 3 || span * (n as f64) < 2.0 {
            return None;
        }
        let fitted = loess_smooth(&xs, &ys, span, 4);
        let model = Model::Spline(Spline::natural(&xs, &fitted));
        Some(Fit {
            xs,
            ys,
            model,
            degree: 1.0,
        })
    }

    fn value(&self, x: f64) -> f64 {
        match &self.model {
            Model::Line { slope, intercept } => slope * x + intercept,
            Model::Orthogonal(polynomial) => polynomial.value(x),
            Model::Spline(spline) => spline.value(x),
        }
    }
}

/// lets-plot `RegressionEvaluator`: the standard error of the mean at x.
struct Evaluator {
    n: f64,
    mean_x: f64,
    sum_xx: f64,
    standard_error: f64,
    t_critical: f64,
}

impl Evaluator {
    fn new(fit: &Fit, level: f64) -> Self {
        let n = fit.xs.len() as f64;
        let mean_x = mean(&fit.xs);
        let sum_xx = fit.xs.iter().map(|x| (x - mean_x).powi(2)).sum();
        let freedom = n - fit.degree - 1.0;
        let residuals: f64 = fit
            .xs
            .iter()
            .zip(&fit.ys)
            .map(|(&x, y)| (y - fit.value(x)).powi(2))
            .sum();
        let t_critical = if freedom > 0.0 {
            t_quantile(1.0 - (1.0 - level) / 2.0, freedom)
        } else {
            f64::NAN
        };
        Evaluator {
            n,
            mean_x,
            sum_xx,
            standard_error: (residuals / freedom).sqrt(),
            t_critical,
        }
    }

    fn se(&self, x: f64) -> f64 {
        self.standard_error * (1.0 / self.n + (x - self.mean_x).powi(2) / self.sum_xx).sqrt()
    }
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
}

impl OrthogonalPolynomial {
    fn fit(xs: &[f64], ys: &[f64], degree: usize) -> Self {
        let mut polynomial = OrthogonalPolynomial {
            recurrence: Vec::new(),
            mean: mean(xs),
            coefficients: Vec::new(),
        };
        // Values of p(k-1) and p(k) at the knots.
        let mut previous = vec![1.0; xs.len()];
        let mut current: Vec<f64> = xs.iter().map(|x| x - polynomial.mean).collect();
        let coefficient = |values: &[f64]| {
            let (weighted, squared) = values
                .iter()
                .zip(ys)
                .fold((0.0, 0.0), |(w, s), (p, y)| (w + y * p, s + p * p));
            weighted / squared
        };
        polynomial.coefficients.push(coefficient(&previous));
        if degree >= 1 {
            polynomial.coefficients.push(coefficient(&current));
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
            polynomial.coefficients.push(coefficient(&next));
            previous = std::mem::replace(&mut current, next);
        }
        polynomial
    }

    fn value(&self, x: f64) -> f64 {
        let (mut previous, mut current) = (1.0, x - self.mean);
        let mut sum = self.coefficients[0] * previous;
        if let Some(coefficient) = self.coefficients.get(1) {
            sum += coefficient * current;
        }
        for (&(alpha, beta), coefficient) in self.recurrence.iter().zip(&self.coefficients[2..]) {
            let next = (x - alpha) * current - beta * previous;
            sum += coefficient * next;
            (previous, current) = (current, next);
        }
        sum
    }
}

/// Apache Commons Math `LoessInterpolator.smooth` with unit weights: fitted
/// values at every x (sorted, distinct).
fn loess_smooth(xs: &[f64], ys: &[f64], span: f64, robustness_iterations: usize) -> Vec<f64> {
    const ACCURACY: f64 = 1e-12;
    let n = xs.len();
    if n <= 2 {
        return ys.to_vec();
    }
    let window = (span * n as f64) as usize;
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
    fitted
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
fn t_quantile(p: f64, freedom: f64) -> f64 {
    let cdf = |t: f64| {
        let tail = 0.5 * regularized_beta(freedom / (freedom + t * t), freedom / 2.0, 0.5);
        if t >= 0.0 { 1.0 - tail } else { tail }
    };
    let (mut low, mut high) = (0.0, 1.0);
    while cdf(high) < p && high < 1e12 {
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
fn regularized_beta(x: f64, a: f64, b: f64) -> f64 {
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
fn ln_gamma(x: f64) -> f64 {
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
        let evaluator = Evaluator::new(&fit, 0.95);
        let expected = 0.9_f64.sqrt() * (0.25_f64 + 0.0).sqrt();
        assert!((evaluator.se(2.5) - expected).abs() < 1e-12);
    }

    #[test]
    fn loess_is_exact_on_a_line_and_spline_interpolates() {
        let xs: Vec<f64> = (0..20).map(f64::from).collect();
        let ys: Vec<f64> = xs.iter().map(|x| 3.0 * x + 1.0).collect();
        let fitted = loess_smooth(&xs, &ys, 0.5, 4);
        for (value, y) in fitted.iter().zip(&ys) {
            assert!((value - y).abs() < 1e-9);
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
