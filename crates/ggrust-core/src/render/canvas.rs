//! Dense point batches for the HTML canvas (plan 4.4): a batch with more than
//! [`CANVAS_THRESHOLD`] drawable markers is written to the SVG as an empty
//! `<foreignObject class="ggrust-canvas">` placeholder (same place in the
//! document, so z-order and clipping hold) and its markers travel as base64
//! typed arrays that the runtime (`js/ggrust.js`) stamps on a canvas.
//!
//! Payload of one batch: `xy` interleaved Float32 centres in scene pixels,
//! `styles` the distinct marker styles (size, paints and the marker parts drawn
//! around (0, 0), from [`MarkerShape::for_each_part`], so canvas and SVG markers
//! have the same geometry), and `style` one index per marker (`u16`, or `u32`
//! past 65536 styles; absent with a single style). Markers are kept in data
//! order. Static SVG and PNG never use this.

use std::collections::HashMap;

use serde_json::{Value, json};

use super::svg::{marker_stroke, push_base64};
use crate::scene::geometry::{ClipBox, is_drawable_marker};
use crate::scene::{Color, MarkerPart, MarkerShape, PerItem, Points};

/// Batches with more drawable markers than this go to the canvas.
pub(super) const CANVAS_THRESHOLD: usize = 50_000;

/// The style of one marker: everything but its position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct StyleKey {
    shape: MarkerShape,
    size: u64,
    fill: Option<Color>,
    /// Visible outline: color and width bits (see `svg::marker_stroke`).
    stroke: Option<(Color, u64)>,
    /// Outline width as given to the shape geometry (triangles depend on it).
    stroke_width: u64,
}

/// Whether `points` has more drawable markers inside `clip_box` than the threshold.
pub(super) fn is_dense(points: &Points, clip_box: &ClipBox) -> bool {
    points.len() > CANVAS_THRESHOLD && drawable(points, clip_box).nth(CANVAS_THRESHOLD).is_some()
}

/// Indices of the markers the SVG writer would draw.
fn drawable<'a>(points: &'a Points, clip_box: &'a ClipBox) -> impl Iterator<Item = usize> + 'a {
    (0..points.len()).filter(move |&index| {
        let (x, y) = (points.x[index], points.y[index]);
        is_drawable_marker(x, y, points.size.get(index)) && clip_box.contains(x, y)
    })
}

/// The canvas payload of a dense batch (see the module docs).
pub(super) fn encode(points: &Points, clip_box: &ClipBox) -> Value {
    let uniform = matches!(
        (
            &points.shape,
            &points.size,
            &points.fill,
            &points.stroke,
            &points.stroke_width
        ),
        (
            PerItem::Uniform(_),
            PerItem::Uniform(_),
            PerItem::Uniform(_),
            PerItem::Uniform(_),
            PerItem::Uniform(_)
        )
    );
    let mut xy: Vec<u8> = Vec::with_capacity(points.len() * 8);
    let mut keys: Vec<StyleKey> = Vec::new();
    let mut lookup: HashMap<StyleKey, u32> = HashMap::new();
    let mut indices: Vec<u32> = Vec::new();
    for index in drawable(points, clip_box) {
        xy.extend_from_slice(&(points.x[index] as f32).to_le_bytes());
        xy.extend_from_slice(&(points.y[index] as f32).to_le_bytes());
        if uniform && !keys.is_empty() {
            continue;
        }
        let key = style_key(points, index);
        let next = keys.len() as u32;
        let style = *lookup.entry(key).or_insert_with(|| {
            keys.push(key);
            next
        });
        if !uniform {
            indices.push(style);
        }
    }
    let mut xy_text = String::new();
    push_base64(&mut xy_text, &xy);
    let mut batch = json!({
        "count": xy.len() / 8,
        "xy": xy_text,
        "styles": keys.iter().map(style_json).collect::<Vec<_>>(),
    });
    if keys.len() > 1 {
        let mut bytes = Vec::new();
        let wide = keys.len() > usize::from(u16::MAX) + 1;
        for style in indices {
            if wide {
                bytes.extend_from_slice(&style.to_le_bytes());
            } else {
                bytes.extend_from_slice(&(style as u16).to_le_bytes());
            }
        }
        let mut text = String::new();
        push_base64(&mut text, &bytes);
        batch["style"] = Value::String(text);
        batch["style_type"] = Value::from(if wide { "u32" } else { "u16" });
    }
    batch
}

fn style_key(points: &Points, index: usize) -> StyleKey {
    let stroke_width = points.stroke_width.get(index);
    StyleKey {
        shape: points.shape.get(index),
        size: points.size.get(index).to_bits(),
        fill: points.fill.get(index),
        stroke: marker_stroke(points.stroke.get(index), stroke_width)
            .map(|(color, width)| (color, width.to_bits())),
        stroke_width: stroke_width.to_bits(),
    }
}

/// `{size, fill, stroke, width, parts}`; parts are `["c", x, y, r]`,
/// `["r", x, y, w, h]`, `["p", x0, y0, x1, y1, ...]` and `["l", x1, y1, x2, y2]`.
fn style_json(key: &StyleKey) -> Value {
    let size = f64::from_bits(key.size);
    let mut parts = Vec::new();
    key.shape
        .for_each_part(0.0, 0.0, size, f64::from_bits(key.stroke_width), |part| {
            parts.push(match part {
                MarkerPart::Circle { x, y, radius } => json!(["c", x, y, radius]),
                MarkerPart::Rect(rect) => json!(["r", rect.x, rect.y, rect.width, rect.height]),
                MarkerPart::Polygon(vertices) => {
                    let mut values = vec![json!("p")];
                    for (x, y) in vertices {
                        values.push(json!(x));
                        values.push(json!(y));
                    }
                    Value::Array(values)
                }
                MarkerPart::Line(start, end) => json!(["l", start.0, start.1, end.0, end.1]),
            });
        });
    json!({
        "size": size,
        "fill": key.fill.map(css_color),
        "stroke": key.stroke.map(|(color, _)| css_color(color)),
        "width": key.stroke.map_or(0.0, |(_, width)| f64::from_bits(width)),
        "parts": parts,
    })
}

/// A canvas paint, `null` when nothing is drawn (as the SVG writer's `none`).
fn css_color(color: Color) -> Value {
    if color.is_transparent() {
        return Value::Null;
    }
    Value::String(format!(
        "rgba({},{},{},{})",
        color.red,
        color.green,
        color.blue,
        color.opacity()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn batch(count: usize, fill: PerItem<Option<Color>>) -> Points {
        Points {
            x: (0..count).map(|index| (index % 500) as f64).collect(),
            y: (0..count).map(|index| (index / 500) as f64).collect(),
            shape: PerItem::Uniform(MarkerShape::Circle),
            size: PerItem::Uniform(4.0),
            fill,
            stroke: PerItem::Uniform(None),
            stroke_width: PerItem::Uniform(0.0),
        }
    }

    fn decode(text: &str) -> Vec<u8> {
        let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut bytes = Vec::new();
        for chunk in text.as_bytes().chunks(4) {
            let mut value = 0u32;
            let mut valid = 0;
            for (position, &character) in chunk.iter().enumerate() {
                if character != b'=' {
                    let digit = alphabet.iter().position(|&a| a == character).unwrap() as u32;
                    value |= digit << (18 - 6 * position);
                    valid += 1;
                }
            }
            for position in 0..valid - 1 {
                bytes.push((value >> (16 - 8 * position)) as u8);
            }
        }
        bytes
    }

    #[test]
    fn density_counts_only_drawable_markers() {
        let clip = ClipBox::for_scene(1000.0, 1000.0);
        let mut points = batch(CANVAS_THRESHOLD + 1, PerItem::Uniform(Some(Color::BLACK)));
        assert!(is_dense(&points, &clip));
        points.x[0] = f64::NAN;
        assert!(!is_dense(&points, &clip));
        assert!(!is_dense(
            &batch(CANVAS_THRESHOLD, PerItem::Uniform(None)),
            &clip
        ));
    }

    #[test]
    fn uniform_batch_has_one_style_and_no_index() {
        let clip = ClipBox::for_scene(1000.0, 1000.0);
        let points = batch(3, PerItem::Uniform(Some(Color::rgba(255, 0, 0, 128))));
        let value = encode(&points, &clip);
        assert_eq!(value["count"], 3);
        assert!(value.get("style").is_none());
        assert_eq!(
            value["styles"][0]["fill"],
            "rgba(255,0,0,0.5019607843137255)"
        );
        assert_eq!(value["styles"][0]["parts"][0], json!(["c", 0.0, 0.0, 2.0]));
        let xy = decode(value["xy"].as_str().unwrap());
        let floats: Vec<f32> = xy
            .chunks(4)
            .map(|bytes| f32::from_le_bytes(bytes.try_into().unwrap()))
            .collect();
        assert_eq!(floats, vec![0.0, 0.0, 1.0, 0.0, 2.0, 0.0]);
    }

    #[test]
    fn varying_styles_are_indexed_in_data_order() {
        let clip = ClipBox::for_scene(1000.0, 1000.0);
        let red = Some(Color::rgb(255, 0, 0));
        let blue = Some(Color::rgb(0, 0, 255));
        let mut points = batch(4, PerItem::Varying(vec![red, blue, red, None]));
        points.y[3] = f64::INFINITY; // skipped, as in the SVG
        let value = encode(&points, &clip);
        assert_eq!(value["count"], 3);
        assert_eq!(value["style_type"], "u16");
        assert_eq!(value["styles"].as_array().unwrap().len(), 2);
        assert_eq!(
            decode(value["style"].as_str().unwrap()),
            vec![0, 0, 1, 0, 0, 0]
        );
    }

    #[test]
    fn parts_around_the_origin_translate_to_any_centre() {
        for code in 0..=25 {
            let Some((shape, _)) = MarkerShape::from_code(code) else {
                continue;
            };
            let at_origin = shape.parts(0.0, 0.0, 7.0, 1.5);
            let moved = shape.parts(10.0, 20.0, 7.0, 1.5);
            let shift = |part: &MarkerPart| match part.clone() {
                MarkerPart::Circle { x, y, radius } => MarkerPart::Circle {
                    x: x + 10.0,
                    y: y + 20.0,
                    radius,
                },
                MarkerPart::Rect(mut rect) => {
                    rect.x += 10.0;
                    rect.y += 20.0;
                    MarkerPart::Rect(rect)
                }
                MarkerPart::Polygon(vertices) => MarkerPart::Polygon(
                    vertices
                        .into_iter()
                        .map(|(x, y)| (x + 10.0, y + 20.0))
                        .collect(),
                ),
                MarkerPart::Line(a, b) => {
                    MarkerPart::Line((a.0 + 10.0, a.1 + 20.0), (b.0 + 10.0, b.1 + 20.0))
                }
            };
            let shifted: Vec<MarkerPart> = at_origin.iter().map(shift).collect();
            assert_eq!(shifted.len(), moved.len(), "shape {code}");
            for (a, b) in shifted.iter().zip(&moved) {
                assert_eq!(format!("{a:.6?}"), format!("{b:.6?}"), "shape {code}");
            }
        }
    }
}
