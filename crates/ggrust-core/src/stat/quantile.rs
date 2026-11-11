//! `stat="quantile"`: linear quantile regression lines, as ggplot2's
//! `stat_quantile()` with its default `formula = y ~ x` and `method = "rq"`.
//!
//! Per group and per quantile tau: the line a + b x minimising the check loss
//! sum rho_tau(y - a - b x). For a fixed slope the best intercept is the tau
//! quantile of the residuals, and the loss left is convex in the slope, so a
//! golden-section search over the slope finds the fit (`quantreg::rq` gives the
//! same coefficients). Each line is drawn over 100 points from the group's
//! smallest to largest x (ggplot2 `xseq`), as its own output group, with the
//! computed variable `..quantile..`.
//!
//! What it does not do: other formulas (splines), `method="rqss"`, `weight`.

use super::groups::per_group_pieces;
use super::{Stat, StatContext};
use crate::data::{Computed, ContinuousColumn, LayerData, Variable};
use crate::parameters::Parameters;
use crate::spec::Aesthetic;
use crate::{Error, Result};

/// Quantile regression lines.
pub struct QuantileStat;

/// ggplot2 `stat_quantile(quantiles = c(0.25, 0.5, 0.75))`.
const DEFAULT_QUANTILES: [f64; 3] = [0.25, 0.5, 0.75];

/// ggplot2 `xseq`: points per line.
const LINE_POINTS: usize = 100;

impl Stat for QuantileStat {
    fn name(&self) -> &'static str {
        "quantile"
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
        let quantiles = match super::qq::numbers(parameters, "quantiles")? {
            Some(values) => values,
            None => DEFAULT_QUANTILES.to_vec(),
        };
        if let Some(bad) = quantiles
            .iter()
            .find(|tau| !(tau.is_finite() && **tau > 0.0 && **tau < 1.0))
        {
            return Err(Error::spec(
                parameters.path_of("quantiles"),
                format!("expected probabilities between 0 and 1 (exclusive), got {bad}"),
            ));
        }
        let empty = ContinuousColumn::from_values(Vec::new());
        let x = data.continuous(Aesthetic::X).unwrap_or(&empty);
        let y = data.continuous(Aesthetic::Y).unwrap_or(&empty);
        per_group_pieces(&data, |rows| {
            let (xs, ys): (Vec<f64>, Vec<f64>) = rows
                .iter()
                .filter_map(|&row| {
                    let (x, y) = (x.get(row)?, y.get(row)?);
                    (x.is_finite() && y.is_finite()).then_some((x, y))
                })
                .unzip();
            let mut line_x = Vec::new();
            let mut line_y = Vec::new();
            let mut line_quantile = Vec::new();
            let mut sizes = Vec::new();
            let low = xs.iter().copied().fold(f64::INFINITY, f64::min);
            let high = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            // ggplot2 needs at least two distinct x values to fit a line.
            if xs.len() >= 2 && high > low {
                let grid: Vec<f64> = (0..LINE_POINTS)
                    .map(|index| low + (high - low) / (LINE_POINTS - 1) as f64 * index as f64)
                    .collect();
                for &tau in &quantiles {
                    let (intercept, slope) = fit(&xs, &ys, tau);
                    line_x.extend(&grid);
                    line_y.extend(grid.iter().map(|x| intercept + slope * x));
                    line_quantile.extend(std::iter::repeat_n(tau, grid.len()));
                    sizes.push(grid.len());
                }
            }
            Ok((
                vec![
                    (Variable::Aesthetic(Aesthetic::X), line_x),
                    (Variable::Aesthetic(Aesthetic::Y), line_y),
                    (Variable::Computed(Computed::Quantile), line_quantile),
                ],
                Some(sizes),
            ))
        })
    }
}

/// `(intercept, slope)` of the tau quantile regression line through the points.
fn fit(x: &[f64], y: &[f64], tau: f64) -> (f64, f64) {
    let mut residuals = vec![0.0; x.len()];
    let mut loss = |slope: f64| profile(x, y, tau, slope, &mut residuals);
    // Bracket the convex profile around the least squares slope, then shrink.
    let ordinary = least_squares_slope(x, y);
    let x_span = x.iter().copied().fold(f64::NEG_INFINITY, f64::max)
        - x.iter().copied().fold(f64::INFINITY, f64::min);
    let y_span = y.iter().copied().fold(f64::NEG_INFINITY, f64::max)
        - y.iter().copied().fold(f64::INFINITY, f64::min);
    let mut step = (y_span / x_span).abs().max(1e-12);
    let (mut low, mut high) = (ordinary - step, ordinary + step);
    for _ in 0..64 {
        if loss(low).0 > loss(low + step * 1e-6).0 {
            break;
        }
        low -= step;
        step *= 2.0;
    }
    for _ in 0..64 {
        if loss(high).0 > loss(high - step * 1e-6).0 {
            break;
        }
        high += step;
        step *= 2.0;
    }
    let ratio = (5f64.sqrt() - 1.0) / 2.0;
    for _ in 0..200 {
        if high - low <= 1e-14 * (1.0 + low.abs().max(high.abs())) {
            break;
        }
        let (first, second) = (high - ratio * (high - low), low + ratio * (high - low));
        if loss(first).0 <= loss(second).0 {
            high = second;
        } else {
            low = first;
        }
    }
    let slope = (low + high) / 2.0;
    (loss(slope).1, slope)
}

/// Check loss of the best line with `slope`, and its intercept: the tau quantile
/// of the residuals (R `type = 1`, the lower order statistic at n tau).
fn profile(x: &[f64], y: &[f64], tau: f64, slope: f64, residuals: &mut [f64]) -> (f64, f64) {
    for ((residual, x), y) in residuals.iter_mut().zip(x).zip(y) {
        *residual = y - slope * x;
    }
    let rank = ((residuals.len() as f64 * tau).ceil() as usize).clamp(1, residuals.len()) - 1;
    let (_, intercept, _) = residuals.select_nth_unstable_by(rank, f64::total_cmp);
    let intercept = *intercept;
    let loss = residuals
        .iter()
        .map(|residual| {
            let error = residual - intercept;
            if error < 0.0 {
                (tau - 1.0) * error
            } else {
                tau * error
            }
        })
        .sum();
    (loss, intercept)
}

/// The ordinary least squares slope (0 when x is constant).
fn least_squares_slope(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    let (mean_x, mean_y) = (x.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
    let (mut covariance, mut variance) = (0.0, 0.0);
    for (x, y) in x.iter().zip(y) {
        covariance += (x - mean_x) * (y - mean_y);
        variance += (x - mean_x) * (x - mean_x);
    }
    if variance > 0.0 {
        covariance / variance
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_line_matches_quantreg() {
        // quantreg::rq(y ~ x, tau = 0.5) on these four points: -1/3 + 4/3 x.
        let (intercept, slope) = fit(&[1.0, 2.0, 3.0, 4.0], &[1.0, 3.0, 2.0, 5.0], 0.5);
        assert!((slope - 4.0 / 3.0).abs() < 1e-9, "{slope}");
        assert!((intercept + 1.0 / 3.0).abs() < 1e-9, "{intercept}");
    }

    #[test]
    fn lines_on_exact_data_recover_it() {
        let x: Vec<f64> = (0..50).map(f64::from).collect();
        let y: Vec<f64> = x.iter().map(|x| 3.0 - 2.0 * x).collect();
        for tau in [0.1, 0.5, 0.9] {
            let (intercept, slope) = fit(&x, &y, tau);
            assert!((slope + 2.0).abs() < 1e-9 && (intercept - 3.0).abs() < 1e-7);
        }
    }

    #[test]
    fn higher_quantiles_sit_higher() {
        let x: Vec<f64> = (0..200).map(|index| f64::from(index) / 20.0).collect();
        let y: Vec<f64> = x
            .iter()
            .enumerate()
            .map(|(index, x)| x + ((index * 7919) % 101) as f64 / 50.0)
            .collect();
        let at = |tau: f64| {
            let (intercept, slope) = fit(&x, &y, tau);
            intercept + slope * 5.0
        };
        assert!(at(0.1) < at(0.5) && at(0.5) < at(0.9));
    }
}
