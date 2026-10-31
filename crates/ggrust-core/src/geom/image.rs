//! `geom_image`: one bitmap (`href`, a PNG data URL) stretched over the box
//! `xmin`..`xmax` x `ymin`..`ymax`. `geom_imshow` builds this layer in Python.
//!
//! Follows lets-plot `ImageGeom`: the image's top row is drawn at `ymax`, pixels
//! are scaled without smoothing, the plot defaults to `coord_fixed()`. Under
//! `coord_flip` the bitmap is transposed and reversed axes mirror it, so each
//! pixel stays at its data position (as `geom_raster`).
//!
//! What it does not do: image formats other than PNG, `href` URLs that are not
//! `data:image/png;base64,...`, a legend key (the `paint_c` colorbar comes from
//! the color scale, the layer itself draws no key).

use super::{DrawLayer, Geom};
use crate::coord::Panel;
use crate::parameters::Parameters;
use crate::scene::{Color, Image, Node, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};
use crate::{Error, Result};

/// The image box.
const BOX_AESTHETICS: &[Aesthetic] = &[
    Aesthetic::Xmin,
    Aesthetic::Xmax,
    Aesthetic::Ymin,
    Aesthetic::Ymax,
];

/// The colorbar aesthetics (`color_by`); the image does not draw them.
const STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (
        Aesthetic::PaintA,
        StyleDefault::Color(Color::rgb(0x47, 0x47, 0x47)),
    ),
    (
        Aesthetic::PaintB,
        StyleDefault::Color(Color::rgb(0x47, 0x47, 0x47)),
    ),
    (
        Aesthetic::PaintC,
        StyleDefault::Color(Color::rgb(0x47, 0x47, 0x47)),
    ),
];

/// The only `href` form ggrust reads (what `geom_imshow` writes).
const PNG_DATA_PREFIX: &str = "data:image/png;base64,";

/// A bitmap image placed in data space.
pub struct ImageGeom;

impl Geom for ImageGeom {
    fn name(&self) -> &'static str {
        "image"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        BOX_AESTHETICS
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        STYLE_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["href", "color_by"]
    }

    fn default_coord(&self) -> Option<&'static str> {
        Some("fixed")
    }

    fn inherits_aes(&self) -> bool {
        false
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let corners = BOX_AESTHETICS
            .iter()
            .map(|&aesthetic| {
                layer
                    .data
                    .continuous(aesthetic)
                    .and_then(|column| column.get(0))
            })
            .collect::<Option<Vec<f64>>>();
        let Some(&[x0, x1, y0, y1]) = corners.as_deref() else {
            return Ok(Vec::new());
        };
        if !(x0.is_finite() && x1.is_finite() && y0.is_finite() && y1.is_finite()) {
            return Ok(Vec::new());
        }
        let (columns, rows, pixels) = decode_href(&layer.parameters)?;

        // Pixel corners of the image's top-left (xmin, ymax) and bottom-right
        // (xmax, ymin); data x may run vertically (coord_flip), axes may be reversed.
        let top_left = panel.map(x0, y1);
        let bottom_right = panel.map(x1, y0);
        let rect = Rect {
            x: top_left.0.min(bottom_right.0),
            y: top_left.1.min(bottom_right.1),
            width: (top_left.0 - bottom_right.0).abs(),
            height: (top_left.1 - bottom_right.1).abs(),
        };
        let (screen_columns, screen_rows) = if panel.flipped {
            (rows, columns)
        } else {
            (columns, rows)
        };
        let mirror_horizontal = bottom_right.0 < top_left.0;
        let mirror_vertical = bottom_right.1 < top_left.1;
        let mut screen = vec![Color::TRANSPARENT; screen_columns * screen_rows];
        for row in 0..rows {
            for column in 0..columns {
                // Image column runs along data x, image row along data y (top down).
                let (mut horizontal, mut vertical) = if panel.flipped {
                    (row, column)
                } else {
                    (column, row)
                };
                if mirror_horizontal {
                    horizontal = screen_columns - 1 - horizontal;
                }
                if mirror_vertical {
                    vertical = screen_rows - 1 - vertical;
                }
                screen[vertical * screen_columns + horizontal] = pixels[row * columns + column];
            }
        }
        Ok(vec![Node::Image(Image {
            rect,
            columns: screen_columns as u32,
            rows: screen_rows as u32,
            pixels: screen,
        })])
    }

    fn minimum_key_size(&self, _styles: &Styles) -> Result<(f64, f64)> {
        Ok((0.0, 0.0))
    }

    fn draw_key(&self, _styles: &Styles, _cell: Rect) -> Result<Vec<Node>> {
        Ok(Vec::new())
    }
}

/// The layer's `href` as (columns, rows, row-major straight-alpha colors).
fn decode_href(parameters: &Parameters) -> Result<(usize, usize, Vec<Color>)> {
    let path = parameters.path_of("href");
    let Some(href) = parameters.string("href")? else {
        return Err(Error::InvalidSpec(format!(
            "at '{path}': geom_image needs an 'href' (a '{PNG_DATA_PREFIX}...' data URL); \
             build the layer with geom_imshow()"
        )));
    };
    let Some(encoded) = href.strip_prefix(PNG_DATA_PREFIX) else {
        let start: String = href.chars().take(30).collect();
        return Err(Error::InvalidSpec(format!(
            "at '{path}': expected a '{PNG_DATA_PREFIX}...' data URL, got '{start}...'; \
             ggrust reads embedded PNG images only (as geom_imshow() writes them)"
        )));
    };
    let invalid = |reason: String| Error::InvalidSpec(format!("at '{path}': {reason}"));
    let bytes = decode_base64(encoded).ok_or_else(|| invalid("invalid base64 data".into()))?;
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder
        .read_info()
        .map_err(|error| invalid(format!("invalid PNG image: {error}")))?;
    let mut buffer = vec![
        0;
        reader
            .output_buffer_size()
            .ok_or_else(|| { invalid("the PNG image is too large".into()) })?
    ];
    let info = reader
        .next_frame(&mut buffer)
        .map_err(|error| invalid(format!("invalid PNG image: {error}")))?;
    let (columns, rows) = (info.width as usize, info.height as usize);
    let channels = info.color_type.samples();
    let pixels = buffer[..info.buffer_size()]
        .chunks_exact(channels)
        .map(|sample| match (info.color_type, sample) {
            (png::ColorType::Rgba, &[red, green, blue, alpha]) => Color {
                red,
                green,
                blue,
                alpha,
            },
            (png::ColorType::Rgb, &[red, green, blue]) => Color::rgb(red, green, blue),
            (png::ColorType::GrayscaleAlpha, &[grey, alpha]) => Color {
                red: grey,
                green: grey,
                blue: grey,
                alpha,
            },
            (_, &[grey, ..]) => Color::rgb(grey, grey, grey),
            _ => Color::TRANSPARENT,
        })
        .collect::<Vec<_>>();
    if columns == 0 || rows == 0 || pixels.len() != columns * rows {
        return Err(invalid(format!(
            "the PNG image has {} pixels, expected {columns} x {rows}",
            pixels.len()
        )));
    }
    Ok((columns, rows, pixels))
}

/// Standard base64 (with or without padding); `None` on any other character.
fn decode_base64(text: &str) -> Option<Vec<u8>> {
    let text = text.trim_end_matches('=');
    let mut output = Vec::with_capacity(text.len() * 3 / 4);
    let mut accumulator = 0u32;
    let mut bits = 0;
    for byte in text.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        accumulator = accumulator << 6 | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((accumulator >> bits) as u8);
            accumulator &= (1 << bits) - 1;
        }
    }
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trips() {
        for (text, bytes) in [
            ("", &b""[..]),
            ("TQ==", b"M"),
            ("TWE=", b"Ma"),
            ("TWFu", b"Man"),
            ("/+8A", &[0xff, 0xef, 0x00]),
        ] {
            assert_eq!(decode_base64(text).as_deref(), Some(bytes));
        }
        assert_eq!(decode_base64("ab$c"), None);
    }
}
