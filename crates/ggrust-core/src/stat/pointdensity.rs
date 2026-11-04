//! `stat="pointdensity"` (R package ggpointdensity): every point keeps its row
//! and gets `..density..`, the density of points around it (mapped to color by
//! default), and `..ndensity..` (density over the group's maximum).
//!
//! Per group, as ggpointdensity 0.2.1 behaves (GPL: the behaviour is measured and
//! reimplemented, no code is ported):
//! - `method="neighbors"`: the number of points, itself included, in an ellipse
//!   around the point: `dx^2 / s + dy^2 * s <= r2`, with `r2 = (w + h) / 70` and
//!   `s = w / h`, where `w` and `h` are the x and y ranges over every layer times
//!   `adjust`, plus R's special cases for equal coordinates and zero ranges (see
//!   [`neighbor_counts`]). Counted exactly on a grid, in parallel.
//! - `method="kde2d"`: `MASS::kde2d` on a 100 x 100 grid spanning the ranges, with
//!   bandwidths `max(sqrt(eps), MASS::bandwidth.nrd) * adjust` (kde2d's Gaussian
//!   sd is a quarter of that), read at the grid node below and left of the point
//!   (R `findInterval`), floored at machine epsilon. The grid is [`super::density2d`]'s.
//! - `method="auto"` (default): `kde2d` when a group has more than 20000 rows,
//!   else `neighbors`.
//!
//! A group with one finite point gets no density (NaN), as R's NA; rows with a
//! missing or infinite x or y get NaN too (R counts 0 for infinite ones).
//!
//! What it does not do: custom density functions or `method.args`, and the
//! "auto" choice is made per facet panel (R: per layer) without R's message.

use rayon::prelude::*;

use super::density::{Bandwidth, Kernel};
use super::density2d::Density2dOptions;
use super::groups::rows_by_group;
use super::{Stat, StatContext};
use crate::data::{Column, Computed, ContinuousColumn, LayerData};
use crate::parallel;
use crate::parameters::Parameters;
use crate::spec::Aesthetic;
use crate::{Error, Result};

/// Rows in a group above which `method="auto"` picks kde2d (ggpointdensity).
const AUTO_LIMIT: usize = 20_000;
/// `MASS::kde2d` grid points per axis (ggpointdensity's default `n`).
const KDE_N: usize = 100;
/// Grid cells per ellipse semi-axis: finer cells leave fewer points to test one
/// by one at the ellipse border.
const SUBDIVISIONS: f64 = 16.0;
/// Upper bound on grid cells per axis (the grid coarsens beyond it).
const MAX_CELLS_PER_AXIS: f64 = 2048.0;
/// Safety margin, in cells, against rounding in cell indices: cells are tested
/// as if this much larger (may contain) or smaller (fully inside). Large
/// coordinates add a few of their ulps (see [`Grid::new`]).
const MARGIN: f64 = 1e-6;
/// Query points per parallel task.
const CHUNK: usize = 2048;

/// Per-point neighbour counts or 2D kernel densities.
pub struct PointDensityStat;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Method {
    Auto,
    Neighbors,
    Kde2d,
}

const METHODS: [&str; 3] = ["auto", "neighbors", "kde2d"];

impl Stat for PointDensityStat {
    fn name(&self) -> &'static str {
        "pointdensity"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["method", "adjust"]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn default_mapping(&self) -> &'static [(Aesthetic, Computed)] {
        &[(Aesthetic::Color, Computed::Density)]
    }

    fn compute(
        &self,
        mut data: LayerData,
        parameters: &Parameters,
        context: &StatContext,
    ) -> Result<LayerData> {
        let (method, adjust) = options(parameters)?;
        let row_count = data.row_count();
        let mut density = vec![f64::NAN; row_count];
        let mut normalized = vec![f64::NAN; row_count];
        let ranges = match (data.continuous(Aesthetic::X), data.continuous(Aesthetic::Y)) {
            (Some(x), Some(y)) => context
                .x_range
                .or_else(|| super::bin::finite_range(x))
                .zip(context.y_range.or_else(|| super::bin::finite_range(y)))
                .map(|ranges| (x, y, ranges)),
            _ => None,
        };
        if let Some((x, y, (x_range, y_range))) = ranges {
            let groups = rows_by_group(&data);
            let method = match method {
                Method::Auto if groups.iter().any(|rows| rows.len() > AUTO_LIMIT) => Method::Kde2d,
                Method::Auto => Method::Neighbors,
                chosen => chosen,
            };
            for rows in groups {
                let (finite_rows, points): (Vec<usize>, Vec<(f64, f64)>) = rows
                    .iter()
                    .filter_map(|&row| {
                        let point = (x.get(row)?, y.get(row)?);
                        (point.0.is_finite() && point.1.is_finite()).then_some((row, point))
                    })
                    .unzip();
                if points.len() < 2 {
                    continue;
                }
                let values = match method {
                    Method::Kde2d => kde2d_densities(&points, x_range, y_range, adjust),
                    _ => neighbor_counts(
                        &points,
                        (x_range.1 - x_range.0) * adjust,
                        (y_range.1 - y_range.0) * adjust,
                    ),
                };
                let maximum = values.iter().copied().fold(f64::NAN, f64::max);
                for (row, value) in finite_rows.into_iter().zip(values) {
                    density[row] = value;
                    normalized[row] = value / maximum;
                }
            }
        }
        let continuous =
            |values: Vec<f64>| Column::Continuous(ContinuousColumn::from_values(values));
        data.insert(Computed::Density, continuous(density))?;
        data.insert(Computed::NormalizedDensity, continuous(normalized))?;
        Ok(data)
    }
}

/// `method` and `adjust`, validated.
fn options(parameters: &Parameters) -> Result<(Method, f64)> {
    let method = match parameters.value("method") {
        None => Method::Auto,
        Some(value) => match value.as_str() {
            Some("auto") => Method::Auto,
            Some("neighbors") => Method::Neighbors,
            Some("kde2d") => Method::Kde2d,
            given => {
                return Err(Error::spec(
                    parameters.path_of("method"),
                    format!(
                        "unknown point density method {value}; expected 'auto', 'neighbors' \
                         or 'kde2d'{}",
                        crate::suggest::did_you_mean(given.unwrap_or_default(), METHODS)
                    ),
                ));
            }
        },
    };
    let adjust = parameters.number("adjust")?.unwrap_or(1.0);
    if !(adjust.is_finite() && adjust > 0.0) {
        return Err(Error::spec(
            parameters.path_of("adjust"),
            format!(
                "adjust must be a positive number (it scales the neighbour radius or the \
                 bandwidth), got {adjust}; pass e.g. adjust=0.5"
            ),
        ));
    }
    Ok((method, adjust))
}

/// For every point, how many points j (itself included) ggpointdensity counts as
/// neighbours, with `r2 = (width + height) / 70`, `s = width / height`, `d = xi - xj`
/// (measured on its C counter, all in R's float order):
///
/// 1. `xj == xi && yj == yi`, or
/// 2. `xj == xi && s * dy * dy <= r2`, or
/// 3. `yj == yi && s * dx * dx <= r2` (`s`, not `1 / s`: wider than the ellipse
///    when `s < 1`, a quirk kept for parity), or
/// 4. `(1 / s) * dx * dx + s * dy * dy <= r2` (the ellipse).
///
/// With both ranges positive, rules 1 and 2 add nothing to 4, so the count is the
/// ellipse's, counted exactly on a grid (see [`Grid`]), plus rule 3's extra points
/// on the same y (see [`same_y_extra`]). A zero range makes R's arithmetic
/// NaN or infinite: zero width counts points on the same x or the same y, any
/// other degenerate case counts exact duplicates (see [`degenerate_counts`]).
fn neighbor_counts(points: &[(f64, f64)], width: f64, height: f64) -> Vec<f64> {
    let r2 = (width + height) / 70.0;
    let ratio = width / height;
    if !(r2 > 0.0 && r2.is_finite() && ratio > 0.0 && ratio.is_finite()) {
        return degenerate_counts(points, ratio == 0.0 && r2 >= 0.0);
    }
    let ellipse = Ellipse {
        r2,
        ratio,
        inverse: 1.0 / ratio,
    };
    let grid = Grid::new(points, (r2 * ratio).sqrt(), (r2 / ratio).sqrt());
    let order: Vec<usize> = (0..grid.xs.len()).collect();
    let chunks: Vec<&[usize]> = order.chunks(CHUNK).collect();
    let counts: Vec<Vec<u64>> =
        parallel::ordered_map(chunks, parallel::is_worth(points.len()), |chunk| {
            chunk
                .iter()
                .map(|&index| grid.count(&ellipse, grid.xs[index], grid.ys[index]))
                .collect()
        });
    // Counts are in cell order; give each point its own.
    let mut result = vec![0.0; points.len()];
    for (&point, count) in grid.point_of.iter().zip(counts.into_iter().flatten()) {
        result[point] = count as f64;
    }
    if ellipse.ratio < 1.0 {
        for (value, extra) in result.iter_mut().zip(same_y_extra(points, &ellipse)) {
            *value += extra as f64;
        }
    }
    result
}

/// Rule 3's points outside the ellipse: on the same y, with
/// `ratio * dx * dx <= r2` but not `(1 / ratio) * dx * dx <= r2`.
fn same_y_extra(points: &[(f64, f64)], ellipse: &Ellipse) -> Vec<u64> {
    let mut order: Vec<usize> = (0..points.len()).collect();
    order.sort_unstable_by(|&left, &right| {
        let (left, right) = (points[left], points[right]);
        left.1.total_cmp(&right.1).then(left.0.total_cmp(&right.0))
    });
    let mut extra = vec![0u64; points.len()];
    let mut start = 0;
    while start < order.len() {
        let y = points[order[start]].1;
        let end = start + order[start..].partition_point(|&index| points[index].1 == y);
        let end = end.max(start + 1);
        if end - start > 1 {
            let xs: Vec<f64> = order[start..end]
                .iter()
                .map(|&index| points[index].0)
                .collect();
            // Points within reach of xi form a run around it in sorted x.
            let within = |factor: f64, xi: f64, position: usize| -> usize {
                let near = |xj: &f64| {
                    let dx = xi - xj;
                    factor * dx * dx <= ellipse.r2
                };
                let below = xs[..position].partition_point(|xj| !near(xj));
                let above = position + xs[position..].partition_point(near);
                above - below
            };
            for (position, &index) in order[start..end].iter().enumerate() {
                let xi = xs[position];
                extra[index] = (within(ellipse.ratio, xi, position)
                    - within(ellipse.inverse, xi, position)) as u64;
            }
        }
        start = end;
    }
    extra
}

/// Counts under zero or undefined ranges: with `zero_width` (s = 0), rules 2 and
/// 3 hold for any point on the same x or the same y; otherwise (s infinite or
/// NaN) only rule 1, exact duplicates.
fn degenerate_counts(points: &[(f64, f64)], zero_width: bool) -> Vec<f64> {
    use std::collections::HashMap;
    // `+ 0.0` makes -0 equal 0, as `==` does.
    let key = |value: f64| (value + 0.0).to_bits();
    let tally = |keys: &mut dyn Iterator<Item = (u64, u64)>| {
        let mut counts: HashMap<(u64, u64), usize> = HashMap::new();
        for pair in keys {
            *counts.entry(pair).or_default() += 1;
        }
        counts
    };
    let pairs = tally(&mut points.iter().map(|&(x, y)| (key(x), key(y))));
    if !zero_width {
        return points
            .iter()
            .map(|&(x, y)| pairs[&(key(x), key(y))] as f64)
            .collect();
    }
    let same_x = tally(&mut points.iter().map(|&(x, _)| (key(x), 0)));
    let same_y = tally(&mut points.iter().map(|&(_, y)| (0, key(y))));
    points
        .iter()
        .map(|&(x, y)| {
            (same_x[&(key(x), 0)] + same_y[&(0, key(y))] - pairs[&(key(x), key(y))]) as f64
        })
        .collect()
}

/// The neighbour ellipse, rule 4: `(1 / ratio) * dx * dx + ratio * dy * dy <= r2`.
struct Ellipse {
    r2: f64,
    ratio: f64,
    /// `1 / ratio`, rounded once as R's counter does.
    inverse: f64,
}

impl Ellipse {
    fn contains(&self, dx: f64, dy: f64) -> bool {
        self.inverse * dx * dx + self.ratio * dy * dy <= self.r2
    }

    /// Half the ellipse's x extent at vertical offset `dy` (0 beyond it).
    fn half_width(&self, dy: f64) -> f64 {
        (self.ratio * (self.r2 - dy * dy * self.ratio))
            .max(0.0)
            .sqrt()
    }
}

/// Points binned into a regular grid, sorted by cell.
struct Grid {
    x0: f64,
    y0: f64,
    cell_width: f64,
    cell_height: f64,
    columns: usize,
    rows: usize,
    /// `starts[cell]..starts[cell + 1]`: the cell's points in `xs` / `ys`.
    starts: Vec<usize>,
    xs: Vec<f64>,
    ys: Vec<f64>,
    /// Input index of each sorted point.
    point_of: Vec<usize>,
    /// Rows a query must look at on each side of its own (columns follow
    /// from the ellipse's width in each row).
    reach: usize,
    /// Slack (x, y) in coordinate units for the cell tests.
    margins: (f64, f64),
}

impl Grid {
    fn new(points: &[(f64, f64)], semi_x: f64, semi_y: f64) -> Grid {
        let bounds = |coordinate: fn(&(f64, f64)) -> f64| {
            points
                .iter()
                .map(coordinate)
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), value| {
                    (low.min(value), high.max(value))
                })
        };
        let (x0, x1) = bounds(|point| point.0);
        let (y0, y1) = bounds(|point| point.1);
        // Cells a sixteenth of the semi-axes, but at most MAX_CELLS_PER_AXIS per
        // axis (counts stay exact on any grid; a coarse one tests more points).
        let size = |semi: f64, span: f64| (semi / SUBDIVISIONS).max(span / MAX_CELLS_PER_AXIS);
        let count = |span: f64, size: f64| -> usize {
            let cells = (span / size).floor() + 1.0;
            if cells.is_finite() && cells >= 1.0 {
                (cells as usize).min(MAX_CELLS_PER_AXIS as usize + 1)
            } else {
                1
            }
        };
        let (cell_width, cell_height) = (size(semi_x, x1 - x0), size(semi_y, y1 - y0));
        let (columns, rows) = (count(x1 - x0, cell_width), count(y1 - y0, cell_height));
        let index = |value: f64, origin: f64, size: f64, cells: usize| -> usize {
            (((value - origin) / size).floor().max(0.0) as usize).min(cells - 1)
        };
        let cell_of: Vec<usize> = points
            .iter()
            .map(|&(x, y)| {
                index(y, y0, cell_height, rows) * columns + index(x, x0, cell_width, columns)
            })
            .collect();
        let mut starts = vec![0usize; columns * rows + 1];
        for &cell in &cell_of {
            starts[cell + 1] += 1;
        }
        for cell in 0..columns * rows {
            starts[cell + 1] += starts[cell];
        }
        let mut next = starts.clone();
        let mut point_of = vec![0usize; points.len()];
        for (point, &cell) in cell_of.iter().enumerate() {
            point_of[next[cell]] = point;
            next[cell] += 1;
        }
        let xs = point_of.iter().map(|&point| points[point].0).collect();
        let ys = point_of.iter().map(|&point| points[point].1).collect();
        // Cell bounds `y0 + k * size` round by up to a few ulps of the coordinates:
        // a margin of that much (plus MARGIN cells) keeps the shortcuts exact.
        let margin = |size: f64, low: f64, high: f64| {
            MARGIN * size + 4.0 * f64::EPSILON * low.abs().max(high.abs())
        };
        let margins = (margin(cell_width, x0, x1), margin(cell_height, y0, y1));
        let steps = ((semi_y + margins.1) / cell_height).ceil() + 1.0;
        let reach = if steps.is_finite() && steps < rows as f64 {
            steps as usize
        } else {
            rows
        };
        Grid {
            x0,
            y0,
            cell_width,
            cell_height,
            columns,
            rows,
            starts,
            xs,
            ys,
            point_of,
            reach,
            margins,
        }
    }

    /// Points of the grid inside `ellipse` around `(x, y)`.
    fn count(&self, ellipse: &Ellipse, x: f64, y: f64) -> u64 {
        let (margin_x, margin_y) = self.margins;
        let row = (((y - self.y0) / self.cell_height).floor().max(0.0) as usize).min(self.rows - 1);
        let first_row = row.saturating_sub(self.reach);
        let last_row = (row + self.reach).min(self.rows - 1);
        let last_column = self.columns as f64 - 1.0;
        let mut total = 0u64;
        for grid_row in first_row..=last_row {
            let bottom = self.y0 + grid_row as f64 * self.cell_height - margin_y;
            let top = self.y0 + (grid_row + 1) as f64 * self.cell_height + margin_y;
            let nearest = if y < bottom {
                bottom - y
            } else if y > top {
                y - top
            } else {
                0.0
            };
            if nearest * nearest * ellipse.ratio > ellipse.r2 {
                continue;
            }
            let farthest = (y - bottom).abs().max((top - y).abs());
            // Columns that may hold a point inside, then those wholly inside.
            let outer = ellipse.half_width(nearest) + margin_x;
            let low = ((x - outer - self.x0) / self.cell_width).floor().max(0.0);
            let high = ((x + outer - self.x0) / self.cell_width)
                .floor()
                .min(last_column);
            if low > high {
                continue;
            }
            let (low, high) = (low as usize, high as usize);
            let line = grid_row * self.columns;
            let inner = ellipse.half_width(farthest) - margin_x;
            let full_low = ((x - inner - self.x0) / self.cell_width).ceil();
            let full_high = ((x + inner - self.x0) / self.cell_width).floor() - 1.0;
            let (full_low, full_high) = (full_low.max(low as f64), full_high.min(high as f64));
            if inner > 0.0 && full_low <= full_high {
                let (full_low, full_high) = (full_low as usize, full_high as usize);
                total += (self.starts[line + full_high + 1] - self.starts[line + full_low]) as u64;
                total += self.test(
                    ellipse,
                    x,
                    y,
                    self.starts[line + low],
                    self.starts[line + full_low],
                );
                total += self.test(
                    ellipse,
                    x,
                    y,
                    self.starts[line + full_high + 1],
                    self.starts[line + high + 1],
                );
                continue;
            }
            total += self.test(
                ellipse,
                x,
                y,
                self.starts[line + low],
                self.starts[line + high + 1],
            );
        }
        total
    }

    /// Points `start..end` (in cell order) inside `ellipse` around `(x, y)`.
    fn test(&self, ellipse: &Ellipse, x: f64, y: f64, start: usize, end: usize) -> u64 {
        self.xs[start..end]
            .iter()
            .zip(&self.ys[start..end])
            .filter(|&(&px, &py)| ellipse.contains(px - x, py - y))
            .count() as u64
    }
}

/// `MASS::kde2d` densities at each point, as ggpointdensity reads them.
fn kde2d_densities(
    points: &[(f64, f64)],
    x_range: (f64, f64),
    y_range: (f64, f64),
    adjust: f64,
) -> Vec<f64> {
    let bandwidth = |coordinate: fn(&(f64, f64)) -> f64| {
        let mut values: Vec<f64> = points.iter().map(coordinate).collect();
        values.sort_by(f64::total_cmp);
        // kde2d's Gaussian sd is a quarter of its `h`.
        mass_bandwidth_nrd(&values).max(f64::EPSILON.sqrt()) * adjust / 4.0
    };
    let options = Density2dOptions {
        kernel: Kernel::Gaussian,
        adjust: 1.0,
        x_bandwidth: Bandwidth::Fixed(bandwidth(|point| point.0)),
        y_bandwidth: Bandwidth::Fixed(bandwidth(|point| point.1)),
        x_count: KDE_N,
        y_count: KDE_N,
    };
    let x_steps = r_sequence(x_range.0, x_range.1, KDE_N);
    let y_steps = r_sequence(y_range.0, y_range.1, KDE_N);
    let weighted: Vec<(f64, f64, f64)> = points.iter().map(|&(x, y)| (x, y, 1.0)).collect();
    // Row-major, row = y step.
    let grid = options.density(&weighted, &x_steps, &y_steps);
    // R `findInterval`: the last grid value at or below the point.
    let below =
        |steps: &[f64], value: f64| steps.partition_point(|step| *step <= value).checked_sub(1);
    points
        .par_iter()
        .with_min_len(CHUNK)
        .map(|&(x, y)| match (below(&x_steps, x), below(&y_steps, y)) {
            (Some(column), Some(row)) => grid[row * KDE_N + column].max(f64::EPSILON),
            _ => f64::NAN,
        })
        .collect()
}

/// `MASS::bandwidth.nrd`: `4 * 1.06 * min(sd, IQR / 1.34) * n^(-1/5)` of sorted
/// values (quantile type 7), with no fallback for a zero spread.
fn mass_bandwidth_nrd(sorted: &[f64]) -> f64 {
    let count = sorted.len() as f64;
    let mean = sorted.iter().sum::<f64>() / count;
    let variance = sorted
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (count - 1.0);
    let spread =
        (super::boxplot::quantile(sorted, 0.75) - super::boxplot::quantile(sorted, 0.25)) / 1.34;
    4.0 * 1.06 * variance.sqrt().min(spread) * count.powf(-0.2)
}

/// R `seq(from, to, length.out = n)`: the ends exact, `from + i * by` between.
fn r_sequence(from: f64, to: f64, n: usize) -> Vec<f64> {
    let by = (to - from) / (n - 1) as f64;
    (0..n)
        .map(|index| match index {
            0 => from,
            last if last == n - 1 => to,
            _ => from + index as f64 * by,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ggpointdensity's four rules (see [`neighbor_counts`]) checked for every pair.
    fn brute_force(points: &[(f64, f64)], width: f64, height: f64) -> Vec<f64> {
        let r2 = (width + height) / 70.0;
        let ratio = width / height;
        let inverse = 1.0 / ratio;
        points
            .iter()
            .map(|&(xi, yi)| {
                points
                    .iter()
                    .filter(|&&(xj, yj)| {
                        let (dx, dy) = (xi - xj, yi - yj);
                        (xi == xj && yi == yj)
                            || (xi == xj && ratio * dy * dy <= r2)
                            || (yi == yj && ratio * dx * dx <= r2)
                            || (inverse * dx * dx + ratio * dy * dy <= r2)
                    })
                    .count() as f64
            })
            .collect()
    }

    /// Deterministic pseudo-random points: clusters plus a uniform background.
    fn sample(count: usize, seed: u64) -> Vec<(f64, f64)> {
        let mut state = seed;
        let mut next = move || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 11) as f64 / (1u64 << 53) as f64
        };
        (0..count)
            .map(|index| match index % 3 {
                0 => (next() * 10.0, next() * 4.0),
                _ => (2.0 + next() * next(), 1.0 + next() * 0.3),
            })
            .collect()
    }

    #[test]
    fn grid_counts_match_brute_force() {
        for (count, seed) in [(500, 1), (3000, 7), (20_011, 3)] {
            let points = sample(count, seed);
            for (width, height) in [
                (10.0, 4.0),
                (0.5, 0.2),
                (100.0, 4.0),
                (10.0, 4000.0),
                (1.0, 10.0),
            ] {
                assert_eq!(
                    neighbor_counts(&points, width, height),
                    brute_force(&points, width, height),
                    "{count} points, ranges {width} x {height}"
                );
            }
        }
    }

    #[test]
    fn tied_coordinates_follow_r_rules() {
        // Rounded coordinates: many points share a y (rule 3) or an x (rule 2).
        let points: Vec<(f64, f64)> = sample(4000, 5)
            .into_iter()
            .map(|(x, y)| ((x * 4.0).round() / 4.0, (y * 10.0).round() / 10.0))
            .collect();
        for (width, height) in [(10.0, 4.0), (1.0, 10.0), (0.3, 40.0), (10.0, 4000.0)] {
            assert_eq!(
                neighbor_counts(&points, width, height),
                brute_force(&points, width, height),
                "ranges {width} x {height}"
            );
        }
    }

    #[test]
    fn huge_offsets_stay_exact() {
        for (offset_x, offset_y, scale_x, scale_y) in [
            (1e15, 0.0, 0.06, 1.0),
            (1e12, 1e12, 7e-4, 1e4),
            (1e11, -1e11, 1e-5, 1e4),
        ] {
            let points: Vec<(f64, f64)> = sample(1500, 9)
                .into_iter()
                .map(|(x, y)| (offset_x + x * scale_x, offset_y + y * scale_y))
                .collect();
            let (width, height) = (10.0 * scale_x, 4.0 * scale_y);
            assert_eq!(
                neighbor_counts(&points, width, height),
                brute_force(&points, width, height),
                "offsets {offset_x}, {offset_y}"
            );
        }
    }

    #[test]
    fn coarse_grids_stay_exact() {
        // A tiny radius against a wide spread: the grid hits MAX_CELLS_PER_AXIS.
        let mut points = sample(2000, 11);
        points.push((1e7, -1e7));
        assert_eq!(
            neighbor_counts(&points, 1e-6, 1e-6),
            brute_force(&points, 1e-6, 1e-6)
        );
        // Duplicates and points exactly on the ellipse border.
        let points = vec![(0.0, 0.0), (0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (0.0, 1.0)];
        let (width, height) = (35.0, 35.0); // r2 = 1, ratio = 1
        assert_eq!(
            neighbor_counts(&points, width, height),
            vec![4.0, 4.0, 4.0, 2.0, 3.0]
        );
    }

    #[test]
    fn neighbor_rule_matches_ggpointdensity() {
        // Rscript, ggpointdensity 0.2.1 count_neighbors(x, y, r2, xy):
        // count_neighbors(c(0, 1, 2), c(0, 0, 0), r2 = 1, xy = 1) = 2 3 2 (border counts);
        // count_neighbors(c(0, 0.9), c(0, 0), r2 = 0.85, xy = 1) = 2 2 (r2 is squared);
        // count_neighbors(c(0, 1), c(0, 0), r2 = 1.5, xy = 2) = 2 2 (dx^2 / xy);
        // count_neighbors(c(0, 0), c(0, 1), r2 = 1.5, xy = 2) = 1 1 (dy^2 * xy).
        let ellipse = |r2: f64, ratio: f64| Ellipse {
            r2,
            ratio,
            inverse: 1.0 / ratio,
        };
        assert!(ellipse(1.0, 1.0).contains(1.0, 0.0));
        assert!(ellipse(0.85, 1.0).contains(0.9, 0.0));
        assert!(ellipse(1.5, 2.0).contains(1.0, 0.0));
        assert!(!ellipse(1.5, 2.0).contains(0.0, 1.0));
    }

    #[test]
    fn degenerate_ranges_follow_r() {
        // Rscript, ggpointdensity 0.2.1 geom_pointdensity(method = "neighbors"):
        // x = c(1, 2, 3, 4, 10), y = 5 gives 1 1 1 1 1 (only duplicates);
        let points = vec![(1.0, 5.0), (2.0, 5.0), (3.0, 5.0), (4.0, 5.0), (10.0, 5.0)];
        assert_eq!(neighbor_counts(&points, 9.0, 0.0), vec![1.0; 5]);
        // x = c(1, 1, 1), y = 5 gives 3 3 3;
        let points = vec![(1.0, 5.0); 3];
        assert_eq!(neighbor_counts(&points, 0.0, 0.0), vec![3.0; 3]);
        // count_neighbors(c(0, 1, 5), c(0, 0, 0), r2 = 1, xy = 0) = 3 3 3 (same y);
        // zero width: same x or same y counts.
        let points = vec![(0.0, 0.0), (0.0, 2.0), (1.0, 0.0), (3.0, 3.0)];
        assert_eq!(neighbor_counts(&points, 0.0, 3.0), vec![3.0, 2.0, 2.0, 1.0]);
        assert_eq!(
            neighbor_counts(&points, 0.0, 3.0),
            brute_force(&points, 0.0, 3.0)
        );
        // Zero height or both zero: duplicates only, also for -0.
        let points = vec![(0.0, 1.0), (-0.0, 1.0), (1.0, 1.0)];
        assert_eq!(neighbor_counts(&points, 1.0, 0.0), vec![2.0, 2.0, 1.0]);
        assert_eq!(
            neighbor_counts(&points, 1.0, 0.0),
            brute_force(&points, 1.0, 0.0)
        );
    }

    #[test]
    fn r_sequence_keeps_both_ends() {
        let steps = r_sequence(0.1, 0.7, 100);
        assert_eq!((steps[0], steps[99]), (0.1, 0.7));
        assert!((steps[50] - (0.1 + 50.0 * 0.6 / 99.0)).abs() < 1e-15);
    }

    #[test]
    fn mass_bandwidth_follows_r() {
        // Rscript: MASS::bandwidth.nrd(c(1, 2, 3, 4, 5, 6, 9)) = 6.43224687944.
        let values = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 9.0];
        assert!((mass_bandwidth_nrd(&values) - 6.432_246_879_44).abs() < 1e-9);
        // Zero IQR: zero, no fallback (ggpointdensity floors it at sqrt(eps)).
        assert_eq!(mass_bandwidth_nrd(&[1.0, 1.0, 1.0, 1.0, 5.0]), 0.0);
    }

    fn parse(options: &str) -> Result<(Method, f64)> {
        let options: crate::spec::Options = serde_json::from_str(options).unwrap();
        super::options(&Parameters::new(&options, "layers[0]"))
    }

    #[test]
    fn options_report_bad_values() {
        assert_eq!(parse("{}").unwrap(), (Method::Auto, 1.0));
        assert_eq!(
            parse(r#"{"method": "kde2d", "adjust": 2}"#).unwrap(),
            (Method::Kde2d, 2.0)
        );
        let error = parse(r#"{"method": "neighbours"}"#)
            .unwrap_err()
            .to_string();
        assert!(error.contains("layers[0].method"), "{error}");
        assert!(error.contains("did you mean 'neighbors'"), "{error}");
        let error = parse(r#"{"adjust": 0}"#).unwrap_err().to_string();
        assert!(error.contains("layers[0].adjust"), "{error}");
        assert!(error.contains("positive"), "{error}");
    }
}
