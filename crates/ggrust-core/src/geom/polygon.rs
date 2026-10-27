//! `geom_polygon`: one closed shape per group, vertices in data order.
//!
//! Fill and outline come from the group's first row. `alpha` replaces the alpha
//! of the fill only (lets-plot leaves the outline opaque).
//!
//! What it does not do (yet): holes, maps (`map`, `use_crs`), `color_by`/`fill_by`.

use super::lines;
use super::point::POINT_UNIT_SIZE;
use super::{DrawLayer, Geom};
use crate::Result;
use crate::coord::Panel;
use crate::scene::{Color, Node, Path, PerItem, Rect, Rects};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};

/// lets-plot polygon style: blue fill, white outline.
const STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (Aesthetic::Color, StyleDefault::Color(Color::WHITE)),
    (
        Aesthetic::Fill,
        StyleDefault::Color(Color::rgb(0x11, 0x8e, 0xd8)),
    ),
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
            // A missing vertex is dropped, as the shape stays one ring.
            let points: Vec<(f64, f64)> = rows
                .iter()
                .map(|&row| {
                    panel.map(
                        x.get(row).unwrap_or(f64::NAN),
                        y.get(row).unwrap_or(f64::NAN),
                    )
                })
                .filter(|(px, py)| px.is_finite() && py.is_finite())
                .collect();
            if points.len() < 2 {
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
