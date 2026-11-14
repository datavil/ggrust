//! Legends (key boxes and colorbars): measure and draw (lets-plot `LegendComponentLayout`,
//! `LegendBreak`, `LegendBoxesLayoutUtil`).
//!
//! A [`Legend`] is a title plus keys; each key draws one entry per layer through
//! its geom. Keys are at least `legend_key_width` x `legend_key_height` (lets-plot
//! rounds a geom's minimum key size up to an odd number + 1). Keys fill a grid: by default up to 15
//! rows per column (vertical legends) or 5 columns per row (horizontal legends),
//! or as `guide_legend(ncol, nrow, byrow)` asks. Horizontal legends put the title
//! at the left. Several legends stack vertically, or side by side with
//! `legend_box="horizontal"`. Text is measured with the real font, as everywhere
//! in the layout. A legend with `bin_edges` is ggplot2's `guide_bins()`: touching
//! keys with labels on their edges along an axis line.
//!
//! Where the block goes (`legend_position`) is decided by the plot builder.

use super::colorbar::{Colorbar, ColorbarLayout, draw_colorbar, layout_colorbar};
use super::{label_node, text_band, text_node};
use crate::geom::Geom;
use crate::interactive_legend::{key_tag, legend_key_id};
use crate::scale::format::Label;
use crate::scene::{Group, HorizontalAlign, LEGEND_KEY_ROLE, Node, Rect};
use crate::style::Styles;
use crate::text::{measure, measure_label};
use crate::theme::{LegendPosition, LegendTitlePosition, Theme};
use crate::{Error, Result};

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
    /// How many earlier keys of the legend carry the same label (repeated
    /// labels with other values keep their own keys).
    pub occurrence: usize,
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
    /// `guide_legend(direction, theme)`.
    pub style: GuideStyle,
    /// `guide_bins()` (ggplot2): one label per bin edge (keys + 1, low end first,
    /// blank ends unless `show_limits`). The keys touch in one row or column with
    /// the labels on their edges along an axis line; `None`: a plain legend.
    pub bin_edges: Option<Vec<Label>>,
}

/// Options of one guide box over the block's (ggplot2 `guide_*(direction=, theme=)`).
#[derive(Debug, Clone, Default)]
pub struct GuideStyle {
    /// `direction`: keys in rows (`Some(true)`) or columns; `None` follows the block.
    pub horizontal: Option<bool>,
    /// The plot theme with the guide's `theme(legend_*=...)` on top.
    pub theme: Option<Box<Theme>>,
    /// `reverse`: keys listed backwards, a colorbar's high values at its start
    /// (applied by the plot builder).
    pub reverse: Option<bool>,
    /// `order`, 0 to 99 (0 or unset: last); the plot builder sorts boxes by it.
    pub order: Option<u32>,
    /// `position`: the side (or the panel inside) this guide goes to; `None`
    /// follows `legend_position` (ggplot2 3.5).
    pub position: Option<GuidePosition>,
}

/// Whether two guide themes are the same: both unset, or equal field by field with
/// NaN equal to NaN (unset text justifications are NaN, so the derived `==` never
/// matches). Compared through their debug text, which prints every field.
fn same_theme(left: &Option<Box<Theme>>, right: &Option<Box<Theme>>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => format!("{left:?}") == format!("{right:?}"),
        _ => false,
    }
}

/// Where one guide goes (ggplot2 `guide_*(position=)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuidePosition {
    /// Above the panel.
    Top,
    /// Right of the panel.
    Right,
    /// Below the panel.
    Bottom,
    /// Left of the panel.
    Left,
    /// Inside the panel, at `legend_position_inside`.
    Inside,
}

impl GuidePosition {
    /// The accepted names, in ggplot2's order.
    pub const NAMES: [&'static str; 5] = ["top", "right", "bottom", "left", "inside"];

    /// The position named `name` (`None`: not a position).
    pub fn parse(name: &str) -> Option<GuidePosition> {
        Some(match name {
            "top" => GuidePosition::Top,
            "right" => GuidePosition::Right,
            "bottom" => GuidePosition::Bottom,
            "left" => GuidePosition::Left,
            "inside" => GuidePosition::Inside,
            _ => return None,
        })
    }

    /// The legend position this guide takes in a plot with `theme`: an inside
    /// guide sits at `theme`'s inside point.
    pub fn resolve(self, theme: &Theme) -> LegendPosition {
        match self {
            GuidePosition::Top => LegendPosition::Top,
            GuidePosition::Right => LegendPosition::Right,
            GuidePosition::Bottom => LegendPosition::Bottom,
            GuidePosition::Left => LegendPosition::Left,
            GuidePosition::Inside => {
                let (x, y) = theme.inside_point();
                LegendPosition::Inside(x, y)
            }
        }
    }
}

impl GuideStyle {
    /// `self`, with `other`'s options where `self` leaves one unset.
    pub fn or(self, other: GuideStyle) -> GuideStyle {
        GuideStyle {
            horizontal: self.horizontal.or(other.horizontal),
            theme: self.theme.or(other.theme),
            reverse: self.reverse.or(other.reverse),
            order: self.order.or(other.order),
            position: self.position.or(other.position),
        }
    }
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
    /// The guide's own direction and theme.
    pub fn style(&self) -> &GuideStyle {
        match self {
            LegendBox::Keys(legend) => &legend.style,
            LegendBox::Colorbar(colorbar) => &colorbar.style,
        }
    }

    /// Where the box goes in a plot with `theme`: its guide's `position`, else
    /// `legend_position`. An inside guide uses its own theme's inside point first.
    pub fn position(&self, theme: &Theme) -> LegendPosition {
        let style = self.style();
        match style.position {
            None => theme.legend_position,
            Some(position) => position.resolve(style.theme.as_deref().unwrap_or(theme)),
        }
    }

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
                left.style.horizontal == right.style.horizontal
                    && same_theme(&left.style.theme, &right.style.theme)
                    && left.title == right.title
                    && left.grid == right.grid
                    && left.bin_edges == right.bin_edges
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
                left.style.horizontal == right.style.horizontal
                    && same_theme(&left.style.theme, &right.style.theme)
                    && left.reverse == right.reverse
                    && left.limit_ticks == right.limit_ticks
                    && left.title == right.title
                    && left.domain == right.domain
                    && left.breaks == right.breaks
                    && left.colors == right.colors
                    && left.bar_width == right.bar_width
                    && left.bar_height == right.bar_height
                    && left.label_ends == right.label_ends
                    && left.na == right.na
                    && left.caps == right.caps
                    && left.cap_size == right.cap_size
                    && left.minor == right.minor
            }
            _ => false,
        }
    }

    /// Whether the box shows nothing (no keys, or a colorbar without breaks); such
    /// boxes are not drawn.
    pub fn is_empty(&self) -> bool {
        match self {
            LegendBox::Keys(legend) => legend.keys.is_empty(),
            LegendBox::Colorbar(colorbar) => colorbar.breaks.is_empty(),
        }
    }
}

/// A box measured for drawing; positions relative to its box origin.
struct MeasuredLegend<'a> {
    /// The theme the box is measured and drawn with (its figure's, in a grid that
    /// collects legends); `None`: the block's theme.
    theme: Option<&'a Theme>,
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
    /// `guide_bins()`: the key cells (one per key), the edge labels with their
    /// anchors (vertical middle) and alignment, the axis line and its ticks.
    Bins(&'a Legend, BinsLayout),
}

/// A measured `guide_bins()`.
struct BinsLayout {
    cells: Vec<Rect>,
    labels: Vec<((f64, f64), HorizontalAlign)>,
    axis: [(f64, f64); 2],
    ticks: Vec<[(f64, f64); 2]>,
}

impl BinsLayout {
    fn translated(self, (dx, dy): (f64, f64)) -> BinsLayout {
        let point = |(x, y): (f64, f64)| (x + dx, y + dy);
        BinsLayout {
            cells: self
                .cells
                .into_iter()
                .map(|cell| Rect {
                    x: cell.x + dx,
                    y: cell.y + dy,
                    ..cell
                })
                .collect(),
            labels: self
                .labels
                .into_iter()
                .map(|(anchor, align)| (point(anchor), align))
                .collect(),
            axis: self.axis.map(point),
            ticks: self.ticks.into_iter().map(|tick| tick.map(point)).collect(),
        }
    }
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

/// Where a guide's title goes: `legend_title_position`, else at the top of vertical
/// guides and the left of horizontal ones (ggplot2 3.5, lets-plot).
pub(super) fn title_position(theme: &Theme, horizontal: bool) -> LegendTitlePosition {
    theme.legend_title_position.unwrap_or(if horizontal {
        LegendTitlePosition::Left
    } else {
        LegendTitlePosition::Top
    })
}

/// `legend_title` justification: hjust (unset: 0, left) and vjust (unset: 0.5,
/// centred), each clamped to 0..1.
fn title_justification(theme: &Theme) -> (f64, f64) {
    let pick = |value: Option<f64>, default: f64| match value {
        Some(value) if value.is_finite() => value.clamp(0.0, 1.0),
        _ => default,
    };
    (
        pick(theme.legend_title.map(|style| style.hjust), 0.0),
        pick(theme.legend_title.map(|style| style.vjust), 0.5),
    )
}

/// Place a title and a graph (keys or bar) in a box (lets-plot `LegendBoxLayout`,
/// ggplot2 3.5 `legend.title.position`). A title above or below the graph is half
/// a title line from it; one at the left touches it (lets-plot) and one at the
/// right is half a line away. Above or below, the title and the graph are both
/// placed across the box width by the title's hjust (ggplot2 `Guide$add_title`);
/// left or right, down the box height by its vjust. Returns the graph origin, the
/// title anchor and the box size, margins included.
fn arrange_title(
    title: &str,
    graph: (f64, f64),
    horizontal: bool,
    theme: &Theme,
) -> ((f64, f64), (f64, f64), (f64, f64)) {
    let margin = theme.legend_margin;
    let (text_width, text_height) = title_size(title, theme);
    // `legend_title` margins pad the title box (ggplot2); unset, they are zero.
    let title_margin = theme.legend_title_margin;
    let (title_width, title_height) = if text_width > 0.0 {
        (
            text_width + title_margin.left + title_margin.right,
            text_height + title_margin.top + title_margin.bottom,
        )
    } else {
        (0.0, 0.0)
    };
    let half_line = match theme.legend_title {
        Some(style) if title_height > 0.0 => text_band(style) / 2.0,
        _ => 0.0,
    };
    let (hjust, vjust) = title_justification(theme);
    let (graph_origin, title_origin, content) = match title_position(theme, horizontal) {
        position @ (LegendTitlePosition::Left | LegendTitlePosition::Right) => {
            let height = title_height.max(graph.1);
            let title_y = (1.0 - vjust) * (height - title_height);
            let graph_y = (1.0 - vjust) * (height - graph.1);
            let gap = if title_width > 0.0 && position == LegendTitlePosition::Right {
                half_line
            } else {
                0.0
            };
            let width = title_width + gap + graph.0;
            if position == LegendTitlePosition::Left {
                ((title_width, graph_y), (0.0, title_y), (width, height))
            } else {
                ((0.0, graph_y), (graph.0 + gap, title_y), (width, height))
            }
        }
        position @ (LegendTitlePosition::Top | LegendTitlePosition::Bottom) => {
            let width = title_width.max(graph.0);
            let title_x = hjust * (width - title_width);
            let graph_x = hjust * (width - graph.0);
            // lets-plot `LegendBoxLayout.graphOrigin`: half a title line between.
            let gap = if title_height > 0.0 { half_line } else { 0.0 };
            let height = title_height + gap + graph.1;
            if position == LegendTitlePosition::Top {
                (
                    (graph_x, title_height + gap),
                    (title_x, 0.0),
                    (width, height),
                )
            } else {
                ((graph_x, 0.0), (title_x, graph.1 + gap), (width, height))
            }
        }
    };
    (
        (margin.left + graph_origin.0, margin.top + graph_origin.1),
        (
            margin.left + title_origin.0 + title_margin.left,
            margin.top + title_origin.1 + title_margin.top,
        ),
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
    measure_themed_legends(boxes, None, theme, budget)
}

/// [`measure_legends`] with a theme per box (`box_themes`, one per box): each box
/// is measured and drawn with its own theme, while `theme` arranges the block. A
/// grid collecting legends from figures with different themes uses this.
pub fn measure_themed_legends<'a>(
    boxes: &'a [LegendBox],
    box_themes: Option<&'a [Theme]>,
    theme: &Theme,
    budget: Option<LegendBudget>,
) -> Result<Option<LegendBlock<'a>>> {
    measure_legends_where(boxes, box_themes, theme, budget, |_| true)
}

/// [`measure_themed_legends`] for the boxes `keep` accepts only (the guides at
/// one position, when guides go to several sides).
pub fn measure_legends_where<'a>(
    boxes: &'a [LegendBox],
    box_themes: Option<&'a [Theme]>,
    theme: &Theme,
    budget: Option<LegendBudget>,
    keep: impl Fn(&LegendBox) -> bool,
) -> Result<Option<LegendBlock<'a>>> {
    if let Some(themes) = box_themes
        && themes.len() != boxes.len()
    {
        return Err(Error::spec(
            "",
            format!(
                "internal: {} legend themes for {} legend boxes",
                themes.len(),
                boxes.len()
            ),
        ));
    }
    let horizontal = theme.legends_horizontal();
    let side_by_side = theme.legend_box_horizontal;
    let spacing = if side_by_side {
        theme.legend_spacing.0
    } else {
        theme.legend_spacing.1
    };
    let along = |size: (f64, f64)| if side_by_side { size.0 } else { size.1 };
    let (shown, shown_themes): (Vec<&LegendBox>, Vec<Option<&'a Theme>>) = boxes
        .iter()
        .enumerate()
        .filter(|(_, legend_box)| !legend_box.is_empty() && keep(legend_box))
        .map(|(index, legend_box)| (legend_box, box_themes.map(|themes| &themes[index])))
        .unzip();
    let measure_one = |index: usize, budget: Option<LegendBudget>| {
        // The guide's own theme, else the box's figure theme, else the block's.
        let style = shown[index].style();
        let own = style.theme.as_deref().or(shown_themes[index]);
        let box_theme = own.unwrap_or(theme);
        let horizontal = style.horizontal.unwrap_or(horizontal);
        let mut measured = match shown[index] {
            LegendBox::Keys(legend) => fit_legend(legend, horizontal, box_theme, budget)?,
            LegendBox::Colorbar(colorbar) => measure_colorbar(colorbar, horizontal, box_theme),
        };
        measured.theme = own;
        Ok::<MeasuredLegend<'a>, Error>(measured)
    };
    let mut measured = (0..shown.len())
        .map(|index| measure_one(index, budget))
        .collect::<Result<Vec<MeasuredLegend<'a>>>>()?;
    if measured.is_empty() {
        return Ok(None);
    }
    // Legends too long for one stack wrap into more stacks side by side (issue
    // lets-plot#1506), if those fit the budget across.
    if let Some(budget) = budget
        && let Some((offsets, size)) = wrap_stacks(&measured, budget.preferred, theme)
    {
        let legends = measured.into_iter().zip(offsets).collect();
        return Ok(Some(LegendBlock { legends, size }));
    }
    // Otherwise stacked legends share the budget along the stack: key legends
    // split what colorbars and spacing leave. Legends that fit an equal share
    // keep their size; the rest split what those leave (water-filling).
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
            let needs: Vec<Option<f64>> = shown
                .iter()
                .zip(&measured)
                .map(|(legend_box, legend)| {
                    matches!(legend_box, LegendBox::Keys(_)).then(|| along(legend.size))
                })
                .collect();
            let preferred = water_fill(&needs, along(budget.preferred) - fixed);
            let most = water_fill(&needs, along(budget.most) - fixed);
            let with_along = |size: (f64, f64), part: f64| {
                if side_by_side {
                    (part, size.1)
                } else {
                    (size.0, part)
                }
            };
            measured = (0..shown.len())
                .zip(preferred.iter().zip(&most))
                .map(|(index, (preferred, most))| {
                    measure_one(
                        index,
                        Some(LegendBudget {
                            preferred: with_along(budget.preferred, *preferred),
                            most: with_along(budget.most, *most),
                        }),
                    )
                })
                .collect::<Result<_>>()?;
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

/// Split `room` among the legends with a need (`None`: colorbars, not split):
/// in order of need, each takes its need if it is within an equal share of what
/// is left, else that share (at least 1 px).
fn water_fill(needs: &[Option<f64>], room: f64) -> Vec<f64> {
    let mut order: Vec<usize> = (0..needs.len())
        .filter(|&index| needs[index].is_some())
        .collect();
    order.sort_by(|&a, &b| needs[a].unwrap_or(0.0).total_cmp(&needs[b].unwrap_or(0.0)));
    let mut parts = vec![f64::INFINITY; needs.len()];
    let mut left = room;
    for (taken, &index) in order.iter().enumerate() {
        let share = left / (order.len() - taken) as f64;
        let part = needs[index].unwrap_or(0.0).min(share).max(1.0);
        parts[index] = part;
        left -= part;
    }
    parts
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
        theme: None,
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
    if let Some(edges) = &legend.bin_edges
        && !legend.keys.is_empty()
        && edges.len() == legend.keys.len() + 1
    {
        // Too large for its room: the plain legend's reflow and "+K more" keep it
        // on the canvas.
        let bins = measure_bins(legend, edges, horizontal, theme)?;
        let fits = budget.is_none_or(|budget| {
            bins.size.0 <= budget.most.0 + 1e-9 && bins.size.1 <= budget.most.1 + 1e-9
        });
        if fits {
            return Ok(bins);
        }
    }
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
            size.0 = size.0.max(theme.legend_key_size.0.max(pretty(width)));
            size.1 = size.1.max(theme.legend_key_size.1.max(pretty(height)));
        }
        key_sizes.push(size);
    }
    if hidden > 0 {
        key_sizes.push(theme.legend_key_size);
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
        theme: None,
        title: &legend.title,
        title_anchor,
        size,
        content: Measured::Keys(legend, cells),
    })
}

/// `guide_bins()` (ggplot2 `GuideBins`): every key as large as the largest, the
/// keys touching in one column (low bin at the top) or row (low bin at the left),
/// an axis line on the label side with a tick toward the keys at each labelled
/// edge, and the edge labels beside it.
fn measure_bins<'a>(
    legend: &'a Legend,
    edges: &[Label],
    horizontal: bool,
    theme: &Theme,
) -> Result<MeasuredLegend<'a>> {
    let mut key = theme.legend_key_size;
    for entry in &legend.keys {
        for layer in &entry.layers {
            let (width, height) = layer.geom.minimum_key_size(&layer.styles)?;
            key.0 = key.0.max(pretty(width));
            key.1 = key.1.max(pretty(height));
        }
    }
    let (label_gap, band) = match theme.legend_text {
        Some(style) => (
            measure("_", style.size, style.font).width / 2.0,
            text_band(style),
        ),
        None => (0.0, 0.0),
    };
    let label_width = |label: &Label| match theme.legend_text {
        Some(style) if !label.text.is_empty() => measure_label(
            &label.text,
            label.superscript.as_deref(),
            label.suffix.as_deref(),
            style.size,
            style.font,
        ),
        _ => 0.0,
    };
    let shown = |label: &Label| !label.text.is_empty() || label.superscript.is_some();
    let count = legend.keys.len();
    let tick = theme.legend_ticks_length.0;
    let last = edges.len() - 1;
    let (cells, labels, axis, ticks, graph) = if horizontal {
        // Room for the end labels, centred on the ends.
        let left = label_width(&edges[0]) / 2.0;
        let right = label_width(&edges[last]) / 2.0;
        let along = |index: usize| left + key.0 * index as f64;
        let cells = (0..count)
            .map(|index| Rect {
                x: along(index),
                y: 0.0,
                width: key.0,
                height: key.1,
            })
            .collect();
        let label_y = key.1 + label_gap + band / 2.0;
        let labels = (0..edges.len())
            .map(|index| ((along(index), label_y), HorizontalAlign::Middle))
            .collect();
        let ticks = (0..edges.len())
            .filter(|index| shown(&edges[*index]))
            .map(|index| [(along(index), key.1 - tick), (along(index), key.1)])
            .collect();
        let axis = [(along(0), key.1), (along(count), key.1)];
        let size = (
            left + key.0 * count as f64 + right,
            key.1 + if band > 0.0 { label_gap + band } else { 0.0 },
        );
        (cells, labels, axis, ticks, size)
    } else {
        // Room for the end labels, centred on the ends.
        let top = if shown(&edges[0]) { band / 2.0 } else { 0.0 };
        let bottom = if shown(&edges[last]) { band / 2.0 } else { 0.0 };
        let along = |index: usize| top + key.1 * index as f64;
        let cells = (0..count)
            .map(|index| Rect {
                x: 0.0,
                y: along(index),
                width: key.0,
                height: key.1,
            })
            .collect();
        let labels = (0..edges.len())
            .map(|index| ((key.0 + label_gap, along(index)), HorizontalAlign::Start))
            .collect();
        let ticks = (0..edges.len())
            .filter(|index| shown(&edges[*index]))
            .map(|index| [(key.0 - tick, along(index)), (key.0, along(index))])
            .collect();
        let axis = [(key.0, along(0)), (key.0, along(count))];
        let widest = edges.iter().map(label_width).fold(0.0, f64::max);
        let size = (
            key.0 + label_gap + widest,
            top + key.1 * count as f64 + bottom,
        );
        (cells, labels, axis, ticks, size)
    };
    let (origin, title_anchor, size) = arrange_title(&legend.title, graph, horizontal, theme);
    let layout = BinsLayout {
        cells,
        labels,
        axis,
        ticks,
    };
    Ok(MeasuredLegend {
        theme: None,
        title: &legend.title,
        title_anchor,
        size,
        content: Measured::Bins(legend, layout.translated(origin)),
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

/// Label lines, lets-plot `RichText.wrap(label, 30, 5)`: [`wrap_words`]; 5 lines
/// or more keep the first 5 and add a line "...".
fn wrap_label(label: &str, line_length: usize) -> Vec<String> {
    let mut lines = wrap_words(label, line_length);
    if lines.len() >= LABEL_MAX_LINES {
        lines.truncate(LABEL_MAX_LINES);
        lines.push("...".to_owned());
    }
    lines
}

/// Lines of a text, lets-plot `RichText.wrap` without a line limit: split at
/// newlines, then words fill lines of up to `line_length` characters (longer words
/// are cut). Also the facet strip `labwidth` wrap.
pub(crate) fn wrap_words(label: &str, line_length: usize) -> Vec<String> {
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

/// Offsets of legends wrapped into stacks, and the block size.
type WrappedStacks = (Vec<(f64, f64)>, (f64, f64));

/// Legends packed in order into stacks no longer than `budget` along the stack,
/// the stacks placed side by side (top to bottom for `legend_box="horizontal"`
/// rows). `None` when one stack already fits (nothing
/// changes), when a single legend is longer than the budget, or when the stacks
/// together are too thick for the budget across. Returns each legend's offset and
/// the block size.
fn wrap_stacks(
    measured: &[MeasuredLegend],
    budget: (f64, f64),
    theme: &Theme,
) -> Option<WrappedStacks> {
    let side_by_side = theme.legend_box_horizontal;
    let (along_spacing, across_spacing) = if side_by_side {
        theme.legend_spacing
    } else {
        (theme.legend_spacing.1, theme.legend_spacing.0)
    };
    let along = |size: (f64, f64)| if side_by_side { size.0 } else { size.1 };
    let across = |size: (f64, f64)| if side_by_side { size.1 } else { size.0 };
    let (budget_along, budget_across) = (along(budget), across(budget));
    let total = measured
        .iter()
        .map(|legend| along(legend.size))
        .sum::<f64>()
        + along_spacing * measured.len().saturating_sub(1) as f64;
    if measured.len() < 2
        || total <= budget_along + 1e-9
        || measured
            .iter()
            .any(|legend| along(legend.size) > budget_along + 1e-9)
    {
        return None;
    }
    // Greedy packing: each stack holds legend indices.
    let mut stacks: Vec<Vec<usize>> = vec![Vec::new()];
    let mut length = 0.0;
    for (index, legend) in measured.iter().enumerate() {
        let extent = along(legend.size);
        let stack = stacks.last_mut()?;
        if !stack.is_empty() && length + along_spacing + extent > budget_along + 1e-9 {
            stacks.push(vec![index]);
            length = extent;
        } else {
            length += if stack.is_empty() { 0.0 } else { along_spacing } + extent;
            stack.push(index);
        }
    }
    let thicknesses: Vec<f64> = stacks
        .iter()
        .map(|stack| {
            stack
                .iter()
                .map(|&index| across(measured[index].size))
                .fold(0.0, f64::max)
        })
        .collect();
    let thickness = thicknesses.iter().sum::<f64>() + across_spacing * (stacks.len() - 1) as f64;
    if thickness > budget_across + 1e-9 {
        return None;
    }
    let justification = theme.legend_box_justification();
    let mut offsets = vec![(0.0, 0.0); measured.len()];
    let mut longest: f64 = 0.0;
    let mut start = 0.0;
    for (stack, stack_thickness) in stacks.iter().zip(&thicknesses) {
        let mut position = 0.0;
        for &index in stack {
            let size = measured[index].size;
            let shift = start + (stack_thickness - across(size)) * justification;
            offsets[index] = if side_by_side {
                (position, shift)
            } else {
                (shift, position)
            };
            position += along(size) + along_spacing;
        }
        longest = longest.max(position - along_spacing);
        start += stack_thickness + across_spacing;
    }
    let size = if side_by_side {
        (longest, thickness)
    } else {
        (thickness, longest)
    };
    Some((offsets, size))
}

/// Draw the block with its top-left corner at `origin`.
pub fn draw_legends(block: &LegendBlock, origin: (f64, f64), theme: &Theme) -> Result<Vec<Node>> {
    let mut nodes = Vec::new();
    let block_theme = theme;
    for (measured, offset) in &block.legends {
        let theme = measured.theme.unwrap_or(block_theme);
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
            // Lines of a multi-line title line up by the title's hjust.
            let (hjust, _) = title_justification(theme);
            let width = title_size(measured.title, theme).0;
            for (index, line) in title_lines(measured.title).into_iter().enumerate() {
                let shift = hjust * (width - measure(line, style.size, style.font).width);
                legend_nodes.push(text_node(
                    line,
                    style,
                    (left + shift, top + band * (index as f64 + 0.5)),
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
                    // A real key's nodes go in a clickable group (interactive
                    // legend, HTML only); "+K more" stays plain.
                    let start = legend_nodes.len();
                    if let Some(index) = key {
                        draw_key(&legend.keys[*index], cell, theme, &mut legend_nodes)?;
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
                    if let Some(index) = key {
                        group_key(
                            legend,
                            &legend.keys[*index],
                            start,
                            theme,
                            &mut legend_nodes,
                        );
                    }
                }
            }
            Measured::Bins(legend, layout) => {
                let point = |(px, py): (f64, f64)| (x + px, y + py);
                for (index, cell) in layout.cells.iter().enumerate() {
                    let cell = Rect {
                        x: x + cell.x,
                        y: y + cell.y,
                        ..*cell
                    };
                    let start = legend_nodes.len();
                    draw_key(&legend.keys[index], cell, theme, &mut legend_nodes)?;
                    group_key(legend, &legend.keys[index], start, theme, &mut legend_nodes);
                }
                // ggplot2 `legend.axis.line`: a thin line in the text color.
                if let Some(style) = theme.legend_text {
                    let stroke = crate::theme::LineStyle {
                        color: style.color,
                        width: BINS_AXIS_WIDTH,
                        dash: Default::default(),
                    }
                    .stroke();
                    for points in std::iter::once(layout.axis).chain(layout.ticks.iter().copied()) {
                        legend_nodes.push(Node::Path(crate::scene::Path {
                            points: points.iter().map(|point_at| point(*point_at)).collect(),
                            closed: false,
                            fill: None,
                            stroke: Some(stroke.clone()),
                        }));
                    }
                    let edges = legend.bin_edges.as_deref().unwrap_or_default();
                    for (label, (anchor, align)) in edges.iter().zip(&layout.labels) {
                        if label.text.is_empty() && label.superscript.is_none() {
                            continue;
                        }
                        legend_nodes.push(label_node(label, style, point(*anchor), *align, 0.0));
                    }
                }
            }
        }
        nodes.push(Node::Group(Group {
            key: None,
            clip: None,
            role: Some("legend"),
            anchor: None,
            nodes: legend_nodes,
        }));
    }
    Ok(nodes)
}

/// Width in pixels of a `guide_bins()` axis line and its ticks.
const BINS_AXIS_WIDTH: f64 = 1.0;

/// Draw one key (its background, then each layer's key) into `cell`.
fn draw_key(key: &LegendKey, cell: Rect, theme: &Theme, nodes: &mut Vec<Node>) -> Result<()> {
    if let Some(style) = theme.legend_key_background {
        nodes.push(super::rect_node(cell, style));
    }
    for layer in &key.layers {
        nodes.extend(layer.geom.draw_key(&layer.styles, cell)?);
    }
    Ok(())
}

/// Move the nodes from `start` on into the key's clickable group (interactive
/// legend, HTML only).
fn group_key(legend: &Legend, key: &LegendKey, start: usize, theme: &Theme, nodes: &mut Vec<Node>) {
    if !theme.legend_interactive {
        return;
    }
    let grouped = nodes.split_off(start);
    nodes.push(Node::Group(Group {
        clip: None,
        role: Some(LEGEND_KEY_ROLE),
        anchor: None,
        key: Some(key_tag(&[legend_key_id(
            &legend.title,
            &key.label.to_string(),
            key.occurrence,
        )])),
        nodes: grouped,
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Margins;

    #[test]
    fn legend_title_margins_pad_the_title_box() {
        let plain = Theme::default();
        let padded = Theme {
            legend_title_margin: Margins {
                top: 3.0,
                right: 10.0,
                bottom: 3.0,
                left: 4.0,
            },
            ..Theme::default()
        };
        let graph = (80.0, 30.0);
        let (origin, anchor, size) = arrange_title("RNA", graph, true, &plain);
        let (padded_origin, padded_anchor, padded_size) =
            arrange_title("RNA", graph, true, &padded);
        // Horizontal: the graph moves right by the side margins, the title by the left one.
        assert_eq!(padded_origin.0 - origin.0, 14.0);
        assert_eq!(padded_anchor.0 - anchor.0, 4.0);
        assert_eq!(padded_size.0 - size.0, 14.0);
        // No title: margins add nothing.
        assert_eq!(
            arrange_title("", graph, true, &padded).2,
            arrange_title("", graph, true, &plain).2
        );
    }

    #[test]
    fn title_positions_place_the_title_on_each_side() {
        let at = |position: LegendTitlePosition, hjust: f64, vjust: f64| {
            let mut theme = Theme {
                legend_title_position: Some(position),
                legend_margin: Margins {
                    top: 0.0,
                    right: 0.0,
                    bottom: 0.0,
                    left: 0.0,
                },
                ..Theme::default()
            };
            if let Some(style) = theme.legend_title.as_mut() {
                style.hjust = hjust;
                style.vjust = vjust;
            }
            arrange_title("RNA", (200.0, 10.0), true, &theme)
        };
        let theme = Theme::default();
        let (title_width, title_height) = title_size("RNA", &theme);
        let half_line = theme.legend_title.map_or(0.0, text_band) / 2.0;
        // Bottom, centred: the title below the bar, both centred on the wider bar.
        let (origin, anchor, size) = at(LegendTitlePosition::Bottom, 0.5, f64::NAN);
        assert_eq!(origin, (0.0, 0.0));
        assert_eq!(anchor, ((200.0 - title_width) / 2.0, 10.0 + half_line));
        assert_eq!(size, (200.0, 10.0 + half_line + title_height));
        // Top, unset hjust: left.
        let (origin, anchor, _) = at(LegendTitlePosition::Top, f64::NAN, f64::NAN);
        assert_eq!(anchor, (0.0, 0.0));
        assert_eq!(origin, (0.0, title_height + half_line));
        // Right, top-justified: the bar at the left, the title after half a line.
        let (origin, anchor, size) = at(LegendTitlePosition::Right, 0.0, 1.0);
        assert_eq!(origin.0, 0.0);
        assert_eq!(anchor, (200.0 + half_line, 0.0));
        assert_eq!(size.0, 200.0 + half_line + title_width);
        // Left: unset vjust centres both (lets-plot's horizontal legends).
        let (origin, anchor, _) = at(LegendTitlePosition::Left, f64::NAN, f64::NAN);
        assert_eq!(origin.0, title_width);
        assert_eq!(anchor.0, 0.0);
        assert!(title_height > 10.0 && origin.1 == (title_height - 10.0) / 2.0);
        // Unset position: top when vertical, left when horizontal.
        assert_eq!(title_position(&theme, false), LegendTitlePosition::Top);
        assert_eq!(title_position(&theme, true), LegendTitlePosition::Left);
    }

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
