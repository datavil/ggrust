//! Terminal text output: a scene drawn as coloured Unicode braille (prototype).
//!
//! What it does: [`to_braille`] rasterises the scene with the PNG backend, splits
//! it into braille cells (2 x 4 dots each, so dots are about square in a terminal
//! whose cells are twice as tall as wide), lights a dot where enough of its pixels
//! differ from the plot background, and colours each cell with the mean colour of
//! its ink pixels (24-bit ANSI escapes). For terminals without an image protocol.
//!
//! What it does not do: keep text legible (labels become dot shapes), draw the
//! plot background, or detect what the terminal supports (the caller chooses).

use resvg::tiny_skia::Pixmap;

use crate::scene::Scene;
use crate::{Error, Result};

use super::png::render_pixmap;

/// Pixels per dot side when rasterising; more catches thin lines.
const OVERSAMPLE: f64 = 3.0;
/// A pixel is ink when a channel differs from the background by more than this.
const INK_DISTANCE: i32 = 40;
/// A dot lights when at least this share of its pixels is ink.
const INK_SHARE: f64 = 0.2;
/// Braille dot bits for (column, row) within a cell, Unicode order.
const DOT_BITS: [[u32; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];

/// Draw `scene` as `columns` terminal cells wide of braille text with 24-bit
/// colour escapes, one line per cell row, each line ending in a reset and `\n`.
pub fn to_braille(scene: &Scene, columns: u32) -> Result<String> {
    if !(1..=1000).contains(&columns) {
        return Err(Error::render(format!(
            "braille output needs 1 to 1000 columns, got {columns}"
        )));
    }
    let dots_wide = f64::from(columns) * 2.0;
    let scale = dots_wide * OVERSAMPLE / scene.width;
    let pixmap = render_pixmap(scene, scale)?;
    let background = scene
        .background
        .map(|color| [color.red, color.green, color.blue])
        .unwrap_or([255, 255, 255]);
    let dot_size = f64::from(pixmap.width()) / dots_wide;
    let rows = ((f64::from(pixmap.height()) / dot_size) / 4.0).ceil() as u32;
    let mut out = String::new();
    for row in 0..rows {
        let mut last_color: Option<[u8; 3]> = None;
        for column in 0..columns {
            let mut bits = 0u32;
            let mut sum = [0u64; 3];
            let mut count = 0u64;
            for (dx, column_bits) in DOT_BITS.iter().enumerate() {
                for (dy, bit) in column_bits.iter().enumerate() {
                    let dot_x = f64::from(column * 2) + dx as f64;
                    let dot_y = f64::from(row * 4) + dy as f64;
                    let ink = dot_ink(&pixmap, dot_x, dot_y, dot_size, background);
                    if ink.share >= INK_SHARE {
                        bits |= bit;
                        for channel in 0..3 {
                            sum[channel] += ink.sum[channel];
                        }
                        count += ink.count;
                    }
                }
            }
            if bits == 0 {
                out.push(' ');
                continue;
            }
            let color = sum.map(|total| (total / count.max(1)) as u8);
            if last_color != Some(color) {
                out.push_str(&format!(
                    "\x1b[38;2;{};{};{}m",
                    color[0], color[1], color[2]
                ));
                last_color = Some(color);
            }
            out.push(char::from_u32(0x2800 + bits).unwrap_or(' '));
        }
        out.push_str("\x1b[0m\n");
    }
    Ok(out)
}

/// Ink pixels under one dot: their share of the dot and their summed colour.
struct DotInk {
    share: f64,
    sum: [u64; 3],
    count: u64,
}

fn dot_ink(pixmap: &Pixmap, dot_x: f64, dot_y: f64, dot_size: f64, background: [u8; 3]) -> DotInk {
    let x0 = (dot_x * dot_size) as u32;
    let y0 = (dot_y * dot_size) as u32;
    let x1 = (((dot_x + 1.0) * dot_size) as u32).min(pixmap.width());
    let y1 = (((dot_y + 1.0) * dot_size) as u32).min(pixmap.height());
    let mut ink = DotInk {
        share: 0.0,
        sum: [0; 3],
        count: 0,
    };
    let mut total = 0u64;
    for y in y0..y1 {
        for x in x0..x1 {
            let Some(pixel) = pixmap.pixel(x, y) else {
                continue;
            };
            total += 1;
            let pixel = pixel.demultiply();
            if pixel.alpha() < 128 {
                continue;
            }
            let rgb = [pixel.red(), pixel.green(), pixel.blue()];
            let distance = (0..3)
                .map(|channel| (i32::from(rgb[channel]) - i32::from(background[channel])).abs())
                .max()
                .unwrap_or(0);
            if distance > INK_DISTANCE {
                for channel in 0..3 {
                    ink.sum[channel] += u64::from(rgb[channel]);
                }
                ink.count += 1;
            }
        }
    }
    if total > 0 {
        ink.share = ink.count as f64 / total as f64;
    }
    ink
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Color, Scene};

    fn blank(width: f64, height: f64) -> Scene {
        Scene {
            width,
            height,
            metadata: false,
            background: Some(Color::WHITE),
            nodes: Vec::new(),
        }
    }

    #[test]
    fn blank_scene_is_spaces_with_square_dots() {
        let text = to_braille(&blank(200.0, 100.0), 10).expect("renders");
        let lines: Vec<&str> = text.lines().collect();
        // 10 cells = 20 dots wide; 100 / 200 * 20 = 10 dots tall = 3 cell rows.
        assert_eq!(lines.len(), 3);
        assert!(
            lines
                .iter()
                .all(|line| line.trim_end_matches("\x1b[0m").trim().is_empty())
        );
    }

    #[test]
    fn zero_columns_is_an_error() {
        let error = to_braille(&blank(10.0, 10.0), 0).expect_err("rejects");
        assert!(error.to_string().contains("1 to 1000 columns"));
    }
}
