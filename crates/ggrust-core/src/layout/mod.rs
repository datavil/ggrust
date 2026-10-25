//! Plot layout: where the panel, axes and titles go, and drawing them.
//!
//! Space is split in bands using lets-plot's constants (margins, spacing, tick
//! length), but text bands use the real font's line box (ascent + descent, about
//! 1.12 x the font size) instead of lets-plot's "font size" estimate, so glyphs never
//! spill into neighbouring bands. Order: fixed vertical bands -> panel height ->
//! y breaks -> y label width -> panel width -> x breaks.
//!
//! Single panel only; facets and grids come in Phase 3.

mod axis;

pub use axis::{AxisBreaks, Orientation, choose_breaks};

use crate::scale::ContinuousPositionScale;
use crate::scale::format::Label;
use crate::scene::{Group, HorizontalAlign, Node, Path, PerItem, Rect, Rects, Text, VerticalAlign};
use crate::text::{measure, measure_label};

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
}

/// Lay out a `width` x `height` plot.
pub fn layout_plot(
    width: f64,
    height: f64,
    theme: &Theme,
    titles: &Titles,
    x_scale: &ContinuousPositionScale,
    y_scale: &ContinuousPositionScale,
) -> PlotLayout {
    let outer = theme.plot_margin.plus(theme.plot_inset);
    let content = shrink(
        Rect {
            x: 0.0,
            y: 0.0,
            width,
            height,
        },
        outer,
    );

    let top = content.y
        + band(theme.plot_title, &titles.title, 0.0)
        + band(
            theme.plot_subtitle,
            &titles.subtitle,
            theme.plot_subtitle_margin_top,
        );
    let bottom =
        content.y + content.height - band(theme.axis_title_x, &titles.x, 0.0) - x_axis_band(theme);
    let panel_height = (bottom - top).max(1.0);

    let y_breaks = choose_breaks(
        y_scale,
        panel_height,
        Orientation::Vertical,
        theme.axis_text_y,
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
    let left = content.x
        + band(theme.axis_title_y, &titles.y, theme.axis_title_spacing)
        + tick_band(theme.axis_ticks_y, theme)
        + theme
            .axis_text_y
            .map_or(0.0, |_| theme.axis_text_spacing + y_label_width);
    let panel_width = (content.x + content.width - left).max(1.0);

    let x_breaks = choose_breaks(
        x_scale,
        panel_width,
        Orientation::Horizontal,
        theme.axis_text_x,
    );
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
    }
}

/// Panel background and grid lines: drawn below the data.
pub fn draw_panel_background(
    layout: &PlotLayout,
    theme: &Theme,
    x_scale: &ContinuousPositionScale,
    y_scale: &ContinuousPositionScale,
) -> Vec<Node> {
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
            let x = panel.x + x_scale.map(value) * panel.width;
            grid.push(segment((x, panel.y), (x, panel.y + panel.height), line));
        }
    }
    if let Some(line) = theme.panel_grid_major_y {
        for &value in &layout.y_breaks.values {
            let y = panel.y + (1.0 - y_scale.map(value)) * panel.height;
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
    x_scale: &ContinuousPositionScale,
    y_scale: &ContinuousPositionScale,
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
    for (value, label) in layout.x_breaks.values.iter().zip(&layout.x_breaks.labels) {
        let x = panel.x + x_scale.map(*value) * panel.width;
        if let Some(tick) = theme.axis_ticks_x {
            axis_x.push(segment(
                (x, bottom),
                (x, bottom + theme.axis_ticks_length),
                tick,
            ));
        }
        if let Some(text) = theme.axis_text_x {
            axis_x.push(label_node(
                label,
                text,
                (x, x_label_top + text_band(text) / 2.0),
                HorizontalAlign::Middle,
                0.0,
            ));
        }
    }
    let mut axis_y = Vec::new();
    if let Some(line) = theme.axis_line_y {
        axis_y.push(segment((panel.x, panel.y), (panel.x, bottom), line));
    }
    let y_label_right = panel.x - tick_band(theme.axis_ticks_y, theme) - theme.axis_text_spacing;
    for (value, label) in layout.y_breaks.values.iter().zip(&layout.y_breaks.labels) {
        let y = panel.y + (1.0 - y_scale.map(*value)) * panel.height;
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
        let y = content.y + content.height - text_band(style) / 2.0;
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

/// Ticks, spacing and labels below the panel.
fn x_axis_band(theme: &Theme) -> f64 {
    tick_band(theme.axis_ticks_x, theme)
        + theme
            .axis_text_x
            .map_or(0.0, |text| theme.axis_text_spacing + text_band(text))
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
