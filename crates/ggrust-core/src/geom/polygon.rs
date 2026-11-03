//! `geom_polygon`: one closed shape per group, vertices in data order.
//!
//! Fill and outline come from the group's first row. `alpha` replaces the alpha
//! of the fill only (lets-plot leaves the outline opaque).
//!
//! What it does not do (yet): rings or holes in `geom_polygon` (contour bands
//! use [`draw_polygons`] with `split_rings`), maps (`map`, `use_crs`), `color_by`/`fill_by`.

use super::lines;
use super::point::POINT_UNIT_SIZE;
use super::{DrawLayer, Geom};
use crate::Result;
use crate::coord::Panel;
use crate::scene::{Node, Path, PerItem, Rect, Rects};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};

/// lets-plot polygon style: blue fill, white outline.
const STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (Aesthetic::Color, StyleDefault::PAPER),
    (Aesthetic::Fill, StyleDefault::BRUSH),
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Size, StyleDefault::Number(0.75)),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
];

/// Filled polygons.
pub struct PolygonGeom;

impl Geom for PolygonGeom {
    fn name(&self) -> &'static str {
        "polygon"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        STYLE_DEFAULTS
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        draw_polygons(layer, panel, false)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        let side = styles.numbers(Aesthetic::Size)?.get(0).max(0.0) * POINT_UNIT_SIZE + 2.0;
        Ok((side, side))
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        let alpha = styles.numbers(Aesthetic::Alpha)?.get(0);
        let fill = styles.colors(Aesthetic::Fill)?.get(0).with_opacity(alpha);
        Ok(vec![Node::Rects(Rects {
            // Inset by a pixel so neighbouring keys do not touch (as lets-plot).
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

/// One closed shape per group. With `split_rings`, a group is several rings one
/// after another, each ending on its first vertex (lets-plot multi-ring polygons:
/// contour bands with holes), drawn as one path with the nonzero rule.
pub(super) fn draw_polygons(
    layer: &DrawLayer,
    panel: &Panel,
    split_rings: bool,
) -> Result<Vec<Node>> {
    let (Some(x), Some(y)) = (
        layer.data.continuous(Aesthetic::X),
        layer.data.continuous(Aesthetic::Y),
    ) else {
        return Ok(Vec::new());
    };
    let group_index = layer.data.group_index();
    let group_count = group_index
        .map(|groups| {
            groups
                .iter()
                .copied()
                .max()
                .map_or(0, |last| last as usize + 1)
        })
        .unwrap_or(1);
    let mut rows_by_group = vec![Vec::new(); group_count];
    for row in 0..layer.data.row_count() {
        let group = group_index.map_or(0, |groups| groups[row] as usize);
        rows_by_group[group].push(row);
    }
    let mut nodes = Vec::new();
    for rows in rows_by_group {
        let Some(&first) = rows.first() else { continue };
        let location = |row: usize| {
            (
                x.get(row).unwrap_or(f64::NAN),
                y.get(row).unwrap_or(f64::NAN),
            )
        };
        let mut points: Vec<(f64, f64)> = Vec::with_capacity(rows.len());
        let mut ring_start: Option<(f64, f64)> = None;
        for &row in &rows {
            let (row_x, row_y) = location(row);
            let (px, py) = panel.map(row_x, row_y);
            // A missing vertex is dropped, as the shape stays one ring.
            if !(px.is_finite() && py.is_finite()) {
                continue;
            }
            points.push((px, py));
            if !split_rings {
                continue;
            }
            match ring_start {
                None => ring_start = Some((row_x, row_y)),
                Some(start) if start == (row_x, row_y) => {
                    // The ring is closed: a gap starts the next one.
                    points.push((f64::NAN, f64::NAN));
                    ring_start = None;
                }
                Some(_) => {}
            }
        }
        if points.last().is_some_and(|(px, _)| px.is_nan()) {
            points.pop();
        }
        if points.iter().filter(|(px, _)| px.is_finite()).count() < 2 {
            continue;
        }
        let alpha = layer.styles.numbers(Aesthetic::Alpha)?.get(first);
        let fill = layer
            .styles
            .colors(Aesthetic::Fill)?
            .get(first)
            .with_opacity(alpha);
        nodes.push(Node::Path(Path {
            points,
            closed: true,
            fill: (fill.alpha > 0).then_some(fill),
            stroke: lines::stroke_with_alpha(layer.styles, first, false)?,
        }));
    }
    Ok(nodes)
}
