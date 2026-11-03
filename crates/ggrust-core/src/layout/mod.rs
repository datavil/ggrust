//! Plot layout: where the panel, axes and titles go, and drawing them.
//!
//! Space is split in bands using lets-plot's constants (margins, spacing, tick
//! length), but text bands use the real font's line box (ascent + descent, about
//! 1.12 x the font size) instead of lets-plot's "font size" estimate, so glyphs never
//! spill into neighbouring bands. Order: fixed vertical bands -> panel height ->
//! y breaks -> y label width -> panel width -> x breaks. When the x labels need a
//! thicker band (tilted or two-row labels), the y axis is laid out once more.
//!
//! Legends take a band at the side `legend_position` names (inside the titles, outside
//! the axes); [`legend_origin`] places them, relative to the panel as lets-plot does.
//!
//! Faceted plots are laid out by [`facet`]; the cells of a grid of plots by [`grid`].

mod axis;
mod colorbar;
pub mod facet;
pub mod grid;
mod legend;
mod rotated;
pub mod zoom;

pub use axis::{AxisBreaks, LabelArrangement, Orientation, choose_breaks};
pub use colorbar::{Colorbar, DEFAULT_BINS};
pub use legend::{
    KeyGrid, KeyLayer, Legend, LegendBlock, LegendBox, LegendBudget, LegendKey, draw_legends,
    measure_legends,
};

use axis::{LABELS_OVERFLOW, TILT_DEGREES};

use crate::coord::Panel;
use crate::scale::PositionScale;
use crate::scale::format::Label;
use crate::scene::{Group, HorizontalAlign, Node, Path, PerItem, Rect, Rects, Text, VerticalAlign};
use crate::text::{measure, measure_label, single_line};

/// lets-plot `GEOM_MIN_SIZE`: the panel never shrinks below 50 x 50 px; axes and
/// labels that do not fit then run past the plot edge instead.
pub const PANEL_MIN_SIZE: f64 = 50.0;

/// Largest share of the plot (inside its margin) the x labels may take in height,
/// or the y labels in width: longer labels are cut with "...". A ggrust deviation
/// (lets-plot squeezes the panel and lets the labels run off the canvas).
pub const AXIS_LABELS_MAX_SHARE: f64 = 0.5;

/// Height of a text band: the real line box of the font at this size.
pub fn text_band(style: TextStyle) -> f64 {
    measure("", style.size, style.font).height()
}
use crate::theme::{
    LegendPosition, LineStyle, Margins, RectStyle, TextStyle, Theme, TitlePosition,
};

/// Plot-level texts. `None` or empty means absent (takes no space).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Titles {
    /// Plot title.
    pub title: Option<String>,
    /// Plot subtitle.
    pub subtitle: Option<String>,
    /// Plot caption (`labs(caption=...)`).
    pub caption: Option<String>,
    /// x axis title.
    pub x: Option<String>,
    /// y axis title.
    pub y: Option<String>,
}

/// Result of laying out a single-panel plot.
#[derive(Debug, Clone, PartialEq)]
pub struct PlotLayout {
    /// Area inside the plot margin: titles, legends and axis titles go here; the
    /// inset (`plot_inset`) is kept around the axes and panel only, as in lets-plot.
    pub content: Rect,
    /// The data panel.
    pub panel: Rect,
    /// x axis breaks.
    pub x_breaks: AxisBreaks,
    /// y axis breaks.
    pub y_breaks: AxisBreaks,
    /// Ticks, spacing and labels below the panel (px): the x title goes below it.
    pub x_axis_band: f64,
    /// Space kept for legends on each side (see [`legend_space`]).
    pub legend_space: Margins,
    /// Space above the panel taken by facet strips (0 without facets): a top
    /// legend goes above it.
    pub top_band: f64,
    /// Space right of the panel taken by `facet_grid` row strips (0 without
    /// them): a right legend goes beyond it.
    pub right_band: f64,
}

/// Lay out a plot inside `area` (the whole canvas for a single plot; a grid cell
/// later), keeping `legend_space` free for legends. With an `aspect` ratio
/// (`coord_fixed`: one y unit is `aspect` x units on screen) the panel shrinks to
/// keep it, and the plot (panel, axes, axis titles) is centred in `area`, as in
/// lets-plot.
///
/// As lets-plot (`TopDownTileLayout.computeAxisInfos`): y axis for the estimated
/// height, then x axis for the resulting width; if the x labels need a thicker band
/// (tilted, two rows), the y axis is laid out again for the shorter panel while the
/// x breaks and their arrangement are kept.
#[allow(clippy::too_many_arguments)]
pub fn layout_plot(
    area: Rect,
    theme: &Theme,
    titles: &Titles,
    x_scale: &PositionScale,
    y_scale: &PositionScale,
    legend_space: Margins,
    aspect: Option<f64>,
    panel_min_size: f64,
) -> PlotLayout {
    let layout = layout_free(
        area,
        theme,
        titles,
        x_scale,
        y_scale,
        legend_space,
        panel_min_size,
    );
    let Some(ratio) = aspect.filter(|ratio| ratio.is_finite() && *ratio > 0.0) else {
        return layout;
    };
    let span = |scale: &PositionScale| {
        let (low, high) = scale.expanded_domain();
        high - low
    };
    let (x_span, y_span) = (span(x_scale), span(y_scale));
    let (width, height) = (layout.panel.width, layout.panel.height);
    // Pixels per unit: y must be `ratio` times x.
    let wanted_height = ratio * width / x_span * y_span;
    let shrunk = if wanted_height < height {
        let cut = height - wanted_height;
        Rect {
            y: area.y + cut / 2.0,
            height: area.height - cut,
            ..area
        }
    } else {
        let cut = width - height * x_span / (ratio * y_span);
        Rect {
            x: area.x + cut / 2.0,
            width: area.width - cut,
            ..area
        }
    };
    let relaid = if shrunk.width > 0.0 && shrunk.height > 0.0 && shrunk != area {
        layout_free(
            shrunk,
            theme,
            titles,
            x_scale,
            y_scale,
            legend_space,
            panel_min_size,
        )
    } else {
        layout
    };
    // The re-layout picks breaks for the new size, which can change the axis
    // bands a little: fit the panel exactly (lets-plot `adjustGeomSize`), centred
    // in its space, even below the minimum panel size.
    fit_ratio(relaid, ratio, x_span, y_span)
}

/// Shrink the panel of `layout` to the exact `ratio`, centred where it was.
fn fit_ratio(mut layout: PlotLayout, ratio: f64, x_span: f64, y_span: f64) -> PlotLayout {
    let panel = &mut layout.panel;
    let wanted_height = ratio * panel.width / x_span * y_span;
    if wanted_height < panel.height {
        panel.y += (panel.height - wanted_height) / 2.0;
        panel.height = wanted_height;
    } else {
        let wanted_width = panel.height * x_span / (ratio * y_span);
        if wanted_width.is_finite() {
            panel.x += (panel.width - wanted_width) / 2.0;
            panel.width = wanted_width;
        }
    }
    layout
}

/// [`layout_plot`] without an aspect ratio: the panel takes the space left.
///
/// End x labels that would run past the edge of `area` (a wide last label,
/// lets-plot#906) get room: the plot is laid out again in an area narrowed by
/// their overhang. A ggrust deviation (lets-plot and ggplot2 cut such labels);
/// plots whose labels fit are unchanged.
fn layout_free(
    area: Rect,
    theme: &Theme,
    titles: &Titles,
    x_scale: &PositionScale,
    y_scale: &PositionScale,
    legend_space: Margins,
    panel_min_size: f64,
) -> PlotLayout {
    let layout = layout_free_once(
        area,
        theme,
        titles,
        x_scale,
        y_scale,
        legend_space,
        panel_min_size,
    );
    // Same x breaks and arrangement: only the room changes (choosing again at a
    // few px less could turn simple labels vertical). A label moves by its share
    // of the panel only, so narrowing repeats until it fits (converges fast).
    let labels_height = x_labels_height(&layout.x_breaks, theme);
    let mut layout = layout;
    let mut narrowed = area;
    for _ in 0..FIT_PASSES {
        let (left, right) = x_labels_overhang(&layout, x_scale, theme, area);
        if left <= 0.0 && right <= 0.0 {
            break;
        }
        narrowed = Rect {
            x: narrowed.x + left.max(0.0),
            width: (narrowed.width - left.max(0.0) - right.max(0.0)).max(1.0),
            ..narrowed
        };
        let x_breaks = layout.x_breaks;
        layout = layout_with(
            narrowed,
            theme,
            titles,
            (x_scale, y_scale),
            legend_space,
            labels_height,
            Some(x_breaks),
            panel_min_size,
        );
    }
    layout
}

/// Narrowing passes [`layout_free`] makes at most for overflowing end labels.
const FIT_PASSES: usize = 4;

/// Room (px) an overflowing end x label keeps from the edge once fitted.
const EDGE_CLEARANCE: f64 = 1.0;

/// How far the first x label reaches left of `area` and the last one right of
/// it (px; 0 or less: inside). Centred labels only (simple and two-row layouts);
/// rotated labels end at their tick.
fn x_labels_overhang(
    layout: &PlotLayout,
    x_scale: &PositionScale,
    theme: &Theme,
    area: Rect,
) -> (f64, f64) {
    let breaks = &layout.x_breaks;
    let centred = matches!(
        breaks.arrangement,
        LabelArrangement::Simple | LabelArrangement::TwoRows(_)
    );
    let Some(text) = theme.axis_text_x.filter(|_| centred) else {
        return (0.0, 0.0);
    };
    let sizes = label_sizes(&breaks.labels, text);
    let mut overhang = (0.0_f64, 0.0_f64);
    for (value, (width, _)) in breaks.values.iter().zip(sizes) {
        let centre = layout.panel.x + x_scale.map(*value) * layout.panel.width;
        if !centre.is_finite() {
            continue;
        }
        overhang.0 = overhang.0.max(area.x - (centre - width / 2.0));
        overhang.1 = overhang.1.max(centre + width / 2.0 - (area.x + area.width));
    }
    // Overflowing labels end 1 px clear of the edge, so glyph edges are not cut.
    let clear = |overhang: f64| {
        if overhang > 0.0 {
            overhang + EDGE_CLEARANCE
        } else {
            overhang
        }
    };
    (clear(overhang.0), clear(overhang.1))
}

/// One [`layout_free`] pass in `area`.
fn layout_free_once(
    area: Rect,
    theme: &Theme,
    titles: &Titles,
    x_scale: &PositionScale,
    y_scale: &PositionScale,
    legend_space: Margins,
    panel_min_size: f64,
) -> PlotLayout {
    let simple = theme.axis_text_x.map_or(0.0, text_band);
    let first = layout_with(
        area,
        theme,
        titles,
        (x_scale, y_scale),
        legend_space,
        simple,
        None,
        panel_min_size,
    );
    let needed = x_labels_height(&first.x_breaks, theme);
    if needed <= simple {
        return first;
    }
    let limit = first.content.height * AXIS_LABELS_MAX_SHARE;
    let x_breaks = fit_x_labels(first.x_breaks, x_scale, first.panel.width, limit, theme);
    let needed = x_labels_height(&x_breaks, theme);
    layout_with(
        area,
        theme,
        titles,
        (x_scale, y_scale),
        legend_space,
        needed,
        Some(x_breaks),
        panel_min_size,
    )
}

/// One layout pass with an x label band `labels_height` px tall.
#[allow(clippy::too_many_arguments)]
fn layout_with(
    area: Rect,
    theme: &Theme,
    titles: &Titles,
    (x_scale, y_scale): (&PositionScale, &PositionScale),
    legend_space: Margins,
    labels_height: f64,
    x_breaks: Option<AxisBreaks>,
    panel_min_size: f64,
) -> PlotLayout {
    let inset = theme.plot_inset;
    let content = plot_content(area, theme, titles);

    let top = content.y + headings_band(theme, titles) + legend_space.top + inset.top;
    let x_axis_band = x_axis_band(labels_height, theme);
    let bottom = content.y + content.height
        - legend_space.bottom
        - axis_title_band(theme.axis_title_x, &titles.x, x_title_margin(theme), 0.0)
        - inset.bottom
        - x_axis_band;
    let panel_height = (bottom - top).max(panel_min_size);

    let y_breaks = choose_breaks(
        y_scale,
        panel_height,
        Orientation::Vertical,
        theme.axis_text_y,
        (0.0, 0.0),
    );
    let y_breaks = fit_y_labels(
        y_breaks,
        y_scale,
        panel_height,
        content.width * AXIS_LABELS_MAX_SHARE,
        theme,
    );
    let y_axis_width = y_axis_width(&y_breaks, theme);
    let left = content.x
        + legend_space.left
        + axis_title_band(
            theme.axis_title_y,
            &titles.y,
            theme.axis_title_spacing + y_title_margin(theme),
            90.0,
        )
        + inset.left
        + y_axis_width;
    let panel_width =
        (content.x + content.width - legend_space.right - inset.right - left).max(panel_min_size);

    // lets-plot lets x labels reach over the y axis on the left (geom area insets).
    let x_breaks = x_breaks.unwrap_or_else(|| {
        choose_breaks(
            x_scale,
            panel_width,
            Orientation::Horizontal,
            theme.axis_text_x,
            (y_axis_width.max(LABELS_OVERFLOW), LABELS_OVERFLOW),
        )
    });
    PlotLayout {
        content,
        panel: Rect {
            x: left,
            y: top,
            width: panel_width,
            height: panel_height,
        },
        x_breaks,
        y_breaks,
        x_axis_band,
        legend_space,
        top_band: 0.0,
        right_band: 0.0,
    }
}

/// Ticks, spacing and labels below a panel, with labels `labels_height` px tall.
fn x_axis_band(labels_height: f64, theme: &Theme) -> f64 {
    x_tick_band(theme)
        + theme
            .axis_text_x
            .map_or(0.0, |_| theme.axis_text_spacing.0 + labels_height)
}

/// Ticks, spacing and labels left of a panel with these y breaks.
fn y_axis_width(y_breaks: &AxisBreaks, theme: &Theme) -> f64 {
    let y_label_width = theme.axis_text_y.map_or(0.0, |text| {
        let sizes = label_sizes(&y_breaks.labels, text);
        if rotated::rotates(text) {
            rotated::left(&sizes, text).1
        } else {
            sizes.iter().map(|(width, _)| *width).fold(0.0, f64::max)
        }
    });
    y_tick_band(theme)
        + theme
            .axis_text_y
            .map_or(0.0, |_| theme.axis_text_spacing.1 + y_label_width)
}

/// Space legends of `size` take on each side (lets-plot `PlotLegendsLayoutUtil`):
/// the block plus `legend_box_spacing` at a fixed position, nothing inside the panel
/// or without legends.
pub fn legend_space(theme: &Theme, size: (f64, f64)) -> Margins {
    let spacing = theme.legend_box_spacing;
    let mut space = Margins::uniform(0.0);
    match theme.legend_position {
        LegendPosition::Right => space.right = size.0 + spacing,
        LegendPosition::Left => space.left = size.0 + spacing,
        LegendPosition::Top => space.top = size.1 + spacing,
        LegendPosition::Bottom => space.bottom = size.1 + spacing,
        LegendPosition::None | LegendPosition::Inside(..) => {}
    }
    space
}

/// Largest share of the plot (inside its margin) an outside legend block may take
/// across: the panel keeps the rest. A ggrust deviation (lets-plot and ggplot2 have
/// no limit and let legends run off the canvas).
pub const LEGEND_MAX_SHARE: f64 = 0.5;

/// Room for the legend block of a plot filling `area`: the whole content along its
/// side; across, [`LEGEND_MAX_SHARE`] of it for automatic grids and all but a
/// minimum panel for grids the user fixed (`guide_legend(ncol, nrow)`).
pub fn legend_budget(theme: &Theme, area: Rect) -> LegendBudget {
    let content = shrink(area, theme.plot_margin);
    let spacing = theme.legend_box_spacing;
    let room = |share: f64, extent: f64| (extent * share - spacing).max(1.0);
    let sized = |across: &dyn Fn(f64) -> f64| match theme.legend_position {
        LegendPosition::Right | LegendPosition::Left => (across(content.width), content.height),
        LegendPosition::Top | LegendPosition::Bottom => (content.width, across(content.height)),
        LegendPosition::None | LegendPosition::Inside(..) => (content.width, content.height),
    };
    LegendBudget {
        preferred: sized(&|extent| room(LEGEND_MAX_SHARE, extent)),
        most: sized(&|extent| (extent - PANEL_MIN_SIZE - spacing).max(1.0)),
    }
}

/// Top-left corner of a legend block of `size` (lets-plot `LegendBoxesLayoutUtil`):
/// at the side of the plot, justified along the panel by `legend_justification`, or
/// inside the panel at `legend_position`. Outside legends sit at the plot margin
/// (beyond the inset). Right and bottom legends are placed from the panel, so with a
/// minimum-size panel they run past the plot edge instead of covering the data.
/// Outside legends longer than the panel are kept inside the plot along their side
/// (a ggrust deviation: lets-plot centres them on the panel, past the canvas).
pub fn legend_origin(
    layout: &PlotLayout,
    theme: &Theme,
    titles: &Titles,
    size: (f64, f64),
) -> (f64, f64) {
    let panel = layout.panel;
    let inset = theme.plot_inset;
    let spacing = theme.legend_box_spacing;
    let (justify_x, justify_y) = theme.legend_justification;
    let content = layout.content;
    // Within the content when it fits there, else from its start.
    let keep_in = |position: f64, extent: f64, start: f64, span: f64| {
        position.min(start + span - extent).max(start)
    };
    let along_y = keep_in(
        panel.y + (panel.height - size.1) * (1.0 - justify_y),
        size.1,
        content.y,
        content.height,
    );
    let along_x = keep_in(
        panel.x + (panel.width - size.0) * justify_x,
        size.0,
        content.x,
        content.width,
    );
    match theme.legend_position {
        LegendPosition::Right | LegendPosition::None => (
            panel.x + panel.width + layout.right_band + inset.right + spacing,
            along_y,
        ),
        LegendPosition::Left => (layout.content.x, along_y),
        LegendPosition::Top => (
            along_x,
            panel.y - layout.top_band - layout.legend_space.top - inset.top,
        ),
        LegendPosition::Bottom => (
            along_x,
            panel.y
                + panel.height
                + layout.x_axis_band
                + axis_title_band(theme.axis_title_x, &titles.x, x_title_margin(theme), 0.0)
                + inset.bottom
                + spacing,
        ),
        // [0, 0] is the bottom-left corner of the panel; justification picks the
        // block's point placed there.
        LegendPosition::Inside(x, y) => (
            panel.x + panel.width * x - size.0 * justify_x,
            panel.y + panel.height * (1.0 - y) - size.1 * (1.0 - justify_y),
        ),
    }
}

/// `breaks` of a discrete axis with plain labels cut to the most characters
/// (start and end kept around "...") for which `extent` stays within `limit`;
/// unchanged when they already fit. Numeric axes and superscripts are kept whole.
fn fit_labels(
    breaks: AxisBreaks,
    discrete: bool,
    limit: f64,
    extent: impl Fn(&AxisBreaks) -> f64,
) -> AxisBreaks {
    // A cut number reads as a different number: only category names are cut.
    if !discrete || extent(&breaks) <= limit {
        return breaks;
    }
    let cut = |characters: usize| AxisBreaks {
        labels: breaks
            .labels
            .iter()
            .map(|label| cut_label(label, characters))
            .collect(),
        ..breaks.clone()
    };
    let longest = breaks
        .labels
        .iter()
        .map(|label| label.text.chars().count())
        .max()
        .unwrap_or(0);
    // The most characters that fit (fewer never take more space).
    let (mut low, mut high) = (1, longest.saturating_sub(1));
    let mut best = cut(1);
    while low <= high {
        let middle = (low + high) / 2;
        let candidate = cut(middle);
        if extent(&candidate) <= limit {
            best = candidate;
            low = middle + 1;
        } else {
            high = middle - 1;
        }
    }
    best
}

/// Separates the lines of a label or title wrapped by the layout. Not `\n`: a
/// user's own `\n` keeps drawing as a space, as before wrapping existed.
pub const LINE_BREAK: char = '\u{2028}';

/// x `breaks` of a panel `panel_width` wide whose labels need at most `limit` px
/// in height: unchanged when they fit; else, on a discrete axis, wrapped at
/// spaces into each tick's slot and laid out flat, or cut (see [`fit_labels`]).
pub(crate) fn fit_x_labels(
    breaks: AxisBreaks,
    scale: &PositionScale,
    panel_width: f64,
    limit: f64,
    theme: &Theme,
) -> AxisBreaks {
    if x_labels_height(&breaks, theme) <= limit {
        return breaks;
    }
    let wrapped = theme
        .axis_text_x
        .filter(|text| scale.is_discrete() && !rotated::rotates(*text))
        .and_then(|text| {
            wrap_labels(
                &breaks,
                text,
                x_label_slot(&breaks, scale, panel_width, theme),
            )
        })
        .map(|labels| AxisBreaks {
            labels,
            arrangement: LabelArrangement::Simple,
            ..breaks.clone()
        })
        .filter(|wrapped| x_labels_height(wrapped, theme) <= limit);
    match wrapped {
        Some(wrapped) => wrapped,
        None => fit_labels(breaks, scale.is_discrete(), limit, |breaks| {
            x_labels_height(breaks, theme)
        }),
    }
}

/// Width a wrapped x label may take: [`WRAP_SLOT_SHARE`] of the room between ticks,
/// and an end label no further past the panel than half the panel spacing (so
/// facet panels' labels never meet).
fn x_label_slot(
    breaks: &AxisBreaks,
    scale: &PositionScale,
    panel_width: f64,
    theme: &Theme,
) -> f64 {
    let (low, high) = scale.expanded_domain();
    let pixels = panel_width / (high - low);
    let mut values = breaks.values.clone();
    values.sort_by(f64::total_cmp);
    let between = values
        .windows(2)
        .map(|pair| (pair[1] - pair[0]) * pixels)
        .fold(panel_width, f64::min);
    let edge = match (values.first(), values.last()) {
        (Some(first), Some(last)) => {
            2.0 * (((first - low).min(high - last)) * pixels + theme.panel_spacing.0 / 2.0)
        }
        _ => panel_width,
    };
    let slot = between.min(edge) * WRAP_SLOT_SHARE;
    if slot.is_finite() { slot.max(0.0) } else { 0.0 }
}

/// y `breaks` of a panel `panel_height` tall whose axis may be at most `limit` px
/// wide: unchanged when it fits; else, on a discrete axis, labels wrapped at
/// spaces when the lines fit each tick's slot, or cut (see [`fit_labels`]).
pub(crate) fn fit_y_labels(
    breaks: AxisBreaks,
    scale: &PositionScale,
    panel_height: f64,
    limit: f64,
    theme: &Theme,
) -> AxisBreaks {
    let width = y_axis_width(&breaks, theme);
    if width <= limit {
        return breaks;
    }
    let wrapped = theme
        .axis_text_y
        .filter(|text| scale.is_discrete() && !rotated::rotates(*text))
        .and_then(|text| {
            let widest = label_sizes(&breaks.labels, text)
                .iter()
                .map(|(width, _)| *width)
                .fold(0.0, f64::max);
            let labels = wrap_labels(&breaks, text, limit - (width - widest))?;
            let slot = panel_height / breaks.values.len().max(1) as f64;
            let tallest = label_sizes(&labels, text)
                .iter()
                .map(|(_, height)| *height)
                .fold(0.0, f64::max);
            (tallest <= slot).then(|| AxisBreaks {
                labels,
                ..breaks.clone()
            })
        });
    match wrapped {
        Some(wrapped) => wrapped,
        None => fit_labels(breaks, scale.is_discrete(), limit, |breaks| {
            y_axis_width(breaks, theme)
        }),
    }
}

/// Share of a tick's slot a wrapped x label may take (the rest keeps labels apart).
const WRAP_SLOT_SHARE: f64 = 0.9;

/// Plain labels wrapped at spaces to lines at most `width` wide; `None` when a
/// word is wider than that (such labels are cut instead).
fn wrap_labels(breaks: &AxisBreaks, text: TextStyle, width: f64) -> Option<Vec<Label>> {
    breaks
        .labels
        .iter()
        .map(|label| {
            if label.superscript.is_some() || label.suffix.is_some() {
                return Some(label.clone());
            }
            let mut lines: Vec<String> = Vec::new();
            for word in single_line(&label.text)
                .split(' ')
                .filter(|word| !word.is_empty())
            {
                if measure(word, text.size, text.font).width > width {
                    return None;
                }
                match lines.last_mut() {
                    Some(line)
                        if measure(&format!("{line} {word}"), text.size, text.font).width
                            <= width =>
                    {
                        line.push(' ');
                        line.push_str(word);
                    }
                    _ => lines.push(word.to_owned()),
                }
            }
            Some(Label::plain(lines.join(&LINE_BREAK.to_string())))
        })
        .collect()
}

/// Lines in a label (see [`LINE_BREAK`]).
fn line_count(label: &Label) -> usize {
    label.text.matches(LINE_BREAK).count() + 1
}

/// `label` cut to `characters` characters, its middle replaced by "..." (the end
/// often tells similar categories apart), if longer.
fn cut_label(label: &Label, characters: usize) -> Label {
    let text: Vec<char> = label.text.chars().collect();
    if label.superscript.is_some() || text.len() <= characters {
        return label.clone();
    }
    let kept = characters.saturating_sub(3);
    let tail = kept / 2;
    let head: String = text[..kept - tail].iter().collect();
    let end: String = text[text.len() - tail..].iter().collect();
    Label::plain(format!("{}...{}", head.trim_end(), end.trim_start()))
}

/// Unrotated size of each label (real font metrics; height = the line box).
fn label_sizes(labels: &[Label], text: TextStyle) -> Vec<(f64, f64)> {
    labels
        .iter()
        .map(|label| {
            let width = label
                .text
                .split(LINE_BREAK)
                .map(|line| {
                    measure_label(
                        line,
                        label.superscript.as_deref(),
                        label.suffix.as_deref(),
                        text.size,
                        text.font,
                    )
                })
                .fold(0.0, f64::max);
            (width, text_band(text) * line_count(label) as f64)
        })
        .collect()
}

/// Height the x labels need in their arrangement (real font metrics).
fn x_labels_height(breaks: &AxisBreaks, theme: &Theme) -> f64 {
    let Some(text) = theme.axis_text_x else {
        return 0.0;
    };
    let line = text_band(text);
    let widest = || {
        breaks
            .labels
            .iter()
            .map(|label| {
                measure_label(
                    &label.text,
                    label.superscript.as_deref(),
                    label.suffix.as_deref(),
                    text.size,
                    text.font,
                )
            })
            .fold(0.0, f64::max)
    };
    match &breaks.arrangement {
        LabelArrangement::Simple => {
            line * breaks.labels.iter().map(line_count).max().unwrap_or(1) as f64
        }
        LabelArrangement::TwoRows(rows) => {
            let row_count = rows.iter().max().map_or(1, |row| row + 1) as f64;
            row_count * line + (row_count - 1.0) * row_gap(text)
        }
        LabelArrangement::Tilted => {
            let (sin, cos) = TILT_DEGREES.to_radians().sin_cos();
            widest() * sin + line * cos
        }
        LabelArrangement::Vertical => widest(),
        LabelArrangement::Rotated => rotated::bottom(&label_sizes(&breaks.labels, text), text).1,
    }
}

/// lets-plot `HorizontalMultilineLabelsLayout`: rows are 0.2 line heights apart.
fn row_gap(text: TextStyle) -> f64 {
    text.size * 0.2
}

/// Panel background and grid lines: drawn below the data.
pub fn draw_panel_background(layout: &PlotLayout, theme: &Theme, mapper: &Panel) -> Vec<Node> {
    let panel = layout.panel;
    let mut nodes = Vec::new();
    if let Some(style) = theme.panel_background {
        nodes.push(rect_node(panel, style));
    }
    let mut grid = Vec::new();
    // Minor lines first (lets-plot `AxisUtil.minorDomainBreaks`), inside the panel.
    if let Some(line) = theme.panel_grid_minor_x {
        for value in minor_breaks(&layout.x_breaks.values) {
            let x = mapper.x_pixel(value);
            if x >= panel.x && x <= panel.x + panel.width {
                grid.push(segment((x, panel.y), (x, panel.y + panel.height), line));
            }
        }
    }
    if let Some(line) = theme.panel_grid_minor_y {
        for value in minor_breaks(&layout.y_breaks.values) {
            let y = mapper.y_pixel(value);
            if y >= panel.y && y <= panel.y + panel.height {
                grid.push(segment((panel.x, y), (panel.x + panel.width, y), line));
            }
        }
    }
    if let Some(line) = theme.panel_grid_major_x {
        for &value in &layout.x_breaks.values {
            let x = mapper.x_pixel(value);
            grid.push(segment((x, panel.y), (x, panel.y + panel.height), line));
        }
    }
    if let Some(line) = theme.panel_grid_major_y {
        for &value in &layout.y_breaks.values {
            let y = mapper.y_pixel(value);
            grid.push(segment((panel.x, y), (panel.x + panel.width, y), line));
        }
    }
    if !grid.is_empty() {
        nodes.push(Node::Group(Group {
            clip: None,
            role: Some("grid"),
            anchor: None,
            nodes: grid,
        }));
    }
    nodes
}

/// Centre of a justified text in `[start, start + span]` (lets-plot
/// `TextJustification`: the text box moves by `(span - width) * hjust`).
fn justified_centre(start: f64, span: f64, width: f64, style: TextStyle) -> f64 {
    let hjust = if style.hjust.is_finite() {
        style.hjust
    } else {
        0.0
    };
    start + width / 2.0 + (span - width) * hjust
}

/// Centre of a text box of `width` justified at `justification` in
/// `[start, start + span]` (as [`justified_centre`]); a box longer than the span
/// is centred on it (justifying it would push it off the plot).
fn justified_at(start: f64, span: f64, width: f64, justification: f64) -> f64 {
    if width >= span {
        return start + span / 2.0;
    }
    start + width / 2.0 + (span - width) * justification
}

/// An axis title's justification on the page (horizontal, vertical): its
/// `hjust`/`vjust` turned by its angle (`default_angle` when unset), as ggplot2's
/// `rotate_just`. So `vjust` moves a horizontal y title along the panel (#1504).
/// Unset `hjust`/`vjust` are 0.5 (centred, as lets-plot; ggplot2 inherits
/// `vjust = 1`, which puts a horizontal y title at the top).
fn axis_title_justification(style: TextStyle, default_angle: f64) -> (f64, f64) {
    let or_centre = |value: f64| if value.is_finite() { value } else { 0.5 };
    let (hjust, vjust) = (or_centre(style.hjust), or_centre(style.vjust));
    let angle = if style.angle.is_finite() {
        style.angle
    } else {
        default_angle
    };
    let (sin, cos) = angle.to_radians().sin_cos();
    (
        cos * hjust - sin * vjust + (1.0 - cos + sin) / 2.0,
        sin * hjust + cos * vjust + (1.0 - cos - sin) / 2.0,
    )
}

/// lets-plot `AxisUtil.minorDomainBreaks`: half a step before the first major break,
/// then one step apart, one more than the majors (none for a single break).
fn minor_breaks(major: &[f64]) -> Vec<f64> {
    let [first, second, ..] = major else {
        return Vec::new();
    };
    let step = second - first;
    let start = first - step / 2.0;
    (0..=major.len())
        .map(|index| start + index as f64 * step)
        .collect()
}

/// A rectangle element: fill and optional border.
pub fn rect_node(rect: Rect, style: RectStyle) -> Node {
    Node::Rects(Rects {
        rects: vec![rect],
        fill: PerItem::Uniform(Some(style.fill)),
        stroke: style.border.map(LineStyle::stroke),
    })
}

/// The panel border, drawn above the data (`panel_border_ontop`).
pub fn draw_panel_border(layout: &PlotLayout, theme: &Theme) -> Vec<Node> {
    theme.panel_border.map_or_else(Vec::new, |line| {
        vec![Node::Rects(Rects {
            rects: vec![layout.panel],
            fill: PerItem::Uniform(None),
            stroke: Some(line.stroke()),
        })]
    })
}

/// Axes (lines, ticks, labels) and titles: drawn above the data.
pub fn draw_axes_and_titles(
    layout: &PlotLayout,
    theme: &Theme,
    titles: &Titles,
    mapper: &Panel,
) -> Vec<Node> {
    let mut nodes = draw_axes(layout, theme, mapper, (true, true));
    nodes.push(draw_titles(layout, theme, titles));
    nodes
}

/// The x and y axes of a panel (lines, ticks, labels), each only when `shown`
/// (`(x, y)`): facet panels inside the grid draw none.
pub fn draw_axes(
    layout: &PlotLayout,
    theme: &Theme,
    mapper: &Panel,
    (show_x, show_y): (bool, bool),
) -> Vec<Node> {
    let panel = layout.panel;
    let bottom = panel.y + panel.height;
    let mut axis_x = Vec::new();
    if let Some(line) = theme.axis_line_x {
        axis_x.push(segment(
            (panel.x, bottom),
            (panel.x + panel.width, bottom),
            line,
        ));
    }
    let x_label_top = bottom + x_tick_band(theme) + theme.axis_text_spacing.0;
    let x_rotated = match (theme.axis_text_x, &layout.x_breaks.arrangement) {
        (Some(text), LabelArrangement::Rotated) => {
            rotated::bottom(&label_sizes(&layout.x_breaks.labels, text), text).0
        }
        _ => Vec::new(),
    };
    for (index, (value, label)) in layout
        .x_breaks
        .values
        .iter()
        .zip(&layout.x_breaks.labels)
        .enumerate()
    {
        let x = mapper.x_pixel(*value);
        if let Some(tick) = theme.axis_ticks_x {
            axis_x.push(segment(
                (x, bottom),
                (x, bottom + theme.axis_ticks_length.0),
                tick,
            ));
        }
        if let Some(text) = theme.axis_text_x {
            axis_x.push(match x_rotated.get(index) {
                Some(placement) => placed_label_node(label, text, (x, x_label_top), placement),
                None => x_label_node(
                    label,
                    text,
                    index,
                    x,
                    x_label_top,
                    &layout.x_breaks.arrangement,
                ),
            });
        }
    }
    // Minor ticks at the minor grid positions inside the panel (lets-plot draws
    // them after the major ticks and labels).
    if let Some(tick) = theme.axis_minor_ticks_x {
        for value in minor_breaks(&layout.x_breaks.values) {
            let x = mapper.x_pixel(value);
            if x >= panel.x && x <= panel.x + panel.width {
                axis_x.push(segment(
                    (x, bottom),
                    (x, bottom + theme.axis_minor_ticks_length.0),
                    tick,
                ));
            }
        }
    }
    let mut axis_y = Vec::new();
    if let Some(line) = theme.axis_line_y {
        axis_y.push(segment((panel.x, panel.y), (panel.x, bottom), line));
    }
    let y_label_right = panel.x - y_tick_band(theme) - theme.axis_text_spacing.1;
    let y_rotated = match theme.axis_text_y {
        Some(text) if rotated::rotates(text) => {
            rotated::left(&label_sizes(&layout.y_breaks.labels, text), text).0
        }
        _ => Vec::new(),
    };
    for (index, (value, label)) in layout
        .y_breaks
        .values
        .iter()
        .zip(&layout.y_breaks.labels)
        .enumerate()
    {
        let y = mapper.y_pixel(*value);
        if let Some(tick) = theme.axis_ticks_y {
            axis_y.push(segment(
                (panel.x - theme.axis_ticks_length.1, y),
                (panel.x, y),
                tick,
            ));
        }
        if let Some(text) = theme.axis_text_y {
            axis_y.push(match y_rotated.get(index) {
                Some(placement) => placed_label_node(label, text, (y_label_right, y), placement),
                None => label_node(label, text, (y_label_right, y), HorizontalAlign::End, 0.0),
            });
        }
    }
    if let Some(tick) = theme.axis_minor_ticks_y {
        for value in minor_breaks(&layout.y_breaks.values) {
            let y = mapper.y_pixel(value);
            if y >= panel.y && y <= panel.y + panel.height {
                axis_y.push(segment(
                    (panel.x - theme.axis_minor_ticks_length.1, y),
                    (panel.x, y),
                    tick,
                ));
            }
        }
    }
    let mut nodes = Vec::new();
    if show_x {
        nodes.push(role_group("axis-x", axis_x));
    }
    if show_y {
        nodes.push(role_group("axis-y", axis_y));
    }
    nodes
}

/// Plot title, subtitle and axis titles, placed around `layout.panel` (the union of
/// all panels in a faceted plot).
pub fn draw_titles(layout: &PlotLayout, theme: &Theme, titles: &Titles) -> Node {
    let panel = layout.panel;
    let content = layout.content;
    let bottom = panel.y + panel.height;
    let mut titles_nodes = Vec::new();
    if let (Some(style), Some(title)) = (theme.axis_title_x, present(&titles.x)) {
        // Below the x axis, not at the plot edge: they differ only when the panel is
        // at its minimum size and the axis runs past the plot (lets-plot does the same).
        let (along, across) = axis_title_extent(style, title, 0.0);
        let y = bottom
            + layout.x_axis_band
            + theme.plot_inset.bottom
            + theme.axis_title_x_margin.top
            + across / 2.0;
        let x = justified_at(
            panel.x,
            panel.width,
            along,
            axis_title_justification(style, 0.0).0,
        );
        titles_nodes.push(text_node(
            title,
            style,
            (x, y),
            HorizontalAlign::Middle,
            rotated::angle(style),
        ));
    }
    if let (Some(style), Some(title)) = (theme.axis_title_y, present(&titles.y)) {
        // Rotated: the justification runs from the bottom of the panel up.
        let (along, across) = axis_title_extent(style, title, 90.0);
        let centre = justified_at(
            0.0,
            panel.height,
            along,
            axis_title_justification(style, 90.0).1,
        );
        let angle = if style.angle.is_finite() {
            style.angle
        } else {
            90.0
        };
        titles_nodes.push(text_node(
            title,
            style,
            (
                content.x
                    + layout.legend_space.left
                    + theme.axis_title_y_margin.left
                    + across / 2.0,
                panel.y + panel.height - centre,
            ),
            HorizontalAlign::Middle,
            angle,
        ));
    }
    let span = |position: TitlePosition| match position {
        TitlePosition::Panel => (panel.x, panel.width),
        TitlePosition::Plot => (content.x, content.width),
    };
    titles_nodes.extend(heading_nodes(
        content.y,
        span(theme.plot_title_position),
        theme,
        titles,
    ));
    titles_nodes.extend(caption_nodes(
        content.y + content.height,
        span(theme.plot_caption_position),
        theme,
        titles,
    ));
    role_group("titles", titles_nodes)
}

/// Height of a plot title, subtitle or caption band: its lines and its top and
/// bottom margins (lets-plot `PlotLayoutUtil.titleThickness`). Titles may hold
/// several lines (see [`wrap_headings`]).
fn heading_band(style: Option<TextStyle>, text: &Option<String>, margin: Margins) -> f64 {
    match (style, present(text)) {
        (Some(style), Some(text)) => {
            let lines = text.matches(LINE_BREAK).count() as f64 + 1.0;
            text_band(style) * lines + margin.top + margin.bottom
        }
        _ => 0.0,
    }
}

/// Height of the title and subtitle bands at the top of a figure.
pub fn headings_band(theme: &Theme, titles: &Titles) -> f64 {
    heading_band(theme.plot_title, &titles.title, theme.plot_title_margin)
        + heading_band(
            theme.plot_subtitle,
            &titles.subtitle,
            theme.plot_subtitle_margin,
        )
}

/// Height of the caption band at the bottom of a plot.
pub fn caption_band(theme: &Theme, titles: &Titles) -> f64 {
    heading_band(
        theme.plot_caption,
        &titles.caption,
        theme.plot_caption_margin,
    )
}

/// The area inside the plot margin that titles, legends, axes and panels share:
/// the caption takes its band at the bottom (lets-plot lays it out below the
/// legends).
pub fn plot_content(area: Rect, theme: &Theme, titles: &Titles) -> Rect {
    let content = shrink(area, theme.plot_margin);
    let caption = caption_band(theme, titles);
    Rect {
        height: (content.height - caption).max(1.0),
        ..content
    }
}

/// `titles` with the title and subtitle wrapped at spaces to lines no wider than
/// `width` (a word longer than a line is broken). A ggrust deviation: lets-plot
/// and ggplot2 cut long titles at the canvas edge.
pub fn wrap_headings(titles: &Titles, theme: &Theme, width: f64) -> Titles {
    let wrap = |style: Option<TextStyle>, text: &Option<String>| match (style, text) {
        (Some(style), Some(text)) => Some(wrap_line(&single_line(text), style, width)),
        _ => text.clone(),
    };
    Titles {
        title: wrap(theme.plot_title, &titles.title),
        subtitle: wrap(theme.plot_subtitle, &titles.subtitle),
        caption: wrap(theme.plot_caption, &titles.caption),
        x: titles.x.clone(),
        y: titles.y.clone(),
    }
}

/// One line of text broken at spaces into lines no wider than `width`.
fn wrap_line(line: &str, style: TextStyle, width: f64) -> String {
    let fits = |text: &str| measure(text, style.size, style.font).width <= width;
    if fits(line) {
        return line.to_owned();
    }
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in line.split(' ').filter(|word| !word.is_empty()) {
        let candidate = if current.is_empty() {
            word.to_owned()
        } else {
            format!("{current} {word}")
        };
        if fits(&candidate) {
            current = candidate;
            continue;
        }
        if !current.is_empty() {
            lines.push(std::mem::take(&mut current));
        }
        // A word wider than a line is broken between characters.
        for character in word.chars() {
            current.push(character);
            if !fits(&current) && current.chars().count() > 1 {
                current.pop();
                lines.push(std::mem::replace(&mut current, character.to_string()));
            }
        }
    }
    lines.push(current);
    lines.join(&LINE_BREAK.to_string())
}

/// The lines of a title, subtitle or caption band from `top` down, inside its
/// margins, each justified along `(start, width)`. Returns the band's bottom.
fn heading_lines(
    nodes: &mut Vec<Node>,
    (style, text, margin): (TextStyle, &str, Margins),
    top: f64,
    (start, width): (f64, f64),
) -> f64 {
    let (start, width) = (start + margin.left, width - margin.left - margin.right);
    let mut top = top + margin.top;
    for line in text.split(LINE_BREAK) {
        let x = justified_centre(
            start,
            width,
            measure(line, style.size, style.font).width,
            style,
        );
        nodes.push(text_node(
            line,
            style,
            (x, top + text_band(style) / 2.0),
            HorizontalAlign::Middle,
            0.0,
        ));
        top += text_band(style);
    }
    top + margin.bottom
}

/// The title and subtitle, from `top` down, justified along `span` (the panels of
/// a plot, the cells of a grid, or the whole plot).
pub fn heading_nodes(top: f64, span: (f64, f64), theme: &Theme, titles: &Titles) -> Vec<Node> {
    let mut nodes = Vec::new();
    let mut top = top;
    if let (Some(style), Some(title)) = (theme.plot_title, present(&titles.title)) {
        top = heading_lines(
            &mut nodes,
            (style, title, theme.plot_title_margin),
            top,
            span,
        );
    }
    if let (Some(style), Some(subtitle)) = (theme.plot_subtitle, present(&titles.subtitle)) {
        let subtitle = (style, subtitle, theme.plot_subtitle_margin);
        heading_lines(&mut nodes, subtitle, top, span);
    }
    nodes
}

/// The caption, from `top` down, justified along `span`.
pub fn caption_nodes(top: f64, span: (f64, f64), theme: &Theme, titles: &Titles) -> Vec<Node> {
    let mut nodes = Vec::new();
    if let (Some(style), Some(caption)) = (theme.plot_caption, present(&titles.caption)) {
        let caption = (style, caption, theme.plot_caption_margin);
        heading_lines(&mut nodes, caption, top, span);
    }
    nodes
}

fn role_group(role: &'static str, nodes: Vec<Node>) -> Node {
    Node::Group(Group {
        clip: None,
        role: Some(role),
        anchor: None,
        nodes,
    })
}

/// An x tick label placed by its arrangement; `top` is the top of the label band.
fn x_label_node(
    label: &Label,
    text: TextStyle,
    index: usize,
    x: f64,
    top: f64,
    arrangement: &LabelArrangement,
) -> Node {
    let line = text_band(text);
    let (anchor, horizontal, vertical, angle) = match arrangement {
        // Rotated labels are placed by `placed_label_node`.
        LabelArrangement::Simple | LabelArrangement::Rotated => (
            (x, top + line * line_count(label) as f64 / 2.0),
            HorizontalAlign::Middle,
            VerticalAlign::Middle,
            0.0,
        ),
        LabelArrangement::TwoRows(rows) => {
            let row = rows.get(index).copied().unwrap_or(0) as f64;
            let y = top + row * (line + row_gap(text)) + line / 2.0;
            ((x, y), HorizontalAlign::Middle, VerticalAlign::Middle, 0.0)
        }
        // Rotated about the tick: the end of the text sits at the tick.
        LabelArrangement::Tilted => (
            (x, top),
            HorizontalAlign::End,
            VerticalAlign::Top,
            TILT_DEGREES,
        ),
        LabelArrangement::Vertical => ((x, top), HorizontalAlign::End, VerticalAlign::Middle, 90.0),
    };
    let mut node = label_node(label, text, anchor, horizontal, angle);
    if let Node::Text(text) = &mut node {
        text.vertical = vertical;
    }
    node
}

/// A tick label placed by the rotated layout from its reference point.
fn placed_label_node(
    label: &Label,
    text: TextStyle,
    (x, y): (f64, f64),
    placement: &rotated::Placement,
) -> Node {
    let anchor = (x + placement.offset.0, y + placement.offset.1);
    let mut node = label_node(label, text, anchor, placement.horizontal, placement.angle);
    if let Node::Text(text) = &mut node {
        text.vertical = placement.vertical;
    }
    node
}

fn present(text: &Option<String>) -> Option<&str> {
    text.as_deref().filter(|text| !text.is_empty())
}

/// Space the x axis title's margins add across it (top and bottom, as lets-plot).
pub(crate) fn x_title_margin(theme: &Theme) -> f64 {
    theme.axis_title_x_margin.top + theme.axis_title_x_margin.bottom
}

/// Space the y axis title's margins add across it (left and right).
pub(crate) fn y_title_margin(theme: &Theme) -> f64 {
    theme.axis_title_y_margin.left + theme.axis_title_y_margin.right
}

/// Thickness of an axis title band: `gap` plus the title box across its axis, or 0
/// when blank or absent.
fn axis_title_band(
    style: Option<TextStyle>,
    text: &Option<String>,
    gap: f64,
    default_angle: f64,
) -> f64 {
    match (style, present(text)) {
        (Some(style), Some(text)) => gap + axis_title_extent(style, text, default_angle).1,
        _ => 0.0,
    }
}

/// Extent of an axis title (along its axis, across it). A theme `angle` turns the
/// title's box (as ggplot2 does; lets-plot ignores it); unset, the title lies
/// along its axis (`default_angle`: 0 for x, 90 for y).
fn axis_title_extent(style: TextStyle, text: &str, default_angle: f64) -> (f64, f64) {
    let size = (
        measure(text, style.size, style.font).width,
        text_band(style),
    );
    if !style.angle.is_finite() {
        return size;
    }
    let (width, height) = rotated::rotated_size(size, style.angle);
    if default_angle == 0.0 {
        (width, height)
    } else {
        (height, width)
    }
}

/// Room the ticks of one axis take: their `length` when drawn.
fn tick_band(ticks: Option<LineStyle>, length: f64) -> f64 {
    ticks.map_or(0.0, |_| length)
}

/// Space the x ticks take below the panel: the longer of the major and minor
/// ticks shown (ggplot2; lets-plot reserves the major ticks only, so longer minor
/// ticks run into the labels).
pub(crate) fn x_tick_band(theme: &Theme) -> f64 {
    tick_band(theme.axis_ticks_x, theme.axis_ticks_length.0).max(tick_band(
        theme.axis_minor_ticks_x,
        theme.axis_minor_ticks_length.0,
    ))
}

/// Space the y ticks take left of the panel (as [`x_tick_band`]).
pub(crate) fn y_tick_band(theme: &Theme) -> f64 {
    tick_band(theme.axis_ticks_y, theme.axis_ticks_length.1).max(tick_band(
        theme.axis_minor_ticks_y,
        theme.axis_minor_ticks_length.1,
    ))
}

/// `rect` without `margins` (at least 1 px left each way).
pub fn shrink(rect: Rect, margins: Margins) -> Rect {
    Rect {
        x: rect.x + margins.left,
        y: rect.y + margins.top,
        width: (rect.width - margins.left - margins.right).max(1.0),
        height: (rect.height - margins.top - margins.bottom).max(1.0),
    }
}

fn segment(start: (f64, f64), end: (f64, f64), line: LineStyle) -> Node {
    Node::Path(Path {
        points: vec![start, end],
        closed: false,
        fill: None,
        stroke: Some(line.stroke()),
    })
}

/// A tick label: [`text_node`] plus its superscript.
fn label_node(
    label: &Label,
    style: TextStyle,
    anchor: (f64, f64),
    horizontal: HorizontalAlign,
    angle: f64,
) -> Node {
    let lines: Vec<&str> = label.text.split(LINE_BREAK).collect();
    if lines.len() > 1 {
        // A wrapped label: lines centred as a block on the anchor, stacked along
        // the text's own downward direction.
        let band = text_band(style);
        let (sin, cos) = angle.to_radians().sin_cos();
        let middle = (lines.len() - 1) as f64 / 2.0;
        let nodes = lines
            .iter()
            .enumerate()
            .map(|(index, line)| {
                let offset = (index as f64 - middle) * band;
                let anchor = (anchor.0 + offset * sin, anchor.1 + offset * cos);
                text_node(line, style, anchor, horizontal, angle)
            })
            .collect();
        return Node::Group(Group {
            clip: None,
            role: None,
            anchor: None,
            nodes,
        });
    }
    let mut node = text_node(&label.text, style, anchor, horizontal, angle);
    if let Node::Text(text) = &mut node {
        text.superscript = label.superscript.clone();
        text.suffix = label.suffix.clone();
    }
    node
}

fn text_node(
    content: &str,
    style: TextStyle,
    (x, y): (f64, f64),
    horizontal: HorizontalAlign,
    angle: f64,
) -> Node {
    Node::Text(Text {
        content: content.to_owned(),
        superscript: None,
        suffix: None,
        x,
        y,
        size: style.size,
        font: style.font,
        color: style.color,
        horizontal,
        vertical: VerticalAlign::Middle,
        angle,
        halo: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::ContinuousPositionScale;

    #[test]
    fn a_plot_laid_out_in_an_offset_area_is_translated() {
        let mut scale = ContinuousPositionScale::default();
        scale.train_range(0.0, 10.0);
        let scale = PositionScale::Continuous(scale);
        let theme = Theme::default();
        let titles = Titles {
            title: Some("Title".into()),
            x: Some("x".into()),
            y: Some("y".into()),
            ..Titles::default()
        };
        let at = |x, y| {
            let area = Rect {
                x,
                y,
                width: 400.0,
                height: 300.0,
            };
            layout_plot(
                area,
                &theme,
                &titles,
                &scale,
                &scale,
                Margins::uniform(0.0),
                None,
                PANEL_MIN_SIZE,
            )
        };
        let (origin, moved) = (at(0.0, 0.0), at(250.0, 40.0));
        assert_eq!(moved.panel.x, origin.panel.x + 250.0);
        assert_eq!(moved.panel.y, origin.panel.y + 40.0);
        assert_eq!(
            (moved.panel.width, moved.panel.height),
            (origin.panel.width, origin.panel.height)
        );
        assert_eq!(moved.x_breaks, origin.x_breaks);
        assert_eq!(moved.y_breaks, origin.y_breaks);
    }
}
