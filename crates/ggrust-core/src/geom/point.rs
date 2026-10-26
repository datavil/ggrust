//! `geom_point`: one marker per row.
//!
//! Sizes follow lets-plot: diameter = `size` x 2.2 px; default size 3, color
//! `#474747` (the theme "pen"), solid circle (shape 16). Other shapes come in 1.5.
//! `alpha`, when given, replaces the color's alpha (lets-plot semantics).

use super::{DrawLayer, Geom};
use crate::Result;
use crate::coord::Panel;
use crate::scene::{Color, MarkerShape, Node, PerItem, Points, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};

/// lets-plot `POINT_UNIT_SIZE`: pixels of diameter per unit of `size`.
pub const POINT_UNIT_SIZE: f64 = 2.2;

/// In lets-plot's rendered order (`GeomMeta.POINT`), which orders the legends.
const STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (Aesthetic::Size, StyleDefault::Number(3.0)),
    (
        Aesthetic::Color,
        StyleDefault::Color(Color::rgb(0x47, 0x47, 0x47)),
    ),
    // NaN: keep the color's own alpha (lets-plot: alpha replaces it only when set).
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
];

/// lets-plot point legend keys use size 5 unless `size` is mapped to that legend.
const LEGEND_DEFAULTS: &[(Aesthetic, StyleDefault)] =
    &[(Aesthetic::Size, StyleDefault::Number(5.0))];

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
        let (fill, size) = marker_styles(layer.styles, row_count)?;
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

    fn legend_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        LEGEND_DEFAULTS
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        // lets-plot `PointLegendKeyElementFactory`: shape size + stroke + 2.
        let diameter = styles.numbers(Aesthetic::Size)?.get(0) * POINT_UNIT_SIZE;
        let side = diameter + 2.0;
        Ok((side, side))
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        let (fill, size) = marker_styles(styles, 1)?;
        Ok(vec![Node::Points(Points {
            x: vec![cell.x + cell.width / 2.0],
            y: vec![cell.y + cell.height / 2.0],
            shape: PerItem::Uniform(MarkerShape::Circle),
            size,
            fill,
            stroke: PerItem::Uniform(None),
            stroke_width: PerItem::Uniform(0.0),
        })])
    }
}

/// Marker fill (color with alpha) and diameter of `count` items.
fn marker_styles(styles: &Styles, count: usize) -> Result<(PerItem<Option<Color>>, PerItem<f64>)> {
    let colors = styles.colors(Aesthetic::Color)?;
    let alphas = styles.numbers(Aesthetic::Alpha)?;
    let fill = match (colors, alphas) {
        (PerItem::Uniform(color), PerItem::Uniform(alpha)) => {
            PerItem::Uniform(Some(color.with_opacity(*alpha)))
        }
        _ => PerItem::Varying(
            (0..count)
                .map(|row| Some(colors.get(row).with_opacity(alphas.get(row))))
                .collect(),
        ),
    };
    let size = match styles.numbers(Aesthetic::Size)? {
        PerItem::Uniform(size) => PerItem::Uniform(size * POINT_UNIT_SIZE),
        PerItem::Varying(sizes) => {
            PerItem::Varying(sizes.iter().map(|size| size * POINT_UNIT_SIZE).collect())
        }
    };
    Ok((fill, size))
}
