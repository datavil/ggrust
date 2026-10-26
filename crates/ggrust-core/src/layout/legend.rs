//! Discrete legends: measure and draw (lets-plot `LegendComponentLayout`,
//! `LegendBreak`, `LegendBoxesLayoutUtil`).
//!
//! A [`Legend`] is a title plus keys; each key draws one entry per layer through
//! its geom. Keys are at least `legend_key_size` square (lets-plot rounds a geom's
//! minimum key size up to an odd number + 1). Up to 15 keys stack in one column,
//! more fill further columns. Text is measured with the real font, as everywhere
//! in the layout.
//!
//! What it does not do (yet): colorbars, horizontal legends, `guide_legend` options.

use super::{text_band, text_node};
use crate::Result;
use crate::geom::Geom;
use crate::scene::{Group, HorizontalAlign, Node, PerItem, Rect, Rects};
use crate::style::Styles;
use crate::text::measure;
use crate::theme::Theme;

/// lets-plot `LegendAssembler.MAX_ROWS_PER_COL`.
const MAX_ROWS_PER_COLUMN: usize = 15;
/// lets-plot `Legend.LINES_MAX_LENGTH`: labels wrap at this many characters.
const LABEL_LINE_LENGTH: usize = 30;
/// lets-plot `Legend.LINES_MAX_COUNT`.
const LABEL_MAX_LINES: usize = 5;

/// One layer's part of a legend key.
pub struct KeyLayer {
    /// The geom that draws it.
    pub geom: &'static dyn Geom,
    /// One-item style values for every style aesthetic of the geom.
    pub styles: Styles,
}

/// One legend entry: a label and the keys of every layer showing it.
pub struct LegendKey {
    /// Label next to the key.
    pub label: String,
    /// Keys drawn on top of each other, in layer order.
    pub layers: Vec<KeyLayer>,
}

/// A discrete legend.
pub struct Legend {
    /// Title; empty takes no space.
    pub title: String,
    /// Entries, top to bottom (then left to right).
    pub keys: Vec<LegendKey>,
}

/// A legend measured for drawing; positions relative to its box origin.
struct MeasuredLegend<'a> {
    legend: &'a Legend,
    size: (f64, f64),
    /// Per key: the key cell, the label anchor (left, vertical middle) and lines.
    cells: Vec<(Rect, (f64, f64), Vec<String>)>,
}

/// Legends stacked vertically, measured.
pub struct LegendBlock<'a> {
    legends: Vec<MeasuredLegend<'a>>,
    /// Width and height of the whole stack.
    pub size: (f64, f64),
}

/// Measure `legends` (lets-plot vertical arrangement, left-justified).
/// `None` when there is nothing to show.
pub fn measure_legends<'a>(
    legends: &'a [Legend],
    theme: &Theme,
) -> Result<Option<LegendBlock<'a>>> {
    let mut measured = Vec::new();
    for legend in legends.iter().filter(|legend| !legend.keys.is_empty()) {
        measured.push(measure_legend(legend, theme)?);
    }
    if measured.is_empty() {
        return Ok(None);
    }
    let width = measured
        .iter()
        .map(|legend| legend.size.0)
        .fold(0.0, f64::max);
    let height = measured.iter().map(|legend| legend.size.1).sum::<f64>()
        + theme.legend_spacing * (measured.len() - 1) as f64;
    Ok(Some(LegendBlock {
        legends: measured,
        size: (width, height),
    }))
}

fn measure_legend<'a>(legend: &'a Legend, theme: &Theme) -> Result<MeasuredLegend<'a>> {
    let margin = theme.legend_margin;
    let (title_width, title_height) = match theme.legend_title {
        Some(style) if !legend.title.trim().is_empty() => (
            measure(&legend.title, style.size, style.face).width,
            text_band(style),
        ),
        _ => (0.0, 0.0),
    };
    // lets-plot `LegendBoxLayout.graphOrigin`: half a title line between title and keys.
    let graph_top = if title_height > 0.0 {
        title_height * 1.5
    } else {
        0.0
    };

    let (label_gap, label_extra) = match theme.legend_text {
        // lets-plot: half the width of "_" between key and label; a third of a line
        // between labels.
        Some(style) => (
            measure("_", style.size, style.face).width / 2.0,
            text_band(style) / 3.0,
        ),
        None => (0.0, 0.0),
    };
    let label_size = |lines: &[String]| match theme.legend_text {
        Some(style) => (
            lines
                .iter()
                .map(|line| measure(line, style.size, style.face).width)
                .fold(0.0, f64::max),
            text_band(style) * lines.len() as f64,
        ),
        None => (0.0, 0.0),
    };
    let labels: Vec<Vec<String>> = legend
        .keys
        .iter()
        .map(|key| wrap_label(&key.label))
        .collect();

    let mut key_sizes = Vec::with_capacity(legend.keys.len());
    for key in &legend.keys {
        let mut size = (0.0_f64, 0.0_f64);
        for layer in &key.layers {
            let (width, height) = layer.geom.minimum_key_size(&layer.styles)?;
            size.0 = size.0.max(theme.legend_key_size.max(pretty(width)));
            size.1 = size.1.max(theme.legend_key_size.max(pretty(height)));
        }
        key_sizes.push(size);
    }
    // Vertical legend: every key as wide as the widest.
    let key_width = key_sizes.iter().map(|size| size.0).fold(0.0, f64::max);

    let count = legend.keys.len();
    let rows = count.min(MAX_ROWS_PER_COLUMN);
    let columns = count.div_ceil(rows);
    let mut column_widths = vec![0.0_f64; columns];
    let mut row_heights = vec![0.0_f64; rows];
    let mut label_offsets = Vec::with_capacity(count);
    for index in 0..count {
        let (column, row) = (index / rows, index % rows);
        let extra = if row == rows - 1 { 0.0 } else { label_extra };
        let (label_width, label_height) = label_size(&labels[index]);
        let key_height = key_sizes[index].1;
        let offset = (key_width + label_gap, key_height / 2.0);
        label_offsets.push(offset);
        column_widths[column] = column_widths[column].max(offset.0 + label_width);
        row_heights[row] = row_heights[row].max(key_height).max(label_height + extra);
    }
    let spacing = (
        theme.legend_key_spacing.0 + label_gap,
        theme.legend_key_spacing.1,
    );
    let column_x = running_offsets(&column_widths, spacing.0);
    let row_y = running_offsets(&row_heights, spacing.1);

    let origin = (margin.left, margin.top + graph_top);
    let mut cells = Vec::with_capacity(count);
    let (mut graph_width, mut graph_height) = (0.0_f64, 0.0_f64);
    for index in 0..count {
        let (column, row) = (index / rows, index % rows);
        let (x, y) = (column_x[column], row_y[row]);
        graph_width = graph_width.max(x + column_widths[column]);
        graph_height = graph_height.max(y + row_heights[row]);
        let cell = Rect {
            x: origin.0 + x,
            y: origin.1 + y,
            width: key_width,
            height: key_sizes[index].1,
        };
        let (dx, dy) = label_offsets[index];
        cells.push((
            cell,
            (origin.0 + x + dx, origin.1 + y + dy),
            labels[index].clone(),
        ));
    }
    let size = (
        margin.left + title_width.max(graph_width) + margin.right,
        margin.top + graph_top + graph_height + margin.bottom,
    );
    Ok(MeasuredLegend {
        legend,
        size,
        cells,
    })
}

/// Label lines, lets-plot `RichText.wrap(label, 30, 5)`: split at newlines, then
/// words fill lines of up to 30 characters (longer words are cut); 5 lines or
/// more keep the first 5 and add a line "...".
fn wrap_label(label: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for segment in label.split('\n') {
        let mut line = String::new();
        let mut length = 0;
        for word in segment.split(' ').filter(|word| !word.is_empty()) {
            let word_length = word.chars().count();
            let needed = if length == 0 {
                word_length
            } else {
                length + 1 + word_length
            };
            if needed <= LABEL_LINE_LENGTH {
                if length > 0 {
                    line.push(' ');
                }
                line.push_str(word);
                length = needed;
            } else if word_length <= LABEL_LINE_LENGTH {
                lines.push(std::mem::take(&mut line));
                line.push_str(word);
                length = word_length;
            } else {
                // Cut the word: fill the current line, then whole chunks.
                let mut characters: Vec<char> = word.chars().collect();
                if length > 0 {
                    let room = LABEL_LINE_LENGTH.saturating_sub(length + 1);
                    if room > 0 {
                        line.push(' ');
                        line.extend(characters.drain(..room.min(characters.len())));
                    }
                    lines.push(std::mem::take(&mut line));
                }
                let chunks: Vec<String> = characters
                    .chunks(LABEL_LINE_LENGTH)
                    .map(|chunk| chunk.iter().collect())
                    .collect();
                let (last, full) = chunks
                    .split_last()
                    .map_or((String::new(), &[][..]), |(last, full)| {
                        (last.clone(), full)
                    });
                lines.extend(full.iter().cloned());
                length = last.chars().count();
                line = last;
            }
        }
        lines.push(line);
    }
    if lines.len() >= LABEL_MAX_LINES {
        lines.truncate(LABEL_MAX_LINES);
        lines.push("...".to_owned());
    }
    lines
}

/// lets-plot `LegendBreak.pretty`: round up to an odd number, plus a 1 px margin.
fn pretty(size: f64) -> f64 {
    (size / 2.0).floor() * 2.0 + 2.0
}

/// Start offsets of consecutive extents separated by `spacing`.
fn running_offsets(extents: &[f64], spacing: f64) -> Vec<f64> {
    let mut offsets = Vec::with_capacity(extents.len());
    let mut position = 0.0;
    for extent in extents {
        offsets.push(position);
        position += extent + spacing;
    }
    offsets
}

/// Draw the block with its top-left corner at `origin`.
pub fn draw_legends(block: &LegendBlock, origin: (f64, f64), theme: &Theme) -> Result<Vec<Node>> {
    let mut nodes = Vec::new();
    let mut top = origin.1;
    for measured in &block.legends {
        let (x, y) = (origin.0, top);
        let mut legend_nodes = Vec::new();
        if let Some(fill) = theme.legend_background {
            legend_nodes.push(Node::Rects(Rects {
                rects: vec![Rect {
                    x,
                    y,
                    width: measured.size.0,
                    height: measured.size.1,
                }],
                fill: PerItem::Uniform(Some(fill)),
                stroke: None,
            }));
        }
        if let Some(style) = theme.legend_title
            && !measured.legend.title.trim().is_empty()
        {
            legend_nodes.push(text_node(
                &measured.legend.title,
                style,
                (
                    x + theme.legend_margin.left,
                    y + theme.legend_margin.top + text_band(style) / 2.0,
                ),
                HorizontalAlign::Start,
                0.0,
            ));
        }
        for (key, (cell, label_anchor, lines)) in measured.legend.keys.iter().zip(&measured.cells) {
            let cell = Rect {
                x: x + cell.x,
                y: y + cell.y,
                ..*cell
            };
            for layer in &key.layers {
                legend_nodes.extend(layer.geom.draw_key(&layer.styles, cell)?);
            }
            if let Some(style) = theme.legend_text {
                // Lines centred as a block on the key's middle.
                let band = text_band(style);
                let first = label_anchor.1 - band * (lines.len() as f64 - 1.0) / 2.0;
                for (line_index, line) in lines.iter().enumerate() {
                    legend_nodes.push(text_node(
                        line,
                        style,
                        (x + label_anchor.0, y + first + band * line_index as f64),
                        HorizontalAlign::Start,
                        0.0,
                    ));
                }
            }
        }
        nodes.push(Node::Group(Group {
            clip: None,
            role: Some("legend"),
            nodes: legend_nodes,
        }));
        top += measured.size.1 + theme.legend_spacing;
    }
    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretty_rounds_up_to_odd_plus_one() {
        assert_eq!(pretty(13.0), 14.0);
        assert_eq!(pretty(14.0), 16.0);
        assert_eq!(pretty(21.0), 22.0);
    }

    #[test]
    fn labels_wrap_like_lets_plot() {
        assert_eq!(wrap_label("short"), ["short"]);
        assert_eq!(
            wrap_label("this is a fairly long label with spaces in it ok"),
            ["this is a fairly long label", "with spaces in it ok"]
        );
        let long = "a".repeat(60);
        assert_eq!(wrap_label(&long), ["a".repeat(30), "a".repeat(30)]);
        assert_eq!(wrap_label("one\ntwo"), ["one", "two"]);
        assert_eq!(
            wrap_label("1\n2\n3\n4\n5\n6"),
            ["1", "2", "3", "4", "5", "..."]
        );
    }

    #[test]
    fn offsets_accumulate_with_spacing() {
        assert_eq!(running_offsets(&[10.0, 20.0, 5.0], 2.0), [0.0, 12.0, 34.0]);
    }
}
