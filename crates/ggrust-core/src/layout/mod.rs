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
//! Single panel only; facets and grids come in Phase 3.

mod axis;
mod colorbar;
mod legend;
mod rotated;

pub use axis::{AxisBreaks, LabelArrangement, Orientation, choose_breaks};
pub use colorbar::{Colorbar, DEFAULT_BINS};
pub use legend::{
    KeyGrid, KeyLayer, Legend, LegendBlock, LegendBox, LegendKey, draw_legends, measure_legends,
};

use axis::{LABELS_OVERFLOW, TILT_DEGREES};

use crate::coord::Panel;
use crate::scale::PositionScale;
use crate::scale::format::Label;
use crate::scene::{Group, HorizontalAlign, Node, Path, PerItem, Rect, Rects, Text, VerticalAlign};
use crate::text::{measure, measure_label};

/// lets-plot `GEOM_MIN_SIZE`: the panel never shrinks below 50 x 50 px; axes and
/// labels that do not fit then run past the plot edge instead.
pub const PANEL_MIN_SIZE: f64 = 50.0;

/// Height of a text band: the real line box of the font at this size.
pub fn text_band(style: TextStyle) -> f64 {
    measure("", style.size, style.font).height()
}
use crate::theme::{LegendPosition, LineStyle, Margins, RectStyle, TextStyle, Theme};

/// Plot-level texts. `None` or empty means absent (takes no space).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Titles {
    /// Plot title.
    pub title: Option<String>,
    /// Plot subtitle.
    pub subtitle: Option<String>,
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
pub fn layout_plot(
    area: Rect,
    theme: &Theme,
    titles: &Titles,
    x_scale: &PositionScale,
    y_scale: &PositionScale,
    legend_space: Margins,
    aspect: Option<f64>,
) -> PlotLayout {
    let layout = layout_free(area, theme, titles, x_scale, y_scale, legend_space);
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
        layout_free(shrunk, theme, titles, x_scale, y_scale, legend_space)
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
fn layout_free(
    area: Rect,
    theme: &Theme,
    titles: &Titles,
    x_scale: &PositionScale,
    y_scale: &PositionScale,
    legend_space: Margins,
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
    );
    let needed = x_labels_height(&first.x_breaks, theme);
    if needed <= simple {
        return first;
    }
    let x_breaks = first.x_breaks;
    layout_with(
        area,
        theme,
        titles,
        (x_scale, y_scale),
        legend_space,
        needed,
        Some(x_breaks),
    )
}

/// One layout pass with an x label band `labels_height` px tall.
fn layout_with(
    area: Rect,
    theme: &Theme,
    titles: &Titles,
    (x_scale, y_scale): (&PositionScale, &PositionScale),
    legend_space: Margins,
    labels_height: f64,
    x_breaks: Option<AxisBreaks>,
) -> PlotLayout {
    let inset = theme.plot_inset;
    let content = shrink(area, theme.plot_margin);

    let top = content.y
        + band(theme.plot_title, &titles.title, 0.0)
        + band(
            theme.plot_subtitle,
            &titles.subtitle,
            theme.plot_subtitle_margin_top,
        )
        + legend_space.top
        + inset.top;
    let x_axis_band = tick_band(theme.axis_ticks_x, theme)
        + theme
            .axis_text_x
            .map_or(0.0, |_| theme.axis_text_spacing + labels_height);
    let bottom = content.y + content.height
        - legend_space.bottom
        - band(theme.axis_title_x, &titles.x, 0.0)
        - inset.bottom
        - x_axis_band;
    let panel_height = (bottom - top).max(PANEL_MIN_SIZE);

    let y_breaks = choose_breaks(
        y_scale,
        panel_height,
        Orientation::Vertical,
        theme.axis_text_y,
        (0.0, 0.0),
    );
    let y_label_width = theme.axis_text_y.map_or(0.0, |text| {
        let sizes = label_sizes(&y_breaks.labels, text);
        if rotated::rotates(text) {
            rotated::left(&sizes, text).1
        } else {
            sizes.iter().map(|(width, _)| *width).fold(0.0, f64::max)
        }
    });
    let y_axis_width = tick_band(theme.axis_ticks_y, theme)
        + theme
            .axis_text_y
            .map_or(0.0, |_| theme.axis_text_spacing + y_label_width);
    let left = content.x
        + legend_space.left
        + band(theme.axis_title_y, &titles.y, theme.axis_title_spacing)
        + inset.left
        + y_axis_width;
    let panel_width =
        (content.x + content.width - legend_space.right - inset.right - left).max(PANEL_MIN_SIZE);

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
    }
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

/// Top-left corner of a legend block of `size` (lets-plot `LegendBoxesLayoutUtil`):
/// at the side of the plot, justified along the panel by `legend_justification`, or
/// inside the panel at `legend_position`. Outside legends sit at the plot margin
/// (beyond the inset). Right and bottom legends are placed from the panel, so with a
/// minimum-size panel they run past the plot edge instead of covering the data.
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
    let along_y = panel.y + (panel.height - size.1) * (1.0 - justify_y);
    let along_x = panel.x + (panel.width - size.0) * justify_x;
    match theme.legend_position {
        LegendPosition::Right | LegendPosition::None => {
            (panel.x + panel.width + inset.right + spacing, along_y)
        }
        LegendPosition::Left => (layout.content.x, along_y),
        LegendPosition::Top => (along_x, panel.y - layout.legend_space.top - inset.top),
        LegendPosition::Bottom => (
            along_x,
            panel.y
                + panel.height
                + layout.x_axis_band
                + band(theme.axis_title_x, &titles.x, 0.0)
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

/// Unrotated size of each label (real font metrics; height = the line box).
fn label_sizes(labels: &[Label], text: TextStyle) -> Vec<(f64, f64)> {
    labels
        .iter()
        .map(|label| {
            let width = measure_label(
                &label.text,
                label.superscript.as_deref(),
                label.suffix.as_deref(),
                text.size,
                text.font,
            );
            (width, text_band(text))
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
        LabelArrangement::Simple => line,
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
            nodes: grid,
        }));
    }
    nodes
}

/// Centre of a justified text in `[start, start + span]` (lets-plot
/// `TextJustification`: the text box moves by `(span - width) * hjust`).
fn justified_centre(start: f64, span: f64, text: &str, style: TextStyle) -> f64 {
    let width = measure(text, style.size, style.font).width;
    let hjust = if style.hjust.is_finite() {
        style.hjust
    } else {
        0.0
    };
    start + width / 2.0 + (span - width) * hjust
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
    let panel = layout.panel;
    let content = layout.content;
    let bottom = panel.y + panel.height;
    let mut axis_x = Vec::new();
    if let Some(line) = theme.axis_line_x {
        axis_x.push(segment(
            (panel.x, bottom),
            (panel.x + panel.width, bottom),
            line,
        ));
    }
    let x_label_top = bottom + tick_band(theme.axis_ticks_x, theme) + theme.axis_text_spacing;
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
                (x, bottom + theme.axis_ticks_length),
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
    let mut axis_y = Vec::new();
    if let Some(line) = theme.axis_line_y {
        axis_y.push(segment((panel.x, panel.y), (panel.x, bottom), line));
    }
    let y_label_right = panel.x - tick_band(theme.axis_ticks_y, theme) - theme.axis_text_spacing;
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
                (panel.x - theme.axis_ticks_length, y),
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
    let mut titles_nodes = Vec::new();
    if let (Some(style), Some(title)) = (theme.axis_title_x, present(&titles.x)) {
        // Below the x axis, not at the plot edge: they differ only when the panel is
        // at its minimum size and the axis runs past the plot (lets-plot does the same).
        let y = bottom + layout.x_axis_band + theme.plot_inset.bottom + text_band(style) / 2.0;
        let x = justified_centre(panel.x, panel.width, title, style);
        titles_nodes.push(text_node(
            title,
            style,
            (x, y),
            HorizontalAlign::Middle,
            0.0,
        ));
    }
    if let (Some(style), Some(title)) = (theme.axis_title_y, present(&titles.y)) {
        // Rotated: hjust runs from the bottom of the panel up.
        let along = justified_centre(0.0, panel.height, title, style);
        titles_nodes.push(text_node(
            title,
            style,
            (
                content.x + layout.legend_space.left + text_band(style) / 2.0,
                panel.y + panel.height - along,
            ),
            HorizontalAlign::Middle,
            90.0,
        ));
    }
    let mut top = content.y;
    if let (Some(style), Some(title)) = (theme.plot_title, present(&titles.title)) {
        let x = justified_centre(panel.x, panel.width, title, style);
        titles_nodes.push(text_node(
            title,
            style,
            (x, top + text_band(style) / 2.0),
            HorizontalAlign::Middle,
            0.0,
        ));
        top += text_band(style);
    }
    if let (Some(style), Some(subtitle)) = (theme.plot_subtitle, present(&titles.subtitle)) {
        top += theme.plot_subtitle_margin_top;
        let x = justified_centre(panel.x, panel.width, subtitle, style);
        titles_nodes.push(text_node(
            subtitle,
            style,
            (x, top + text_band(style) / 2.0),
            HorizontalAlign::Middle,
            0.0,
        ));
    }
    let group = |role, nodes| {
        Node::Group(Group {
            clip: None,
            role: Some(role),
            nodes,
        })
    };
    vec![
        group("axis-x", axis_x),
        group("axis-y", axis_y),
        group("titles", titles_nodes),
    ]
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
            (x, top + line / 2.0),
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

/// Thickness of a text band (`gap` + line box), or 0 when blank or absent.
fn band(style: Option<TextStyle>, text: &Option<String>, gap: f64) -> f64 {
    match (style, present(text)) {
        (Some(style), Some(_)) => gap + text_band(style),
        _ => 0.0,
    }
}

fn tick_band(ticks: Option<LineStyle>, theme: &Theme) -> f64 {
    ticks.map_or(0.0, |_| theme.axis_ticks_length)
}

fn shrink(rect: Rect, margins: Margins) -> Rect {
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
