//! `geom_point`: one marker per row.
//!
//! Sizes follow lets-plot: diameter = `size` x 2.2 px; default size 3, color
//! `#474747` (the theme "pen"), solid circle (shape 16). Other shapes come in 1.5.

use super::{DrawLayer, Geom, StyleDefault};
use crate::Result;
use crate::coord::Panel;
use crate::scene::{Color, MarkerShape, Node, PerItem, Points};
use crate::spec::Aesthetic;

/// lets-plot `POINT_UNIT_SIZE`: pixels of diameter per unit of `size`.
pub const POINT_UNIT_SIZE: f64 = 2.2;

const STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (
        Aesthetic::Color,
        StyleDefault::Color(Color::rgb(0x47, 0x47, 0x47)),
    ),
    (Aesthetic::Size, StyleDefault::Number(3.0)),
    (Aesthetic::Alpha, StyleDefault::Number(1.0)),
];

/// Scatter points.
pub struct PointGeom;

impl Geom for PointGeom {
    fn name(&self) -> &'static str {
        "point"
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
        let row_count = layer.data.row_count();
        let mut pixel_x = Vec::with_capacity(row_count);
        let mut pixel_y = Vec::with_capacity(row_count);
        for row in 0..row_count {
            let value_x = x.get(row).unwrap_or(f64::NAN);
            let value_y = y.get(row).unwrap_or(f64::NAN);
            let (px, py) = panel.map(value_x, value_y);
            pixel_x.push(px);
            pixel_y.push(py);
        }
        let colors = layer.colors(Aesthetic::Color);
        let alphas = layer.numbers(Aesthetic::Alpha);
        let fill = match (&colors, &alphas) {
            (PerItem::Uniform(color), PerItem::Uniform(alpha)) => {
                PerItem::Uniform(Some(color.with_opacity(*alpha)))
            }
            _ => PerItem::Varying(
                (0..row_count)
                    .map(|row| Some(colors.get(row).with_opacity(alphas.get(row))))
                    .collect(),
            ),
        };
        let size = match layer.numbers(Aesthetic::Size) {
            PerItem::Uniform(size) => PerItem::Uniform(size * POINT_UNIT_SIZE),
            PerItem::Varying(sizes) => {
                PerItem::Varying(sizes.iter().map(|size| size * POINT_UNIT_SIZE).collect())
            }
        };
        Ok(vec![Node::Points(Points {
            x: pixel_x,
            y: pixel_y,
            shape: PerItem::Uniform(MarkerShape::Circle),
            size,
            fill,
            stroke: PerItem::Uniform(None),
            stroke_width: PerItem::Uniform(0.0),
        })])
    }
}
