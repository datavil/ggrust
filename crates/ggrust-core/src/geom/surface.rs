//! `geom_surface`: a filled surface over an x-y grid, under `coord_3d` only.
//!
//! Rows are the grid's vertices (a meshgrid flattened, any row order): the distinct
//! x and y values span the grid, and every cell whose four corners have a finite z
//! becomes one quad. Cells are drawn far to near (painter's algorithm on the mean
//! corner depth), so nearer cells hide farther ones within the layer. A cell's
//! fill is the mean of its corners' fills (matplotlib `plot_surface` colours a cell
//! by its mean z); its outline and alpha come from its lower-left corner. Default:
//! `fill` brush, a thin `color` paper outline (size 0.2), as a mesh.
//!
//! Neither lets-plot nor ggplot2 has it; see deviations.md. What it does not do:
//! shading by light direction, scattered (non-grid) data, cells split by depth
//! against other layers.

use super::lines;
use super::point::POINT_UNIT_SIZE;
use super::{DrawLayer, Geom};
use crate::coord::Panel;
use crate::coord3d::{Projector, Scales3d};
use crate::scene::{Color, Node, Path, PerItem, Rect, Rects};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};
use crate::{Error, Result};
use std::collections::BTreeMap;

const STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (Aesthetic::Color, StyleDefault::PAPER),
    (Aesthetic::Fill, StyleDefault::BRUSH),
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Size, StyleDefault::Number(0.2)),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
];

/// Coordinates closer than this fraction of their axis span (or magnitude) are
/// one grid line: values from different arithmetic differ by ~1e-16.
const GRID_TOLERANCE: f64 = 1e-9;

/// Filled surface over a grid.
pub struct SurfaceGeom;

impl Geom for SurfaceGeom {
    fn name(&self) -> &'static str {
        "surface"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y, Aesthetic::Z]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        STYLE_DEFAULTS
    }

    fn draw(&self, _layer: &DrawLayer, _panel: &Panel) -> Result<Vec<Node>> {
        // `build::space3d` draws surfaces with `draw_3d`; plots without
        // coord_3d are rejected before drawing.
        Err(Error::render(
            "geom_surface draws only under coord_3d(); add coord_3d() to the plot",
        ))
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        let side = styles.numbers(Aesthetic::Size)?.get(0).max(0.0) * POINT_UNIT_SIZE + 2.0;
        Ok((side, side))
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        let alpha = styles.numbers(Aesthetic::Alpha)?.get(0);
        let fill = styles.colors(Aesthetic::Fill)?.get(0).with_opacity(alpha);
        Ok(vec![Node::Rects(Rects {
            rects: vec![Rect {
                x: cell.x + 1.0,
                y: cell.y + 1.0,
                width: (cell.width - 2.0).max(0.0),
                height: (cell.height - 2.0).max(0.0),
            }],
            fill: PerItem::Uniform((fill.alpha > 0).then_some(fill)),
            stroke: lines::stroke_with_alpha(styles, 0, false)?,
        })])
    }
}

/// Sorted distinct finite values.
/// Sorted distinct finite values, near-equal ones merged (see [`GRID_TOLERANCE`]),
/// and the merge tolerance.
fn distinct(values: impl Iterator<Item = f64>) -> (Vec<f64>, f64) {
    let mut values: Vec<f64> = values.filter(|value| value.is_finite()).collect();
    values.sort_by(f64::total_cmp);
    let (Some(first), Some(last)) = (values.first().copied(), values.last().copied()) else {
        return (values, 0.0);
    };
    let scale = (last - first).max(first.abs()).max(last.abs());
    let tolerance = GRID_TOLERANCE * scale;
    values.dedup_by(|value, kept| *value - *kept <= tolerance);
    (values, tolerance)
}

/// Index of the grid line nearest `value`, if within `tolerance`.
fn line_of(lines: &[f64], tolerance: f64, value: f64) -> Option<usize> {
    let at = lines.partition_point(|line| *line < value);
    [at.checked_sub(1), Some(at)]
        .into_iter()
        .flatten()
        .filter(|&index| index < lines.len())
        .min_by(|&left, &right| {
            (lines[left] - value)
                .abs()
                .total_cmp(&(lines[right] - value).abs())
        })
        .filter(|&index| (lines[index] - value).abs() <= tolerance)
}

/// One cell of the surface, ready to draw.
struct Quad {
    /// Mean depth of its corners (larger is nearer).
    depth: f64,
    /// Projected corners.
    points: Vec<(f64, f64)>,
    fill: Color,
    /// The row whose outline it takes (lower-left corner).
    row: usize,
}

/// Mean of colors, channel by channel.
fn mean_color(colors: [Color; 4]) -> Color {
    let mean = |channel: fn(&Color) -> u8| {
        let sum: u32 = colors.iter().map(|color| u32::from(channel(color))).sum();
        ((sum + 2) / 4) as u8
    };
    Color {
        red: mean(|color| color.red),
        green: mean(|color| color.green),
        blue: mean(|color| color.blue),
        alpha: mean(|color| color.alpha),
    }
}

/// The surface's cells projected and ordered far to near. `path` names the layer
/// in errors.
pub fn draw_3d(
    layer: &DrawLayer,
    scales: &Scales3d,
    projector: &Projector,
    path: &str,
) -> Result<Vec<Node>> {
    let (Some(x), Some(y), Some(z)) = (
        layer.data.continuous(Aesthetic::X),
        layer.data.continuous(Aesthetic::Y),
        layer.data.continuous(Aesthetic::Z),
    ) else {
        return Ok(Vec::new());
    };
    let rows = layer.data.row_count();
    let value =
        |column: &crate::data::ContinuousColumn, row: usize| column.get(row).unwrap_or(f64::NAN);
    let (xs, x_tolerance) = distinct((0..rows).map(|row| value(x, row)));
    let (ys, y_tolerance) = distinct((0..rows).map(|row| value(y, row)));
    if xs.len() < 2 || ys.len() < 2 {
        return Ok(Vec::new());
    }
    // Grid vertex (line, column) -> row (the first row there). A map, not a dense
    // grid: memory follows the rows, also for sparse or scattered data.
    let mut vertex: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    for row in 0..rows {
        if !value(z, row).is_finite() {
            continue;
        }
        if let (Some(column), Some(line)) = (
            line_of(&xs, x_tolerance, value(x, row)),
            line_of(&ys, y_tolerance, value(y, row)),
        ) {
            vertex.entry((line, column)).or_insert(row);
        }
    }
    let fills = layer.styles.colors(Aesthetic::Fill)?;
    let alphas = layer.styles.numbers(Aesthetic::Alpha)?;
    let mut quads: Vec<Quad> = Vec::new();
    for &(line, column) in vertex.keys() {
        let corners = [
            (line, column),
            (line, column + 1),
            (line + 1, column + 1),
            (line + 1, column),
        ]
        .map(|key| vertex.get(&key).copied());
        let [Some(a), Some(b), Some(c), Some(d)] = corners else {
            continue;
        };
        let corner_rows = [a, b, c, d];
        let mut depth = 0.0;
        let mut points = Vec::with_capacity(4);
        for &row in &corner_rows {
            let (px, py, row_depth) = projector.project([
                scales.x.map(value(x, row)),
                scales.y.map(value(y, row)),
                scales.z.map(value(z, row)),
            ]);
            depth += row_depth / 4.0;
            points.push((px, py));
        }
        if !(depth.is_finite()
            && points
                .iter()
                .all(|(px, py)| px.is_finite() && py.is_finite()))
        {
            continue;
        }
        let fill = mean_color(corner_rows.map(|row| fills.get(row))).with_opacity(alphas.get(a));
        quads.push(Quad {
            depth,
            points,
            fill,
            row: a,
        });
    }
    if quads.is_empty() && vertex.len() >= 4 {
        // Points with z but not one complete cell: scattered data.
        return Err(Error::spec(
            path,
            format!(
                "geom_surface needs x and y on a grid (a flattened meshgrid), but no four of \
                 the {rows} rows ({} distinct x, {} distinct y) form a grid cell; interpolate \
                 the data onto a grid first, or use geom_point",
                xs.len(),
                ys.len()
            ),
        ));
    }
    quads.sort_by(|left, right| left.depth.total_cmp(&right.depth));
    quads
        .into_iter()
        .map(|quad| {
            Ok(Node::Path(Path {
                points: quad.points,
                closed: true,
                fill: (quad.fill.alpha > 0).then_some(quad.fill),
                stroke: lines::stroke_with_alpha(layer.styles, quad.row, false)?,
            }))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinct_values_are_sorted_and_finite() {
        let (values, _) = distinct([3.0, 1.0, f64::NAN, 3.0, 2.0].into_iter());
        assert_eq!(values, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn near_equal_coordinates_are_one_grid_line() {
        let (values, tolerance) = distinct([0.1, 0.30000000000000004, 0.3, 0.2].into_iter());
        assert_eq!(values.len(), 3);
        assert_eq!(line_of(&values, tolerance, 0.3), Some(2));
        assert_eq!(line_of(&values, tolerance, 0.30000000000000004), Some(2));
        assert_eq!(line_of(&values, tolerance, 0.25), None);
    }

    #[test]
    fn cell_fill_is_the_corner_mean() {
        let color = |red| Color {
            red,
            green: 0,
            blue: 255,
            alpha: 255,
        };
        let mean = mean_color([color(0), color(100), color(200), color(100)]);
        assert_eq!((mean.red, mean.blue, mean.alpha), (100, 255, 255));
    }
}
