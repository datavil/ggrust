//! `geom_point`: one marker per row.
//!
//! Follows lets-plot (`PointGeom`, `NamedShape`, `PointShapeSvg`): diameter =
//! `size` x 2.2 px (x 1.5 for the small shapes 18 and 20); shapes 0..14 are drawn
//! in `color`, 15..20 filled with `color`, 21..25 filled with `fill` and outlined
//! in `color`. Outlined shapes are `stroke` x 2.2 px wide and grow by it (by 2x for
//! `+`, `x` and stars, by sqrt 2 for diamonds). Defaults: size 3, stroke 0.75,
//! color `#474747`, fill white, shape 16 (solid circle). `alpha`, when given,
//! replaces the alpha of both fill and outline. A missing shape is a 1 px dot.
//!
//! What it does not do (yet): `angle`, `fatten`.

use super::{DrawLayer, Geom};
use crate::Result;
use crate::coord::Panel;
use crate::scene::{Color, MarkerShape, MarkerStyle, Node, PerItem, Points, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};

/// lets-plot `POINT_UNIT_SIZE`: pixels of diameter per unit of `size` (and `stroke`).
pub const POINT_UNIT_SIZE: f64 = 2.2;
/// lets-plot `AesScaling.circleDiameterSmaller`: pixels per unit for shapes 18, 20.
const SMALL_POINT_UNIT_SIZE: f64 = 1.5;

/// In lets-plot's rendered order (`GeomMeta.POINT`), which orders the legends.
const STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (Aesthetic::Size, StyleDefault::Number(3.0)),
    (Aesthetic::Stroke, StyleDefault::Number(0.75)),
    (
        Aesthetic::Color,
        StyleDefault::Color(Color::rgb(0x47, 0x47, 0x47)),
    ),
    (Aesthetic::Fill, StyleDefault::Color(Color::WHITE)),
    // NaN: keep the color's own alpha (lets-plot: alpha replaces it only when set).
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Shape, StyleDefault::Shape(16)),
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
        let mut points = markers(layer.styles, row_count, false)?;
        points.x = pixel_x;
        points.y = pixel_y;
        Ok(vec![Node::Points(points)])
    }

    fn legend_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        LEGEND_DEFAULTS
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        // lets-plot `PointLegendKeyElementFactory`: shape size + stroke + 2.
        let marker = Marker::of(styles, 0)?;
        let side = marker.size + marker.stroke_width + 2.0;
        Ok((side, side))
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        let mut points = markers(styles, 1, true)?;
        points.x = vec![cell.x + cell.width / 2.0];
        points.y = vec![cell.y + cell.height / 2.0];
        Ok(vec![Node::Points(points)])
    }
}

/// One marker's geometry and paint.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Marker {
    shape: MarkerShape,
    size: f64,
    fill: Option<Color>,
    stroke: Option<Color>,
    stroke_width: f64,
}

impl Marker {
    /// lets-plot `NamedShape.size` / `strokeWidth` and `AestheticsUtil.decorate`.
    fn of(styles: &Styles, row: usize) -> Result<Marker> {
        let code = styles.shapes(Aesthetic::Shape)?.get(row);
        let size = styles.numbers(Aesthetic::Size)?.get(row);
        let stroke = styles.numbers(Aesthetic::Stroke)?.get(row);
        let color = styles.colors(Aesthetic::Color)?.get(row);
        let fill = styles.colors(Aesthetic::Fill)?.get(row);
        let alpha = styles.numbers(Aesthetic::Alpha)?.get(row);
        if code == crate::scale::style::HIDDEN_SHAPE {
            return Ok(Marker {
                shape: MarkerShape::Dot,
                size: f64::NAN,
                fill: None,
                stroke: None,
                stroke_width: 0.0,
            });
        }
        let Some((shape, style)) = MarkerShape::from_code(code) else {
            // lets-plot `TinyPointShape`: a 1 px dot in the color.
            return Ok(Marker {
                shape: MarkerShape::Dot,
                size: 1.0,
                fill: Some(color.with_opacity(alpha)),
                stroke: None,
                stroke_width: 0.0,
            });
        };
        let unit = if matches!(code, 18 | 20) {
            SMALL_POINT_UNIT_SIZE
        } else {
            POINT_UNIT_SIZE
        };
        let stroke_width = match style {
            MarkerStyle::Solid => 0.0,
            _ => stroke * POINT_UNIT_SIZE,
        };
        let growth = match code {
            5 | 9 | 23 => 2f64.sqrt(),
            3 | 4 | 8 => 2.0,
            _ => 1.0,
        };
        let fill = match style {
            MarkerStyle::Hollow => None,
            MarkerStyle::Solid => Some(color.with_opacity(alpha)),
            MarkerStyle::Filled => Some(fill.with_opacity(alpha)),
        };
        Ok(Marker {
            shape,
            size: size * unit + growth * stroke_width,
            fill,
            stroke: (stroke_width > 0.0).then(|| color.with_opacity(alpha)),
            stroke_width,
        })
    }
}

/// Markers of `count` items without positions (the caller sets `x` and `y`); one
/// shared style when every style is uniform. Legend keys centre triangles in
/// their box.
fn markers(styles: &Styles, count: usize, key: bool) -> Result<Points> {
    let uniform = [
        Aesthetic::Shape,
        Aesthetic::Size,
        Aesthetic::Stroke,
        Aesthetic::Color,
        Aesthetic::Fill,
        Aesthetic::Alpha,
    ]
    .iter()
    .all(|aesthetic| styles.is_uniform(*aesthetic));
    let place = |marker: Marker| {
        if key {
            marker.shape.box_aligned()
        } else {
            marker.shape
        }
    };
    if uniform {
        let marker = Marker::of(styles, 0)?;
        return Ok(Points {
            x: Vec::new(),
            y: Vec::new(),
            shape: PerItem::Uniform(place(marker)),
            size: PerItem::Uniform(marker.size),
            fill: PerItem::Uniform(marker.fill),
            stroke: PerItem::Uniform(marker.stroke),
            stroke_width: PerItem::Uniform(marker.stroke_width),
        });
    }
    let markers = (0..count)
        .map(|row| Marker::of(styles, row))
        .collect::<Result<Vec<_>>>()?;
    Ok(Points {
        x: Vec::new(),
        y: Vec::new(),
        shape: PerItem::Varying(markers.iter().map(|marker| place(*marker)).collect()),
        size: PerItem::Varying(markers.iter().map(|marker| marker.size).collect()),
        fill: PerItem::Varying(markers.iter().map(|marker| marker.fill).collect()),
        stroke: PerItem::Varying(markers.iter().map(|marker| marker.stroke).collect()),
        stroke_width: PerItem::Varying(markers.iter().map(|marker| marker.stroke_width).collect()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::StyleValues;

    fn styles(values: &[(Aesthetic, StyleValues)]) -> Styles {
        let mut styles = Styles::default();
        for &(aesthetic, default) in STYLE_DEFAULTS {
            let value = values
                .iter()
                .find(|(known, _)| *known == aesthetic)
                .map_or_else(|| default.uniform(), |(_, value)| value.clone());
            styles.insert(aesthetic, value).unwrap();
        }
        styles
    }

    #[test]
    fn sizes_and_paint_match_lets_plot_svg() {
        // lets-plot 4.11: shape 21 -> r 4.125, stroke 1.65, fill white.
        let filled = Marker::of(
            &styles(&[(Aesthetic::Shape, StyleValues::Shapes(PerItem::Uniform(21)))]),
            0,
        )
        .unwrap();
        assert!((filled.size - 8.25).abs() < 1e-9);
        assert!((filled.stroke_width - 1.65).abs() < 1e-9);
        assert_eq!(filled.fill, Some(Color::WHITE));
        // Solid default: r 3.3, no outline.
        let solid = Marker::of(&styles(&[]), 0).unwrap();
        assert!((solid.size - 6.6).abs() < 1e-9);
        assert_eq!(solid.stroke, None);
        // Small diamond 18: 3 x 1.5 = 4.5.
        let small = Marker::of(
            &styles(&[(Aesthetic::Shape, StyleValues::Shapes(PerItem::Uniform(18)))]),
            0,
        )
        .unwrap();
        assert!((small.size - 4.5).abs() < 1e-9);
        // Cross 4 grows by 2 strokes: 6.6 + 3.3.
        let cross = Marker::of(
            &styles(&[(Aesthetic::Shape, StyleValues::Shapes(PerItem::Uniform(4)))]),
            0,
        )
        .unwrap();
        assert!((cross.size - 9.9).abs() < 1e-9);
        assert_eq!(cross.fill, None);
    }

    #[test]
    fn missing_shapes_are_dots() {
        let dot = Marker::of(
            &styles(&[(
                Aesthetic::Shape,
                StyleValues::Shapes(PerItem::Uniform(crate::scale::style::NA_SHAPE)),
            )]),
            0,
        )
        .unwrap();
        assert_eq!(dot.shape, MarkerShape::Dot);
    }
}
