//! Plot layout: where the panel, axes and titles go, and drawing them.
//!
//! Space is split in bands using lets-plot's constants (margins, spacing, tick
//! length), but text bands use the real font's line box (ascent + descent, about
//! 1.12 x the font size) instead of lets-plot's "font size" estimate, so glyphs never
//! spill into neighbouring bands. Order: fixed vertical bands -> panel height ->
//! y breaks -> y label width -> panel width -> x breaks. When the x labels need a
//! thicker band (tilted or two-row labels), the y axis is laid out once more.
//!
//! Single panel only; facets and grids come in Phase 3.

mod axis;
mod colorbar;
mod legend;

pub use axis::{AxisBreaks, LabelArrangement, Orientation, choose_breaks};
pub use colorbar::{Colorbar, DEFAULT_BINS};
pub use legend::{
    KeyLayer, Legend, LegendBlock, LegendBox, LegendKey, draw_legends, measure_legends,
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
    measure("", style.size, style.face).height()
}
use crate::theme::{LineStyle, Margins, TextStyle, Theme};

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
    /// Area inside the plot margin and inset.
    pub content: Rect,
    /// The data panel.
    pub panel: Rect,
    /// x axis breaks.
    pub x_breaks: AxisBreaks,
    /// y axis breaks.
    pub y_breaks: AxisBreaks,
    /// Ticks, spacing and labels below the panel (px): the x title goes below it.
    pub x_axis_band: f64,
}

/// Lay out a plot inside `area` (the whole canvas for a single plot; a grid cell
/// or the space left by a legend later).
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
) -> PlotLayout {
    let simple = theme.axis_text_x.map_or(0.0, text_band);
    let first = layout_with(area, theme, titles, x_scale, y_scale, simple, None);
    let needed = x_labels_height(&first.x_breaks, theme);
    if needed <= simple {
        return first;
    }
    let x_breaks = first.x_breaks;
    layout_with(
        area,
        theme,
        titles,
        x_scale,
        y_scale,
        needed,
        Some(x_breaks),
    )
}

/// One layout pass with an x label band `labels_height` px tall.
fn layout_with(
    area: Rect,
    theme: &Theme,
    titles: &Titles,
    x_scale: &PositionScale,
    y_scale: &PositionScale,
    labels_height: f64,
    x_breaks: Option<AxisBreaks>,
) -> PlotLayout {
    let outer = theme.plot_margin.plus(theme.plot_inset);
    let content = shrink(area, outer);

    let top = content.y
        + band(theme.plot_title, &titles.title, 0.0)
        + band(
            theme.plot_subtitle,
            &titles.subtitle,
            theme.plot_subtitle_margin_top,
        );
    let x_axis_band = tick_band(theme.axis_ticks_x, theme)
        + theme
            .axis_text_x
            .map_or(0.0, |_| theme.axis_text_spacing + labels_height);
    let bottom =
        content.y + content.height - band(theme.axis_title_x, &titles.x, 0.0) - x_axis_band;
    let panel_height = (bottom - top).max(PANEL_MIN_SIZE);

    let y_breaks = choose_breaks(
        y_scale,
        panel_height,
        Orientation::Vertical,
        theme.axis_text_y,
        (0.0, 0.0),
    );
    let y_label_width = theme.axis_text_y.map_or(0.0, |text| {
        y_breaks
            .labels
            .iter()
            .map(|label| {
                measure_label(
                    &label.text,
                    label.superscript.as_deref(),
                    text.size,
                    text.face,
                )
            })
            .fold(0.0, f64::max)
    });
    let y_axis_width = tick_band(theme.axis_ticks_y, theme)
        + theme
            .axis_text_y
            .map_or(0.0, |_| theme.axis_text_spacing + y_label_width);
    let left =
        content.x + band(theme.axis_title_y, &titles.y, theme.axis_title_spacing) + y_axis_width;
    let panel_width = (content.x + content.width - left).max(PANEL_MIN_SIZE);

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
    }
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
                    text.size,
                    text.face,
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
    if let Some(fill) = theme.panel_background {
        nodes.push(Node::Rects(Rects {
            rects: vec![panel],
            fill: PerItem::Uniform(Some(fill)),
            stroke: None,
        }));
    }
    let mut grid = Vec::new();
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
            axis_x.push(x_label_node(
                label,
                text,
                index,
                x,
                x_label_top,
                &layout.x_breaks.arrangement,
            ));
        }
    }
    let mut axis_y = Vec::new();
    if let Some(line) = theme.axis_line_y {
        axis_y.push(segment((panel.x, panel.y), (panel.x, bottom), line));
    }
    let y_label_right = panel.x - tick_band(theme.axis_ticks_y, theme) - theme.axis_text_spacing;
    for (value, label) in layout.y_breaks.values.iter().zip(&layout.y_breaks.labels) {
        let y = mapper.y_pixel(*value);
        if let Some(tick) = theme.axis_ticks_y {
            axis_y.push(segment(
                (panel.x - theme.axis_ticks_length, y),
                (panel.x, y),
                tick,
            ));
        }
        if let Some(text) = theme.axis_text_y {
            axis_y.push(label_node(
                label,
                text,
                (y_label_right, y),
                HorizontalAlign::End,
                0.0,
            ));
        }
    }
    let mut titles_nodes = Vec::new();
    if let (Some(style), Some(title)) = (theme.axis_title_x, present(&titles.x)) {
        // Below the x axis, not at the plot edge: they differ only when the panel is
        // at its minimum size and the axis runs past the plot (lets-plot does the same).
        let y = bottom + layout.x_axis_band + text_band(style) / 2.0;
        titles_nodes.push(text_node(
            title,
            style,
            (panel.x + panel.width / 2.0, y),
            HorizontalAlign::Middle,
            0.0,
        ));
    }
    if let (Some(style), Some(title)) = (theme.axis_title_y, present(&titles.y)) {
        titles_nodes.push(text_node(
            title,
            style,
            (
                content.x + text_band(style) / 2.0,
                panel.y + panel.height / 2.0,
            ),
            HorizontalAlign::Middle,
            90.0,
        ));
    }
    let mut top = content.y;
    if let (Some(style), Some(title)) = (theme.plot_title, present(&titles.title)) {
        titles_nodes.push(text_node(
            title,
            style,
            (panel.x, top + text_band(style) / 2.0),
            HorizontalAlign::Start,
            0.0,
        ));
        top += text_band(style);
    }
    if let (Some(style), Some(subtitle)) = (theme.plot_subtitle, present(&titles.subtitle)) {
        top += theme.plot_subtitle_margin_top;
        titles_nodes.push(text_node(
            subtitle,
            style,
            (panel.x, top + text_band(style) / 2.0),
            HorizontalAlign::Start,
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
        LabelArrangement::Simple => (
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
        x,
        y,
        size: style.size,
        face: style.face,
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
            layout_plot(area, &theme, &titles, &scale, &scale)
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
