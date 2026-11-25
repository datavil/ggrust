//! Terminal text output: a scene drawn as coloured Unicode braille (prototype).
//!
//! What it does: [`to_braille`] rasterises the scene without its text through the
//! PNG backend, splits it into braille cells (2 x 4 dots each, so dots are about
//! square in a terminal whose cells are twice as tall as wide), lights a dot where
//! enough of its pixels differ from the plot background, and colours each cell
//! with the mean colour of its ink pixels. Text nodes are then written as real
//! terminal characters in the cells they anchor to (rotated text stacked one
//! character per row), over the dots. Colours are 24-bit ANSI escapes. By
//! default the terminal's background shows and neutral colours (text, axes)
//! take the terminal's text colour; `plot_background` paints the plot's
//! background behind every cell and keeps all of the plot's colours.
//!
//! What it does not do: keep font sizes (every label is one cell tall), draw a
//! label that would overlap an earlier one (it is skipped), or detect what the
//! terminal supports (the caller chooses).

use resvg::tiny_skia::Pixmap;

use crate::scene::{Group, HorizontalAlign, Node, Raster, Scene, Text, VerticalAlign};
use crate::{Error, Result};

use super::png::render_pixmap;

/// Pixels per dot side when rasterising; more catches thin lines.
const OVERSAMPLE: f64 = 3.0;
/// A pixel is ink when a channel differs from the background by more than this.
const INK_DISTANCE: i32 = 40;
/// A dot lights when at least this share of its pixels is ink.
const INK_SHARE: f64 = 0.2;
/// Colours whose channels spread less than this are neutral (greys).
const NEUTRAL_SPREAD: u8 = 24;
/// Braille dot bits for (column, row) within a cell, Unicode order.
const DOT_BITS: [[u32; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];

/// One terminal cell: a glyph and its colour; `None` colour is an empty cell.
#[derive(Clone, Copy)]
struct Cell {
    glyph: char,
    color: Option<[u8; 3]>,
    text: bool,
}

const EMPTY: Cell = Cell {
    glyph: ' ',
    color: None,
    text: false,
};

/// Draw `scene` as `columns` terminal cells wide: braille for shapes, plain
/// characters for text, with 24-bit colour escapes. One line per cell row, each
/// ending in a reset and `\n`. `plot_background` paints every cell with the
/// plot's background and keeps all of the plot's colours; otherwise the
/// terminal's background shows and neutral colours use the terminal's text colour.
pub fn to_braille(scene: &Scene, columns: u32, plot_background: bool) -> Result<String> {
    if !(1..=1000).contains(&columns) {
        return Err(Error::render(format!(
            "braille output needs 1 to 1000 columns, got {columns}"
        )));
    }
    let mut texts = Vec::new();
    let shapes = Scene {
        nodes: without_text(&scene.nodes, &mut texts),
        ..scene.clone()
    };
    let background = scene
        .background
        .filter(|color| color.alpha == 255)
        .map(|color| [color.red, color.green, color.blue]);
    let mut grid = braille_grid(&shapes, columns, background.unwrap_or([255, 255, 255]))?;
    let cell_width = scene.width / f64::from(columns);
    for text in &texts {
        place_text(&mut grid, text, cell_width, cell_width * 2.0);
    }
    Ok(encode(&grid, background.filter(|_| plot_background)))
}

/// `nodes` with every text node removed (recursively), the removed ones pushed
/// to `texts`. Halo copies (outline only) are dropped.
fn without_text(nodes: &[Node], texts: &mut Vec<Text>) -> Vec<Node> {
    let mut kept = Vec::with_capacity(nodes.len());
    for node in nodes {
        match node {
            Node::Text(text) => {
                if text.halo.is_none() {
                    texts.push(text.clone());
                }
            }
            Node::Group(group) => kept.push(Node::Group(Group {
                nodes: without_text(&group.nodes, texts),
                ..group.clone()
            })),
            Node::Raster(raster) => kept.push(Node::Raster(Raster {
                nodes: without_text(&raster.nodes, texts),
                ..raster.clone()
            })),
            other => kept.push(other.clone()),
        }
    }
    kept
}

/// The scene's shapes as rows of braille cells.
fn braille_grid(scene: &Scene, columns: u32, background: [u8; 3]) -> Result<Vec<Vec<Cell>>> {
    let dots_wide = f64::from(columns) * 2.0;
    let scale = dots_wide * OVERSAMPLE / scene.width;
    let pixmap = render_pixmap(scene, scale)?;
    let dot_size = f64::from(pixmap.width()) / dots_wide;
    let rows = ((f64::from(pixmap.height()) / dot_size) / 4.0).ceil() as u32;
    let mut grid = Vec::with_capacity(rows as usize);
    for row in 0..rows {
        let mut line = Vec::with_capacity(columns as usize);
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
            line.push(if bits == 0 {
                EMPTY
            } else {
                Cell {
                    glyph: char::from_u32(0x2800 + bits).unwrap_or(' '),
                    color: Some(sum.map(|total| (total / count.max(1)) as u8)),
                    text: false,
                }
            });
        }
        grid.push(line);
    }
    Ok(grid)
}

/// Write `text` into the cells it anchors to, unless a cell already holds text.
fn place_text(grid: &mut [Vec<Cell>], text: &Text, cell_width: f64, cell_height: f64) {
    let mut content = text.content.clone();
    if let Some(superscript) = &text.superscript {
        content.push('^');
        content.push_str(superscript);
    }
    if let Some(suffix) = &text.suffix {
        content.push_str(suffix);
    }
    let glyphs: Vec<char> = content.trim().chars().collect();
    if glyphs.is_empty() {
        return;
    }
    let length = glyphs.len() as f64;
    // Share of the label before the anchor, along its reading direction.
    let before = match text.horizontal {
        HorizontalAlign::Start => 0.0,
        HorizontalAlign::Middle => 0.5,
        HorizontalAlign::End => 1.0,
    };
    // Offset from the anchor to the line's middle, across the reading direction.
    let across = match text.vertical {
        VerticalAlign::Top => 0.4,
        VerticalAlign::Middle => 0.0,
        VerticalAlign::Baseline => -0.3,
        VerticalAlign::Bottom => -0.4,
    } * text.size;
    let angle = text.angle.rem_euclid(360.0);
    let cells: Vec<(i64, i64)> =
        if (45.0..135.0).contains(&angle) || (225.0..315.0).contains(&angle) {
            // Rotated: stack the characters top to bottom around the anchor.
            let column = ((text.x + across) / cell_width).floor() as i64;
            let reads_up = (45.0..135.0).contains(&angle);
            let top = if reads_up { 1.0 - before } else { before };
            let first = (text.y / cell_height - top * length).round() as i64;
            (0..glyphs.len() as i64)
                .map(|index| (first + index, column))
                .collect()
        } else {
            let row = ((text.y + across) / cell_height).floor() as i64;
            let first = (text.x / cell_width - before * length).round() as i64;
            (0..glyphs.len() as i64)
                .map(|index| (row, first + index))
                .collect()
        };
    let (rows, columns) = (grid.len() as i64, grid.first().map_or(0, Vec::len) as i64);
    let inside =
        |&(row, column): &(i64, i64)| (0..rows).contains(&row) && (0..columns).contains(&column);
    if cells
        .iter()
        .filter(|cell| inside(cell))
        .any(|&(row, column)| grid[row as usize][column as usize].text)
    {
        return;
    }
    let color = [text.color.red, text.color.green, text.color.blue];
    for (&glyph, cell) in glyphs.iter().zip(&cells) {
        if inside(cell) {
            grid[cell.0 as usize][cell.1 as usize] = Cell {
                glyph,
                color: Some(color),
                text: true,
            };
        }
    }
}

/// The grid as text with 24-bit colour escapes. With a `background`, every cell
/// gets it and keeps the plot's colours. Without one the terminal's own
/// background shows, and neutral colours (greys, black, white: text, axes,
/// uncoloured marks) become the terminal's own text colour so they stay legible.
fn encode(grid: &[Vec<Cell>], background: Option<[u8; 3]>) -> String {
    let mut out = String::new();
    for line in grid {
        if let Some([red, green, blue]) = background {
            out.push_str(&format!("\x1b[48;2;{red};{green};{blue}m"));
        }
        let mut last_color: Option<Option<[u8; 3]>> = None;
        for cell in line {
            if let Some(color) = cell.color {
                // `None` is the terminal's default text colour.
                let color = if background.is_none() && is_neutral(color) {
                    None
                } else {
                    Some(color)
                };
                if last_color != Some(color) {
                    match color {
                        Some([red, green, blue]) => {
                            out.push_str(&format!("\x1b[38;2;{red};{green};{blue}m"));
                        }
                        None => out.push_str("\x1b[39m"),
                    }
                    last_color = Some(color);
                }
            }
            out.push(cell.glyph);
        }
        out.push_str("\x1b[0m\n");
    }
    out
}

/// Whether a colour is grey-ish: its channels differ by less than [`NEUTRAL_SPREAD`].
fn is_neutral([red, green, blue]: [u8; 3]) -> bool {
    red.max(green).max(blue) - red.min(green).min(blue) < NEUTRAL_SPREAD
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
        let text = to_braille(&blank(200.0, 100.0), 10, true).expect("renders");
        let lines: Vec<&str> = text.lines().collect();
        // 10 cells = 20 dots wide; 100 / 200 * 20 = 10 dots tall = 3 cell rows.
        assert_eq!(lines.len(), 3);
        assert!(lines.iter().all(|line| {
            line.starts_with("\x1b[48;2;255;255;255m")
                && !line
                    .chars()
                    .any(|glyph| ('\u{2801}'..='\u{28ff}').contains(&glyph))
        }));
    }

    #[test]
    fn zero_columns_is_an_error() {
        let error = to_braille(&blank(10.0, 10.0), 0, false).expect_err("rejects");
        assert!(error.to_string().contains("1 to 1000 columns"));
    }

    #[test]
    fn text_is_written_as_characters_not_dots() {
        let mut scene = blank(200.0, 100.0);
        scene.nodes.push(Node::Text(Text {
            content: "hwy".to_owned(),
            superscript: None,
            suffix: None,
            x: 100.0,
            y: 50.0,
            size: 12.0,
            font: crate::text::Font::default(),
            color: Color::rgb(10, 20, 30),
            horizontal: HorizontalAlign::Middle,
            vertical: VerticalAlign::Middle,
            angle: 0.0,
            halo: None,
        }));
        let text = to_braille(&scene, 20, true).expect("renders");
        assert!(text.contains("\x1b[38;2;10;20;30mhwy"), "{text:?}");
        // On the terminal's background the dark grey text takes its text colour.
        let text = to_braille(&scene, 20, false).expect("renders");
        assert!(text.contains("\x1b[39mhwy"), "{text:?}");
        assert!(!text.contains("\x1b[48;2;"), "{text:?}");
        assert!(
            !text
                .chars()
                .any(|glyph| ('\u{2801}'..='\u{28ff}').contains(&glyph))
        );
    }
}
