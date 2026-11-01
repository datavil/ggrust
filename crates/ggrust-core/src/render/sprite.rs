//! Marker sprites for the PNG renderer: large point batches are stamped, not traced.
//!
//! A sprite holds the anti-aliased coverage of one marker geometry (shape, size,
//! outline width) at one sub-pixel offset, rendered once by tiny-skia exactly as
//! the direct path would draw it (fill and outline coverage per part, in drawing
//! order). Stamping blends each item's own colors through that coverage
//! (premultiplied source-over, the clip mask applied), so per-item colors keep
//! one sprite. Centres are quantized to [`SUBPIXELS`] steps per device pixel
//! (at most 1/32 px off). This module does not decide when sprites are used;
//! the PNG renderer does.

use std::collections::HashMap;

use resvg::tiny_skia::{self, FillRule, Mask, Paint, Pixmap, Transform};

use crate::scene::{Color, MarkerShape};

/// Sub-pixel steps per device pixel and axis.
pub(super) const SUBPIXELS: usize = 16;

/// Geometries cached per render; markers of further geometries are traced.
const MAX_GEOMETRIES: usize = 256;

/// Sprites wider or taller than this (device pixels) are not cached.
const MAX_SPRITE_SIDE: f64 = 256.0;

/// Markers of a geometry traced before it gets sprites: rendering a sprite costs
/// about one traced marker, so rare geometries (continuous sizes) stay traced.
const STAMP_AFTER: usize = 64;

/// Coverage bytes cached per render; past it, new sprites are traced instead.
const MAX_SPRITE_BYTES: usize = 64 << 20;

/// A marker's geometry: what its coverage depends on (colors do not).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Geometry {
    shape: MarkerShape,
    size: u64,
    stroke_width: u64,
}

/// Coverage of one marker part, in drawing order.
struct Layer {
    /// True for an outline, false for a fill.
    outline: bool,
    /// One coverage byte per sprite pixel (row major).
    coverage: Vec<u8>,
}

/// One geometry at one sub-pixel offset, cropped to its non-zero coverage.
struct Sprite {
    /// Top-left pixel relative to the marker's whole-pixel centre.
    left: i64,
    top: i64,
    width: usize,
    height: usize,
    layers: Vec<Layer>,
}

/// A geometry's markers so far, and its sprites once it has [`STAMP_AFTER`].
struct Entry {
    seen: usize,
    /// Indexed by sub-pixel offset (`y * SUBPIXELS + x`); `None` until stamped.
    sprites: Option<Vec<Option<Sprite>>>,
}

/// Per-render sprite cache.
pub(super) struct Sprites {
    scale: f64,
    cache: HashMap<Geometry, Entry>,
    /// Coverage bytes held by all sprites.
    bytes: usize,
}

impl Sprites {
    pub(super) fn new(scale: f64) -> Self {
        Sprites {
            scale,
            cache: HashMap::new(),
            bytes: 0,
        }
    }

    /// Stamp one marker centred at scene `(x, y)`. Returns false (nothing drawn)
    /// when the geometry is rare, too large or over the cache budget, so the
    /// caller traces it instead.
    pub(super) fn stamp(
        &mut self,
        pixmap: &mut Pixmap,
        mask: Option<&Mask>,
        (x, y): (f64, f64),
        (shape, size, stroke_width): (MarkerShape, f64, f64),
        fill: Option<Color>,
        stroke: Option<Color>,
    ) -> bool {
        let device_x = (x * self.scale * SUBPIXELS as f64).round();
        let device_y = (y * self.scale * SUBPIXELS as f64).round();
        // Far outside the canvas: nothing visible, and quantizing must not overflow.
        // The margin covers the largest sprite reaching in from outside.
        let limit = (f64::from(pixmap.width().max(pixmap.height())) * 4.0 + MAX_SPRITE_SIDE)
            * SUBPIXELS as f64;
        if !(-limit..=limit).contains(&device_x) || !(-limit..=limit).contains(&device_y) {
            return true;
        }
        let (quantized_x, quantized_y) = (device_x as i64, device_y as i64);
        let steps = SUBPIXELS as i64;
        let (pixel_x, sub_x) = (quantized_x.div_euclid(steps), quantized_x.rem_euclid(steps));
        let (pixel_y, sub_y) = (quantized_y.div_euclid(steps), quantized_y.rem_euclid(steps));
        let geometry = Geometry {
            shape,
            size: size.to_bits(),
            stroke_width: stroke_width.to_bits(),
        };
        if !self.cache.contains_key(&geometry) && self.cache.len() >= MAX_GEOMETRIES {
            return false;
        }
        let fits = (size + stroke_width * 4.0) * self.scale + 4.0 <= MAX_SPRITE_SIDE;
        let entry = self.cache.entry(geometry).or_insert(Entry {
            seen: 0,
            sprites: None,
        });
        entry.seen += 1;
        if entry.seen <= STAMP_AFTER || !fits {
            return false;
        }
        let sprites = entry
            .sprites
            .get_or_insert_with(|| (0..SUBPIXELS * SUBPIXELS).map(|_| None).collect());
        let slot = &mut sprites[sub_y as usize * SUBPIXELS + sub_x as usize];
        if slot.is_none() {
            if self.bytes >= MAX_SPRITE_BYTES {
                return false;
            }
            let sprite = Sprite::render(
                (shape, size, stroke_width),
                self.scale,
                (sub_x as f64 / steps as f64, sub_y as f64 / steps as f64),
            );
            self.bytes += sprite.layers.len() * sprite.width * sprite.height;
            *slot = Some(sprite);
        }
        if let Some(sprite) = slot {
            sprite.blend(pixmap, mask, (pixel_x, pixel_y), fill, stroke);
        }
        true
    }
}

impl Sprite {
    /// Render the geometry with its centre `offset` device pixels past a whole pixel.
    fn render(
        (shape, size, stroke_width): (MarkerShape, f64, f64),
        scale: f64,
        offset: (f64, f64),
    ) -> Sprite {
        let half = ((size + stroke_width * 4.0) * scale / 2.0).ceil() as i64 + 2;
        let side = (2 * half + 1) as u32;
        let empty = Sprite {
            left: 0,
            top: 0,
            width: 0,
            height: 0,
            layers: Vec::new(),
        };
        let Some(mut canvas) = Pixmap::new(side, side) else {
            return empty;
        };
        // Same scene-space geometry and scale as the direct path; only the
        // translation differs (the centre lands at `half + offset` in the sprite).
        let transform = Transform::from_scale(scale as f32, scale as f32).post_translate(
            (half as f64 + offset.0) as f32,
            (half as f64 + offset.1) as f32,
        );
        let mut white = Paint {
            anti_alias: true,
            ..Paint::default()
        };
        white.set_color(tiny_skia::Color::WHITE);
        let draws_outline = stroke_width.is_finite() && stroke_width > 0.0;
        let mut layers = Vec::new();
        let mut capture = |canvas: &mut Pixmap, outline: bool| {
            let coverage: Vec<u8> = canvas.pixels().iter().map(|pixel| pixel.alpha()).collect();
            canvas.fill(tiny_skia::Color::TRANSPARENT);
            layers.push(Layer { outline, coverage });
        };
        shape.for_each_part(0.0, 0.0, size, stroke_width, |part| {
            let Some((path, closed)) = super::png::marker_path(&part) else {
                return;
            };
            if closed {
                canvas.fill_path(&path, &white, FillRule::Winding, transform, None);
                capture(&mut canvas, false);
            }
            if draws_outline {
                let style = tiny_skia::Stroke {
                    width: stroke_width as f32,
                    ..tiny_skia::Stroke::default()
                };
                canvas.stroke_path(&path, &white, &style, transform, None);
                capture(&mut canvas, true);
            }
        });
        crop(layers, side as usize, half)
    }

    /// Blend through the coverage with the sprite's centre pixel at `(x, y)`.
    fn blend(
        &self,
        pixmap: &mut Pixmap,
        mask: Option<&Mask>,
        (x, y): (i64, i64),
        fill: Option<Color>,
        stroke: Option<Color>,
    ) {
        let (canvas_width, canvas_height) = (i64::from(pixmap.width()), i64::from(pixmap.height()));
        let (left, top) = (x + self.left, y + self.top);
        let column_start = (-left).max(0) as usize;
        let row_start = (-top).max(0) as usize;
        let column_end = (canvas_width - left).clamp(0, self.width as i64) as usize;
        let row_end = (canvas_height - top).clamp(0, self.height as i64) as usize;
        if column_start >= column_end || row_start >= row_end {
            return;
        }
        let mask = mask.map(Mask::data);
        let data = pixmap.data_mut();
        for layer in &self.layers {
            let color = if layer.outline { stroke } else { fill };
            let Some(color) = color else {
                continue;
            };
            let source = premultiply(color);
            for row in row_start..row_end {
                let canvas_row = (top + row as i64) as usize * canvas_width as usize;
                let coverage_row = &layer.coverage[row * self.width..(row + 1) * self.width];
                for (column, &value) in coverage_row
                    .iter()
                    .enumerate()
                    .take(column_end)
                    .skip(column_start)
                {
                    let mut coverage = u32::from(value);
                    if coverage == 0 {
                        continue;
                    }
                    let pixel = canvas_row + (left + column as i64) as usize;
                    if let Some(mask) = mask {
                        coverage = div255(coverage * u32::from(mask[pixel]));
                        if coverage == 0 {
                            continue;
                        }
                    }
                    let target = &mut data[pixel * 4..pixel * 4 + 4];
                    let keep = 255 - div255(source[3] * coverage);
                    for (channel, value) in target.iter_mut().enumerate() {
                        *value = (div255(source[channel] * coverage)
                            + div255(u32::from(*value) * keep))
                        .min(255) as u8;
                    }
                }
            }
        }
    }
}

/// Crop the layers to the box where any of them has coverage.
fn crop(layers: Vec<Layer>, side: usize, half: i64) -> Sprite {
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (side, side, 0, 0);
    for layer in &layers {
        for (index, &value) in layer.coverage.iter().enumerate() {
            if value != 0 {
                let (column, row) = (index % side, index / side);
                min_x = min_x.min(column);
                max_x = max_x.max(column);
                min_y = min_y.min(row);
                max_y = max_y.max(row);
            }
        }
    }
    if min_x > max_x || min_y > max_y {
        return Sprite {
            left: 0,
            top: 0,
            width: 0,
            height: 0,
            layers: Vec::new(),
        };
    }
    let (width, height) = (max_x - min_x + 1, max_y - min_y + 1);
    let layers = layers
        .into_iter()
        .map(|layer| Layer {
            outline: layer.outline,
            coverage: (min_y..=max_y)
                .flat_map(|row| layer.coverage[row * side + min_x..=row * side + max_x].to_vec())
                .collect(),
        })
        .collect();
    Sprite {
        left: min_x as i64 - half,
        top: min_y as i64 - half,
        width,
        height,
        layers,
    }
}

/// Premultiplied RGBA channels.
fn premultiply(color: Color) -> [u32; 4] {
    let alpha = u32::from(color.alpha);
    [
        div255(u32::from(color.red) * alpha),
        div255(u32::from(color.green) * alpha),
        div255(u32::from(color.blue) * alpha),
        alpha,
    ]
}

/// `value / 255`, rounded, for `value <= 255 * 255`.
fn div255(value: u32) -> u32 {
    (value + 128 + ((value + 128) >> 8)) >> 8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn div255_rounds_like_division() {
        for value in 0..=255 * 255 {
            assert_eq!(
                div255(value),
                (value as f64 / 255.0).round() as u32,
                "{value}"
            );
        }
    }

    #[test]
    fn open_parts_have_no_fill_layer() {
        // Lines are outlines only: no fill layer even with a fill color.
        let sprite = Sprite::render((MarkerShape::Plus, 6.0, 1.0), 1.0, (0.0, 0.0));
        assert!(!sprite.layers.is_empty());
        assert!(sprite.layers.iter().all(|layer| layer.outline));
    }
}
