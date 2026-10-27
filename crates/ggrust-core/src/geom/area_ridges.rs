//! `geom_area_ridges`: ridgeline densities, one per y, as lets-plot's
//! `AreaRidgesGeom`.
//!
//! Setup gives every point a top at `ymax = y + resolution(y) x scale x height`
//! (`ymin` = y, the baseline), so ridges rise into the row above. Ridges are drawn
//! from the top row down, each as a filled band (top line to baseline, simplified
//! by Douglas-Peucker at 0.25 px, `alpha` on the fill) and its top line (no
//! baseline). Points lower than `min_height` split a ridge. With fill or colour
//! varying along a ridge (`fill='..quantile..'`) each band is drawn on its own;
//! `quantile_lines` drops lines from the top to the baseline.
//!
//! What it does not do (yet): `orientation`.

use std::collections::HashMap;

use super::bar::{draw_key, key_size};
use super::lines;
use super::quantiles::{quantile_line_rows, split_by_band};
use super::{DrawLayer, Geom};
use crate::coord::Panel;
use crate::data::{Column, ContinuousColumn, LayerData, resolution};
use crate::parameters::Parameters;
use crate::scene::{Color, Node, Path, Rect};
use crate::spec::Aesthetic;
use crate::stat::ydensity_quantiles;
use crate::style::{StyleDefault, Styles};
use crate::{Error, Result};

/// lets-plot ridge style: pen outline, pen fill at 10% opacity, size 0.75.
const STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (
        Aesthetic::Color,
        StyleDefault::Color(Color::rgb(0x47, 0x47, 0x47)),
    ),
    (
        Aesthetic::Fill,
        StyleDefault::Color(Color {
            red: 0x47,
            green: 0x47,
            blue: 0x47,
            alpha: 26,
        }),
    ),
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Size, StyleDefault::Number(0.75)),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
];

/// lets-plot `PolylineSimplifier.DOUGLAS_PEUCKER_PIXEL_THRESHOLD`.
const SIMPLIFY_PIXELS: f64 = 0.25;

/// Ridgeline plots.
pub struct AreaRidgesGeom;

impl Geom for AreaRidgesGeom {
    fn name(&self) -> &'static str {
        "area_ridges"
    }

    fn default_stat(&self) -> &'static str {
        "densityridges"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y, Aesthetic::Height]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        STYLE_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["scale", "min_height", "quantile_lines", "quantiles"]
    }

    fn setup_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Ymin, Aesthetic::Ymax]
    }

    fn span_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Ymin, Aesthetic::Ymax]
    }

    fn setup(&self, mut data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        let (Some(y), Some(height)) = (
            data.continuous(Aesthetic::Y).cloned(),
            data.continuous(Aesthetic::Height).cloned(),
        ) else {
            return Ok(data);
        };
        let scale = parameters.number("scale")?.unwrap_or(1.0);
        let min_height = parameters.number("min_height")?.unwrap_or(0.0);
        let step = resolution(&y) * scale;
        let base = |row: usize| y.get(row).unwrap_or(f64::NAN);
        // lets-plot `heightSpan`: points below `min_height` span nothing.
        let top = |row: usize| match height.get(row) {
            Some(height) if height >= min_height => base(row) + step * height,
            _ => f64::NAN,
        };
        let continuous = |values| Column::Continuous(ContinuousColumn::from_values(values));
        let rows = 0..data.row_count();
        data.insert(
            Aesthetic::Ymin,
            continuous(rows.clone().map(base).collect()),
        )?;
        data.insert(Aesthetic::Ymax, continuous(rows.map(top).collect()))?;
        Ok(data)
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let min_height = layer.parameters.number("min_height")?.unwrap_or(0.0);
        let quantile_lines = layer.parameters.boolean("quantile_lines")?.unwrap_or(false);
        let quantiles = ydensity_quantiles(&layer.parameters)?;
        let column = |aesthetic| {
            layer.data.continuous(aesthetic).ok_or_else(|| {
                Error::InvalidData(format!("ridges are missing their '{aesthetic}' column"))
            })
        };
        let (x, y, height, top) = (
            column(Aesthetic::X)?,
            column(Aesthetic::Y)?,
            column(Aesthetic::Height)?,
            column(Aesthetic::Ymax)?,
        );
        let band = layer.data.continuous(Aesthetic::Quantile);
        let band_of = |row: usize| band.and_then(|band| band.get(row)).unwrap_or(f64::NAN);
        let finite =
            |column: &ContinuousColumn, row: usize| column.get(row).is_some_and(f64::is_finite);
        // Ridges by (y, group); drawn from the highest y down.
        let mut ridges: Vec<(f64, Vec<usize>)> = Vec::new();
        let mut slot_of: HashMap<(u64, u32), usize> = HashMap::new();
        for row in 0..layer.data.row_count() {
            if !(finite(x, row) && finite(y, row)) {
                continue;
            }
            let level = y.get(row).unwrap_or(0.0);
            let key = (level.to_bits(), layer.data.group_of(row));
            let slot = *slot_of.entry(key).or_insert_with(|| {
                ridges.push((level, Vec::new()));
                ridges.len() - 1
            });
            ridges[slot].1.push(row);
        }
        // Highest level first; stable, so groups keep their order within a level.
        ridges.sort_by(|left, right| right.0.total_cmp(&left.0));
        let top_of = |row: usize| panel.map(x.get(row).unwrap_or(0.0), top.get(row).unwrap_or(0.0));
        let base_of = |row: usize| panel.map(x.get(row).unwrap_or(0.0), y.get(row).unwrap_or(0.0));
        let fills = layer.styles.colors(Aesthetic::Fill)?;
        let colors = layer.styles.colors(Aesthetic::Color)?;
        let alphas = layer.styles.numbers(Aesthetic::Alpha)?;

        let mut nodes = Vec::new();
        // One y level at a time. lets-plot draws a level's groups from one merged
        // list: all their bands, then all top lines, then quantile lines. A level
        // with one group goes band, line per piece and quantile lines per run.
        let mut index = 0;
        while index < ridges.len() {
            let level = ridges[index].0;
            let shared = ridges
                .get(index + 1)
                .is_some_and(|next| next.0.to_bits() == level.to_bits());
            let (mut bands, mut tops, mut cuts) = (Vec::new(), Vec::new(), Vec::new());
            while index < ridges.len() && ridges[index].0.to_bits() == level.to_bits() {
                let rows = &mut ridges[index].1;
                index += 1;
                rows.sort_by(|left, right| {
                    x.get(*left)
                        .unwrap_or(0.0)
                        .total_cmp(&x.get(*right).unwrap_or(0.0))
                });
                // lets-plot `splitDataPoints`: runs of points at least `min_height`
                // high. Unlike lets-plot, per group: lets-plot merges the groups of a
                // level first, so one group's low points cut another's ridge.
                let mut runs: Vec<Vec<usize>> = vec![Vec::new()];
                for &row in rows.iter() {
                    if height.get(row).is_some_and(|value| value >= min_height) {
                        runs.last_mut().expect("a run").push(row);
                    } else if !runs.last().expect("a run").is_empty() {
                        runs.push(Vec::new());
                    }
                }
                for run in runs.iter().filter(|run| !run.is_empty()) {
                    let varies = run.iter().any(|&row| {
                        fills.get(row) != fills.get(run[0]) || colors.get(row) != colors.get(run[0])
                    });
                    let pieces = if varies {
                        split_by_band(run, band_of, x)
                    } else {
                        vec![run.clone()]
                    };
                    for piece in &pieces {
                        let first = piece[0];
                        let upper: Vec<(f64, f64)> = piece.iter().map(|&row| top_of(row)).collect();
                        let mut outline = upper.clone();
                        outline.extend(piece.iter().rev().map(|&row| base_of(row)));
                        let fill = fills.get(first).with_opacity(alphas.get(first));
                        bands.push(Node::Path(Path {
                            points: simplify(&outline, SIMPLIFY_PIXELS),
                            closed: true,
                            fill: (fill.alpha > 0).then_some(fill),
                            stroke: None,
                        }));
                        if let Some(stroke) = lines::stroke_with_alpha(layer.styles, first, false)?
                            && upper.len() > 1
                        {
                            tops.push(Node::Path(Path {
                                points: upper,
                                closed: false,
                                fill: None,
                                stroke: Some(stroke),
                            }));
                        }
                        if !shared {
                            nodes.append(&mut bands);
                            nodes.append(&mut tops);
                        }
                    }
                    // Quantile lines per run (lets-plot `buildRidge`).
                    if quantile_lines {
                        for row in quantile_line_rows(run, band_of, x, &quantiles) {
                            let Some(stroke) = lines::stroke_with_alpha(layer.styles, row, false)?
                            else {
                                continue;
                            };
                            cuts.push(Node::Path(Path {
                                points: vec![top_of(row), base_of(row)],
                                closed: false,
                                fill: None,
                                stroke: Some(stroke),
                            }));
                        }
                    }
                    if !shared {
                        nodes.append(&mut cuts);
                    }
                }
            }
            nodes.extend(bands);
            nodes.extend(tops);
            nodes.extend(cuts);
        }
        Ok(nodes)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        draw_key(styles, cell)
    }
}

/// lets-plot's Douglas-Peucker (`DouglasPeuckerSimplification` + `setWeightLimit`):
/// every point gets the distance at which the recursion picked it as its weight
/// (subdividing all the way down), then only weights above `tolerance` pixels stay.
fn simplify(points: &[(f64, f64)], tolerance: f64) -> Vec<(f64, f64)> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let last = points.len() - 1;
    let mut weights = vec![0.0; points.len()];
    weights[0] = f64::MAX;
    weights[last] = f64::MAX;
    let mut stack = vec![(0, last)];
    while let Some((start, end)) = stack.pop() {
        let (mut farthest, mut index) = (0.0, start);
        for middle in start + 1..end {
            let distance = segment_distance(points[middle], points[start], points[end]);
            if distance > farthest {
                (farthest, index) = (distance, middle);
            }
        }
        // Kotlin `Double.MIN_VALUE`: any positive distance subdivides further.
        if farthest >= f64::from_bits(1) {
            stack.push((start, index));
            stack.push((index, end));
            weights[index] = farthest;
        }
    }
    points
        .iter()
        .zip(weights)
        .filter_map(|(point, weight)| (weight > tolerance).then_some(*point))
        .collect()
}

/// Distance from `point` to the segment `start`..`end`.
fn segment_distance(point: (f64, f64), start: (f64, f64), end: (f64, f64)) -> f64 {
    let (dx, dy) = (end.0 - start.0, end.1 - start.1);
    let length = dx * dx + dy * dy;
    let t = if length > 0.0 {
        (((point.0 - start.0) * dx + (point.1 - start.1) * dy) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (nearest_x, nearest_y) = (start.0 + t * dx, start.1 + t * dy);
    ((point.0 - nearest_x).powi(2) + (point.1 - nearest_y).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simplify_drops_collinear_points() {
        let line = [(0.0, 0.0), (1.0, 0.0), (2.0, 0.1), (3.0, 0.0), (3.0, 5.0)];
        assert_eq!(simplify(&line, 0.25), [(0.0, 0.0), (3.0, 0.0), (3.0, 5.0)]);
        assert_eq!(simplify(&line, 0.05).len(), 4);
    }
}
