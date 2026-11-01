//! Legends (key boxes and colorbars): measure and draw (lets-plot `LegendComponentLayout`,
//! `LegendBreak`, `LegendBoxesLayoutUtil`).
//!
//! A [`Legend`] is a title plus keys; each key draws one entry per layer through
//! its geom. Keys are at least `legend_key_size` square (lets-plot rounds a geom's
//! minimum key size up to an odd number + 1). Keys fill a grid: by default up to 15
//! rows per column (vertical legends) or 5 columns per row (horizontal legends),
//! or as `guide_legend(ncol, nrow, byrow)` asks. Horizontal legends put the title
//! at the left. Several legends stack vertically, or side by side with
//! `legend_box="horizontal"`. Text is measured with the real font, as everywhere
//! in the layout.
//!
//! Where the block goes (`legend_position`) is decided by the plot builder.

use super::colorbar::{Colorbar, ColorbarLayout, draw_colorbar, layout_colorbar};
use super::{label_node, text_band, text_node};
use crate::Result;
use crate::geom::Geom;
use crate::scale::format::Label;
use crate::scene::{Group, HorizontalAlign, Node, Rect};
use crate::style::Styles;
use crate::text::{measure, measure_label};
use crate::theme::Theme;

/// lets-plot `LegendAssembler.MAX_ROWS_PER_COL`.
const MAX_ROWS_PER_COLUMN: usize = 15;
/// lets-plot `LegendAssembler.MAX_COLS_PER_ROW`.
const MAX_COLUMNS_PER_ROW: usize = 5;
/// lets-plot `Legend.LINES_MAX_LENGTH`: labels wrap at this many characters.
const LABEL_LINE_LENGTH: usize = 30;
/// Shorter line lengths tried, in order, when a legend is too wide for its budget.
const NARROW_LINE_LENGTHS: [usize; 3] = [20, 14, 10];
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
    pub label: Label,
    /// Keys drawn on top of each other, in layer order.
    pub layers: Vec<KeyLayer>,
}

/// A discrete legend.
pub struct Legend {
    /// Title; empty takes no space.
    pub title: String,
    /// Entries, in grid order.
    pub keys: Vec<LegendKey>,
    /// `guide_legend(ncol, nrow, byrow)`.
    pub grid: KeyGrid,
}

/// How keys fill their grid (`guide_legend(ncol, nrow, byrow)`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyGrid {
    /// Columns asked for (at least 1).
    pub columns: Option<usize>,
    /// Rows asked for (at least 1).
    pub rows: Option<usize>,
    /// Fill row by row instead of column by column.
    pub by_row: bool,
}

impl KeyGrid {
    /// `self` with `other`'s counts where given; `by_row` if either asks.
    pub fn combine(self, other: KeyGrid) -> KeyGrid {
        KeyGrid {
            columns: other.columns.or(self.columns),
            rows: other.rows.or(self.rows),
            by_row: self.by_row || other.by_row,
        }
    }

    /// (columns, rows) for `count` keys (lets-plot `createLegendSpec.computeLayoutGrid`).
    fn shape(self, count: usize, horizontal: bool) -> (usize, usize) {
        let primary = |first: Option<usize>, second: Option<usize>| match (first, second) {
            (Some(first), _) => first.min(count),
            (None, Some(second)) => count.div_ceil(second),
            (None, None) => match (horizontal, self.by_row) {
                (true, true) => count.min(MAX_COLUMNS_PER_ROW),
                (true, false) => count.div_ceil(MAX_COLUMNS_PER_ROW),
                (false, true) => count.div_ceil(MAX_ROWS_PER_COLUMN),
                (false, false) => count.min(MAX_ROWS_PER_COLUMN),
            },
        };
        if self.by_row {
            let columns = primary(self.columns, self.rows).max(1);
            (columns, count.div_ceil(columns))
        } else {
            let rows = primary(self.rows, self.columns).max(1);
            (count.div_ceil(rows), rows)
        }
    }

    /// (column, row) of key `index` in a `columns` x `rows` grid.
    fn position(self, index: usize, (columns, rows): (usize, usize)) -> (usize, usize) {
        if self.by_row {
            (index % columns, index / columns)
        } else {
            (index / rows, index % rows)
        }
    }
}

/// Room a legend block may take (see [`super::legend_budget`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegendBudget {
    /// Width and height automatic key grids are reflowed and cut to.
    pub preferred: (f64, f64),
    /// Width and height grids fixed by `guide_legend(ncol, nrow)` are cut to.
    pub most: (f64, f64),
}

/// A guide box: a key legend or a colorbar.
pub enum LegendBox {
    /// Keys with labels.
    Keys(Legend),
    /// A colorbar.
    Colorbar(Colorbar),
}

impl LegendBox {
    /// The title.
    pub fn title(&self) -> &str {
        match self {
            LegendBox::Keys(legend) => &legend.title,
            LegendBox::Colorbar(colorbar) => &colorbar.title,
        }
    }

    /// Whether two boxes look the same (lets-plot `hasSameContent`): a grid that
    /// collects legends draws such boxes once.
    pub fn same_content(&self, other: &LegendBox) -> bool {
        match (self, other) {
            (LegendBox::Keys(left), LegendBox::Keys(right)) => {
                left.title == right.title
                    && left.grid == right.grid
                    && left.keys.len() == right.keys.len()
                    && left.keys.iter().zip(&right.keys).all(|(left, right)| {
                        left.label == right.label
                            && left.layers.len() == right.layers.len()
                            && left.layers.iter().zip(&right.layers).all(|(left, right)| {
                                left.geom.name() == right.geom.name()
                                    && left.styles.same_values(&right.styles)
                            })
                    })
            }
            (LegendBox::Colorbar(left), LegendBox::Colorbar(right)) => {
                left.title == right.title
                    && left.domain == right.domain
                    && left.breaks == right.breaks
                    && left.colors == right.colors
                    && left.bar_width == right.bar_width
                    && left.bar_height == right.bar_height
            }
            _ => false,
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            LegendBox::Keys(legend) => legend.keys.is_empty(),
            LegendBox::Colorbar(colorbar) => colorbar.breaks.is_empty(),
        }
    }
}

/// A box measured for drawing; positions relative to its box origin.
struct MeasuredLegend<'a> {
    title: &'a str,
    /// Top-left corner of the title's first line.
    title_anchor: (f64, f64),
    size: (f64, f64),
    content: Measured<'a>,
}

enum Measured<'a> {
    /// Per entry: the index of the key drawn (`None`: the label-only "+K more"
    /// entry of a truncated legend), the key cell, the label anchor (left, vertical
    /// middle) and lines.
    Keys(&'a Legend, Vec<KeyCell>),
    Colorbar(&'a Colorbar, ColorbarLayout),
}

type KeyCell = (Option<usize>, Rect, (f64, f64), Vec<Label>);

/// Legends arranged in a block (stacked, or side by side), measured.
pub struct LegendBlock<'a> {
    /// Each legend with its offset from the block origin.
    legends: Vec<(MeasuredLegend<'a>, (f64, f64))>,
    /// Width and height of the whole block.
    pub size: (f64, f64),
}

/// Title lines: split at newlines (lets-plot does not wrap titles).
fn title_lines(title: &str) -> Vec<&str> {
    title.split('\n').collect()
}

/// Title width and height (zero when absent).
fn title_size(title: &str, theme: &Theme) -> (f64, f64) {
    match theme.legend_title {
        Some(style) if !title.trim().is_empty() => {
            let lines = title_lines(title);
            (
                lines
                    .iter()
                    .map(|line| measure(line, style.size, style.font).width)
                    .fold(0.0, f64::max),
                text_band(style) * lines.len() as f64,
            )
        }
        _ => (0.0, 0.0),
    }
}

/// Place a title and a graph (keys or bar) in a box (lets-plot `LegendBoxLayout`):
/// vertical legends put the title above, half a title line apart; horizontal ones
/// put it at the left, both centred vertically. Returns the graph origin, the title
/// anchor and the box size, margins included.
fn arrange_title(
    title: &str,
    graph: (f64, f64),
    horizontal: bool,
    theme: &Theme,
) -> ((f64, f64), (f64, f64), (f64, f64)) {
    let margin = theme.legend_margin;
    let (title_width, title_height) = title_size(title, theme);
    let (graph_origin, content) = if horizontal {
        let height = title_height.max(graph.1);
        (
            (title_width, (height - graph.1) / 2.0),
            (title_width + graph.0, height),
        )
    } else {
        // lets-plot `LegendBoxLayout.graphOrigin`: half a title line below the title.
        let top = match theme.legend_title {
            Some(style) if title_height > 0.0 => title_height + text_band(style) / 2.0,
            _ => 0.0,
        };
        ((0.0, top), (title_width.max(graph.0), top + graph.1))
    };
    let title_top = if horizontal {
        (content.1 - title_height) / 2.0
    } else {
        0.0
    };
    (
        (margin.left + graph_origin.0, margin.top + graph_origin.1),
        (margin.left, margin.top + title_top),
        (
            margin.left + content.0 + margin.right,
            margin.top + content.1 + margin.bottom,
        ),
    )
}

/// Measure the boxes and arrange them (lets-plot `LegendBoxesLayoutUtil`): stacked
/// and left-justified, or side by side and top-justified with
/// `legend_box="horizontal"`; `legend_box_just` changes the justification.
/// `None` when there is nothing to show.
///
/// With a `budget` (the most width and height the block may take), a key legend
/// that would overflow it is reflowed into more columns (rows when horizontal),
/// then truncated to the keys that fit plus a "+K more" entry. This is a ggrust
/// deviation: lets-plot and ggplot2 let such legends run off the canvas. Legends
/// with an explicit `guide_legend(ncol, nrow)` are only truncated.
pub fn measure_legends<'a>(
    boxes: &'a [LegendBox],
    theme: &Theme,
    budget: Option<LegendBudget>,
) -> Result<Option<LegendBlock<'a>>> {
    let horizontal = theme.legends_horizontal();
    let side_by_side = theme.legend_box_horizontal;
    let spacing = if side_by_side {
        theme.legend_spacing.0
    } else {
        theme.legend_spacing.1
    };
    let along = |size: (f64, f64)| if side_by_side { size.0 } else { size.1 };
    let shown: Vec<&LegendBox> = boxes
        .iter()
        .filter(|legend_box| !legend_box.is_empty())
        .collect();
    let measure_all = |budget: Option<LegendBudget>| -> Result<Vec<MeasuredLegend<'a>>> {
        shown
            .iter()
            .map(|legend_box| match legend_box {
                LegendBox::Keys(legend) => fit_legend(legend, horizontal, theme, budget),
                LegendBox::Colorbar(colorbar) => Ok(measure_colorbar(colorbar, horizontal, theme)),
            })
            .collect()
    };
    let mut measured = measure_all(budget)?;
    if measured.is_empty() {
        return Ok(None);
    }
    // Stacked legends share the budget along the stack: key legends split what
    // colorbars and spacing leave, equally.
    if let Some(budget) = budget {
        let total = measured
            .iter()
            .map(|legend| along(legend.size))
            .sum::<f64>()
            + spacing * (measured.len() - 1) as f64;
        let keys = shown
            .iter()
            .filter(|legend_box| matches!(legend_box, LegendBox::Keys(_)))
            .count();
        if keys > 1 && total > along(budget.preferred) + 1e-9 {
            let fixed: f64 = shown
                .iter()
                .zip(&measured)
                .filter(|(legend_box, _)| matches!(legend_box, LegendBox::Colorbar(_)))
                .map(|(_, legend)| along(legend.size))
                .sum::<f64>()
                + spacing * (measured.len() - 1) as f64;
            let share = |size: (f64, f64)| {
                let part = ((along(size) - fixed) / keys as f64).max(1.0);
                if side_by_side {
                    (part, size.1)
                } else {
                    (size.0, part)
                }
            };
            measured = measure_all(Some(LegendBudget {
                preferred: share(budget.preferred),
                most: share(budget.most),
            }))?;
        }
    }
    let justification = theme.legend_box_justification();
    let across = |size: (f64, f64)| if side_by_side { size.1 } else { size.0 };
    let thickness = measured
        .iter()
        .map(|legend| across(legend.size))
        .fold(0.0, f64::max);
    let mut legends = Vec::with_capacity(measured.len());
    let mut along = 0.0;
    for legend in measured {
        let shift = (thickness - across(legend.size)) * justification;
        let (offset, extent) = if side_by_side {
            ((along, shift), legend.size.0)
        } else {
            ((shift, along), legend.size.1)
        };
        along += extent + spacing;
        legends.push((legend, offset));
    }
    let length = along - spacing;
    let size = if side_by_side {
        (length, thickness)
    } else {
        (thickness, length)
    };
    Ok(Some(LegendBlock { legends, size }))
}

fn measure_colorbar<'a>(
    colorbar: &'a Colorbar,
    horizontal: bool,
    theme: &Theme,
) -> MeasuredLegend<'a> {
    let graph = layout_colorbar(colorbar, (0.0, 0.0), horizontal, theme);
    let (origin, title_anchor, size) =
        arrange_title(&colorbar.title, graph.size, horizontal, theme);
    MeasuredLegend {
        title: &colorbar.title,
        title_anchor,
        size,
        content: Measured::Colorbar(colorbar, graph.translated(origin)),
    }
}

/// Keys a legend needs at least before it is truncated further.
const MIN_SHOWN_KEYS: usize = 1;

/// [`measure_legend`] within `budget`: the natural grid if it fits, else more
/// columns (vertical) or rows (horizontal), else the fewest keys dropped.
fn fit_legend<'a>(
    legend: &'a Legend,
    horizontal: bool,
    theme: &Theme,
    budget: Option<LegendBudget>,
) -> Result<MeasuredLegend<'a>> {
    let count = legend.keys.len();
    let fixed = legend.grid.columns.is_some() || legend.grid.rows.is_some();
    let natural = measure_legend(
        legend,
        count,
        legend.grid,
        LABEL_LINE_LENGTH,
        horizontal,
        theme,
    )?;
    let Some(budget) = budget.map(|budget| if fixed { budget.most } else { budget.preferred })
    else {
        return Ok(natural);
    };
    let fits = |size: (f64, f64)| size.0 <= budget.0 + 1e-9 && size.1 <= budget.1 + 1e-9;
    if fits(natural.size) {
        return Ok(natural);
    }
    // The first reflowed grid of `shown` keys with labels wrapped at `line_length`
    // that fits, if any.
    let reflow_wrapped = |shown: usize, line_length: usize| -> Result<Option<MeasuredLegend<'a>>> {
        let entries = shown + usize::from(shown < count);
        let natural_shape = legend.grid.shape(entries, horizontal);
        let grids: Vec<KeyGrid> = if fixed {
            vec![legend.grid]
        } else if horizontal {
            (natural_shape.1..=entries)
                .map(|rows| KeyGrid {
                    columns: None,
                    rows: Some(rows),
                    by_row: false,
                })
                .collect()
        } else {
            (natural_shape.0..=entries)
                .map(|columns| KeyGrid {
                    columns: Some(columns),
                    rows: None,
                    by_row: false,
                })
                .collect()
        };
        for grid in grids {
            let measured = measure_legend(legend, shown, grid, line_length, horizontal, theme)?;
            if fits(measured.size) {
                return Ok(Some(measured));
            }
            // More columns (rows) only grow the block across: stop once it is too wide
            // (tall).
            let across = if horizontal {
                measured.size.1
            } else {
                measured.size.0
            };
            let limit = if horizontal { budget.1 } else { budget.0 };
            if across > limit + 1e-9 {
                break;
            }
        }
        Ok(None)
    };
    // The first reflowed grid of `shown` keys that fits, labels wrapped ever
    // narrower, if any.
    let reflow = |shown: usize| -> Result<Option<MeasuredLegend<'a>>> {
        for line_length in std::iter::once(LABEL_LINE_LENGTH).chain(NARROW_LINE_LENGTHS) {
            if let Some(measured) = reflow_wrapped(shown, line_length)? {
                return Ok(Some(measured));
            }
        }
        Ok(None)
    };
    if let Some(measured) = reflow(count)? {
        return Ok(measured);
    }
    // Binary search the most keys that fit (fewer keys never need more space).
    let (mut low, mut high) = (MIN_SHOWN_KEYS.min(count), count - 1);
    let mut best = None;
    while low <= high && high > 0 {
        let middle = (low + high).div_ceil(2);
        match reflow(middle)? {
            Some(measured) => {
                best = Some(measured);
                low = middle + 1;
            }
            None => high = middle - 1,
        }
    }
    match best {
        Some(measured) => Ok(measured),
        None => {
            let shown = MIN_SHOWN_KEYS.min(count);
            let narrowest = NARROW_LINE_LENGTHS[NARROW_LINE_LENGTHS.len() - 1];
            measure_legend(legend, shown, legend.grid, narrowest, horizontal, theme)
        }
    }
}

/// Measure the first `shown` keys of `legend` in `grid`; fewer than all keys adds
/// a label-only "+K more" entry.
fn measure_legend<'a>(
    legend: &'a Legend,
    shown: usize,
    grid: KeyGrid,
    line_length: usize,
    horizontal: bool,
    theme: &Theme,
) -> Result<MeasuredLegend<'a>> {
    let hidden = legend.keys.len() - shown;
    let (label_gap, label_extra) = match theme.legend_text {
        // lets-plot: half the width of "_" between key and label; a third of a line
        // between labels.
        Some(style) => (
            measure("_", style.size, style.font).width / 2.0,
            text_band(style) / 3.0,
        ),
        None => (0.0, 0.0),
    };
    let label_size = |lines: &[Label]| match theme.legend_text {
        Some(style) => (
            lines
                .iter()
                .map(|line| {
                    measure_label(
                        &line.text,
                        line.superscript.as_deref(),
                        line.suffix.as_deref(),
                        style.size,
                        style.font,
                    )
                })
                .fold(0.0, f64::max),
            text_band(style) * lines.len() as f64,
        ),
        None => (0.0, 0.0),
    };
    let mut labels: Vec<Vec<Label>> = legend.keys[..shown]
        .iter()
        .map(|key| label_lines(&key.label, line_length))
        .collect();
    if hidden > 0 {
        labels.push(vec![Label::plain(format!("+{hidden} more"))]);
    }

    let mut key_sizes = Vec::with_capacity(labels.len());
    for key in &legend.keys[..shown] {
        let mut size = (0.0_f64, 0.0_f64);
        for layer in &key.layers {
            let (width, height) = layer.geom.minimum_key_size(&layer.styles)?;
            size.0 = size.0.max(theme.legend_key_size.max(pretty(width)));
            size.1 = size.1.max(theme.legend_key_size.max(pretty(height)));
        }
        key_sizes.push(size);
    }
    if hidden > 0 {
        key_sizes.push((theme.legend_key_size, theme.legend_key_size));
    }
    // lets-plot: every key as tall as the tallest in a horizontal legend, as wide as
    // the widest in a vertical one.
    if horizontal {
        let height = key_sizes.iter().map(|size| size.1).fold(0.0, f64::max);
        key_sizes.iter_mut().for_each(|size| size.1 = height);
    } else {
        let width = key_sizes.iter().map(|size| size.0).fold(0.0, f64::max);
        key_sizes.iter_mut().for_each(|size| size.0 = width);
    }

    let count = labels.len();
    let shape = grid.shape(count, horizontal);
    let (columns, rows) = shape;
    let mut column_widths = vec![0.0_f64; columns];
    let mut row_heights = vec![0.0_f64; rows];
    let mut label_offsets = Vec::with_capacity(count);
    for index in 0..count {
        let (column, row) = grid.position(index, shape);
        let extra = if row == rows - 1 { 0.0 } else { label_extra };
        let (label_width, label_height) = label_size(&labels[index]);
        let (key_width, key_height) = key_sizes[index];
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

    let mut cells = Vec::with_capacity(count);
    let (mut graph_width, mut graph_height) = (0.0_f64, 0.0_f64);
    for index in 0..count {
        let (column, row) = grid.position(index, shape);
        let (x, y) = (column_x[column], row_y[row]);
        graph_width = graph_width.max(x + column_widths[column]);
        graph_height = graph_height.max(y + row_heights[row]);
        // lets-plot centres each key (and its label) vertically in its row.
        let shift = (row_heights[row] - key_sizes[index].1) / 2.0;
        let cell = Rect {
            x,
            y: y + shift,
            width: key_sizes[index].0,
            height: key_sizes[index].1,
        };
        let (dx, dy) = label_offsets[index];
        let key = (index < shown).then_some(index);
        cells.push((key, cell, (x + dx, y + shift + dy), labels[index].clone()));
    }
    let (origin, title_anchor, size) = arrange_title(
        &legend.title,
        (graph_width, graph_height),
        horizontal,
        theme,
    );
    let cells = cells
        .into_iter()
        .map(|(key, cell, (x, y), lines)| {
            let cell = Rect {
                x: cell.x + origin.0,
                y: cell.y + origin.1,
                ..cell
            };
            (key, cell, (x + origin.0, y + origin.1), lines)
        })
        .collect();
    Ok(MeasuredLegend {
        title: &legend.title,
        title_anchor,
        size,
        content: Measured::Keys(legend, cells),
    })
}

/// Lines of a label: plain labels wrap; a power label (`2·10` + superscript) stays
/// on one line.
fn label_lines(label: &Label, line_length: usize) -> Vec<Label> {
    if label.superscript.is_some() {
        return vec![label.clone()];
    }
    wrap_label(&label.text, line_length)
        .into_iter()
        .map(Label::plain)
        .collect()
}

/// Label lines, lets-plot `RichText.wrap(label, 30, 5)`: split at newlines, then
/// words fill lines of up to `line_length` characters (30 in lets-plot; longer
/// words are cut); 5 lines or more keep the first 5 and add a line "...".
fn wrap_label(label: &str, line_length: usize) -> Vec<String> {
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
            if needed <= line_length {
                if length > 0 {
                    line.push(' ');
                }
                line.push_str(word);
                length = needed;
            } else if word_length <= line_length {
                lines.push(std::mem::take(&mut line));
                line.push_str(word);
                length = word_length;
            } else {
                // Cut the word: fill the current line, then whole chunks.
                let mut characters: Vec<char> = word.chars().collect();
                if length > 0 {
                    let room = line_length.saturating_sub(length + 1);
                    if room > 0 {
                        line.push(' ');
                        line.extend(characters.drain(..room.min(characters.len())));
                    }
                    lines.push(std::mem::take(&mut line));
                }
                let chunks: Vec<String> = characters
                    .chunks(line_length)
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
    for (measured, offset) in &block.legends {
        let (x, y) = (origin.0 + offset.0, origin.1 + offset.1);
        let mut legend_nodes = Vec::new();
        if let Some(style) = theme.legend_background {
            let rect = Rect {
                x,
                y,
                width: measured.size.0,
                height: measured.size.1,
            };
            legend_nodes.push(super::rect_node(rect, style));
        }
        if let Some(style) = theme.legend_title
            && !measured.title.trim().is_empty()
        {
            let band = text_band(style);
            let (left, top) = (x + measured.title_anchor.0, y + measured.title_anchor.1);
            for (index, line) in title_lines(measured.title).into_iter().enumerate() {
                legend_nodes.push(text_node(
                    line,
                    style,
                    (left, top + band * (index as f64 + 0.5)),
                    HorizontalAlign::Start,
                    0.0,
                ));
            }
        }
        match &measured.content {
            Measured::Colorbar(colorbar, layout) => {
                legend_nodes.extend(draw_colorbar(colorbar, &layout.translated((x, y)), theme));
            }
            Measured::Keys(legend, cells) => {
                for (key, cell, label_anchor, lines) in cells {
                    let cell = Rect {
                        x: x + cell.x,
                        y: y + cell.y,
                        ..*cell
                    };
                    if let Some(key) = key.map(|index| &legend.keys[index]) {
                        if let Some(style) = theme.legend_key_background {
                            legend_nodes.push(super::rect_node(cell, style));
                        }
                        for layer in &key.layers {
                            legend_nodes.extend(layer.geom.draw_key(&layer.styles, cell)?);
                        }
                    }
                    if let Some(style) = theme.legend_text {
                        // Lines centred as a block on the key's middle.
                        let band = text_band(style);
                        let first = label_anchor.1 - band * (lines.len() as f64 - 1.0) / 2.0;
                        for (line_index, line) in lines.iter().enumerate() {
                            legend_nodes.push(label_node(
                                line,
                                style,
                                (x + label_anchor.0, y + first + band * line_index as f64),
                                HorizontalAlign::Start,
                                0.0,
                            ));
                        }
                    }
                }
            }
        }
        nodes.push(Node::Group(Group {
            clip: None,
            role: Some("legend"),
            anchor: None,
            nodes: legend_nodes,
        }));
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
        assert_eq!(wrap_label("short", LABEL_LINE_LENGTH), ["short"]);
        assert_eq!(
            wrap_label(
                "this is a fairly long label with spaces in it ok",
                LABEL_LINE_LENGTH
            ),
            ["this is a fairly long label", "with spaces in it ok"]
        );
        let long = "a".repeat(60);
        assert_eq!(
            wrap_label(&long, LABEL_LINE_LENGTH),
            ["a".repeat(30), "a".repeat(30)]
        );
        assert_eq!(wrap_label("one\ntwo", LABEL_LINE_LENGTH), ["one", "two"]);
        assert_eq!(
            wrap_label("1\n2\n3\n4\n5\n6", LABEL_LINE_LENGTH),
            ["1", "2", "3", "4", "5", "..."]
        );
    }

    #[test]
    fn key_grids_follow_lets_plot() {
        let grid = |columns, rows, by_row| KeyGrid {
            columns,
            rows,
            by_row,
        };
        // Defaults: 15 rows per column (vertical), 5 columns per row (horizontal).
        assert_eq!(KeyGrid::default().shape(7, false), (1, 7));
        assert_eq!(KeyGrid::default().shape(20, false), (2, 15));
        assert_eq!(KeyGrid::default().shape(7, true), (4, 2));
        assert_eq!(grid(None, None, true).shape(7, true), (5, 2));
        assert_eq!(grid(Some(2), None, false).shape(7, false), (2, 4));
        assert_eq!(grid(None, Some(2), true).shape(7, false), (4, 2));
        // Counts larger than the keys are capped.
        assert_eq!(grid(Some(10), None, true).shape(3, false), (3, 1));
        assert_eq!(grid(None, Some(10), false).shape(3, false), (1, 3));
        let by_column = grid(Some(2), None, false);
        assert_eq!(by_column.position(5, (2, 4)), (1, 1));
        let by_row = grid(Some(2), None, true);
        assert_eq!(by_row.position(5, (2, 4)), (1, 2));
    }

    #[test]
    fn combined_grids_keep_the_last_counts_and_any_byrow() {
        let first = KeyGrid {
            columns: Some(2),
            rows: None,
            by_row: true,
        };
        let second = KeyGrid {
            columns: Some(3),
            rows: Some(1),
            by_row: false,
        };
        assert_eq!(
            first.combine(second),
            KeyGrid {
                columns: Some(3),
                rows: Some(1),
                by_row: true
            }
        );
        assert_eq!(first.combine(KeyGrid::default()), first);
    }

    #[test]
    fn offsets_accumulate_with_spacing() {
        assert_eq!(running_offsets(&[10.0, 20.0, 5.0], 2.0), [0.0, 12.0, 34.0]);
    }
}
