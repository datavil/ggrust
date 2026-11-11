//! The panel of a `coord_polar` plot (lets-plot `PolarFrameOfReference`,
//! `PolarAxisUtil`): the background and border as a circle, the grid as spokes and
//! circles, the layers bent by [`Warp`], the theta axis around the ring and the
//! radius axis on the panel's edge.
//!
//! What it does not do: secondary axes (`sec_axis` is ignored) and rotated labels.

use super::axis::{Orientation, choose_breaks};
use super::{AxisBreaks, PlotLayout, draw_axes, label_node, minor_breaks, segment, text_band};
use crate::coord::{Panel, PolarView, Warp};
use crate::scale::AxisSides;
use crate::scale::format::Label;
use crate::scene::{Group, HorizontalAlign, Node, Path, PerItem, Rect, Rects};
use crate::theme::{LineStyle, RectStyle, Theme};

/// Points of a circle's outline; at a radius of 300 px a chord leaves it by 0.01 px.
const CIRCLE_POINTS: usize = 720;

/// Closest the first and last theta ticks may be, in pixels, before their labels merge
/// (lets-plot `PolarAxisUtil`).
const MERGE_DISTANCE: f64 = 3.0;

/// The theme a polar plot is laid out with: the theta axis lives inside the panel, so
/// it takes no band around it.
pub fn layout_theme(theme: &Theme) -> Theme {
    Theme {
        axis_text_x: None,
        axis_ticks_x: None,
        axis_minor_ticks_x: None,
        axis_line_x: None,
        ..theme.clone()
    }
}

/// Breaks of a polar panel: theta along a length of about 1.5 x the width (lets-plot
/// `PolarTileLayout`, whose width is taken before the panel is squared; 1.45 matched
/// its breaks best over a range of sizes and domains), the radius along half the height.
fn polar_breaks(panel: &Panel, theme: &Theme) -> (AxisBreaks, AxisBreaks) {
    let rect = panel.rect;
    (
        choose_breaks(
            panel.x_scale,
            rect.width * 1.45,
            Orientation::Horizontal,
            theme.axis_text_x,
            (0.0, 0.0),
        ),
        choose_breaks(
            panel.y_scale,
            rect.height / 2.0,
            Orientation::Vertical,
            theme.axis_text_y,
            (0.0, 0.0),
        ),
    )
}

/// The circle at share `radius` of the radial domain.
fn circle(warp: &Warp, radius: f64) -> Vec<(f64, f64)> {
    (0..CIRCLE_POINTS)
        .map(|index| warp.at(index as f64 / CIRCLE_POINTS as f64, radius))
        .collect()
}

fn ring(warp: &Warp, radius: f64, line: LineStyle) -> Node {
    Node::Path(Path {
        points: circle(warp, radius),
        closed: true,
        fill: None,
        stroke: Some(line.stroke()),
    })
}

fn background(view: PolarView, warp: &Warp, layout: &PlotLayout, style: RectStyle) -> Node {
    match view.transform_background {
        true => Node::Path(Path {
            points: circle(warp, Warp::RING),
            closed: true,
            fill: Some(style.fill),
            stroke: style.border.map(LineStyle::stroke),
        }),
        false => super::rect_node(layout.panel, style),
    }
}

fn grid(warp: &Warp, panel: &Panel, theme: &Theme, theta: &[f64], radius: &[f64]) -> Vec<Node> {
    let spokes = |values: &[f64], line: LineStyle, nodes: &mut Vec<Node>| {
        for &value in values {
            let share = panel.x_scale.map(value);
            if (0.0..=1.0).contains(&share) {
                nodes.push(segment(
                    warp.at(share, 0.0),
                    warp.at(share, Warp::RING),
                    line,
                ));
            }
        }
    };
    let rings = |values: &[f64], line: LineStyle, nodes: &mut Vec<Node>| {
        for &value in values {
            let share = panel.y_scale.map(value);
            if share > 0.0 && share <= Warp::RING {
                nodes.push(ring(warp, share, line));
            }
        }
    };
    let mut nodes = Vec::new();
    if let Some(line) = theme.panel_grid_minor_x {
        spokes(&minor_breaks(theta), line, &mut nodes);
    }
    if let Some(line) = theme.panel_grid_minor_y {
        rings(&minor_breaks(radius), line, &mut nodes);
    }
    if let Some(line) = theme.panel_grid_major_x {
        spokes(theta, line, &mut nodes);
    }
    if let Some(line) = theme.panel_grid_major_y {
        rings(radius, line, &mut nodes);
    }
    nodes
}

/// How a theta label sits on its anchor, for a tick pointing at `angle` degrees
/// (y down): its alignment and which way half its height moves it (lets-plot
/// `PolarAxisComponent`).
fn label_alignment(angle: f64) -> (HorizontalAlign, f64) {
    match angle {
        -5.0..=5.0 => (HorizontalAlign::Start, 0.0),
        85.0..=95.0 => (HorizontalAlign::Middle, 1.0),
        -95.0..=-85.0 => (HorizontalAlign::Middle, -1.0),
        175.0..=180.0 | -180.0..=-175.0 => (HorizontalAlign::End, 0.0),
        0.0..=90.0 => (HorizontalAlign::Start, 1.0),
        90.0..=180.0 => (HorizontalAlign::End, 1.0),
        -90.0..=0.0 => (HorizontalAlign::Start, -1.0),
        _ => (HorizontalAlign::End, -1.0),
    }
}

/// The theta axis: the ring, ticks pointing out of it and the labels past them.
fn theta_axis(warp: &Warp, panel: &Panel, theme: &Theme, breaks: &AxisBreaks) -> Vec<Node> {
    let mut nodes = Vec::new();
    if let Some(line) = theme.axis_line_x {
        nodes.push(ring(warp, Warp::RING, line.style));
    }
    let mut marks: Vec<(f64, Label)> = breaks
        .values
        .iter()
        .zip(&breaks.labels)
        .map(|(&value, label)| (panel.x_scale.map(value), label.clone()))
        .filter(|(share, _)| (0.0..=1.0).contains(share))
        .collect();
    // A full circle puts the last break on the first: one label says both.
    if let [(first, _), .., (last, _)] = marks[..] {
        let ((x0, y0), (x1, y1)) = (warp.at(first, Warp::RING), warp.at(last, Warp::RING));
        if (x1 - x0).hypot(y1 - y0) <= MERGE_DISTANCE
            && let Some((_, last_label)) = marks.pop()
        {
            marks[0].1.text = format!("{}/{}", last_label.text, marks[0].1.text);
        }
    }
    let length = theme
        .axis_ticks_x
        .map_or(0.0, |_| theme.axis_ticks_length.0);
    let offset = length + theme.axis_text_spacing.0;
    for (share, label) in &marks {
        let tip = warp.at(*share, Warp::RING);
        let (out_x, out_y) = (tip.0 - warp.center.0, tip.1 - warp.center.1);
        let norm = out_x.hypot(out_y).max(f64::EPSILON);
        let (out_x, out_y) = (out_x / norm, out_y / norm);
        if let Some(line) = theme.axis_ticks_x {
            let end = (tip.0 + out_x * length, tip.1 + out_y * length);
            nodes.push(segment(tip, end, line));
        }
        if let Some(style) = theme.axis_text_x {
            let (align, shift) = label_alignment(out_y.atan2(out_x).to_degrees());
            let anchor = (
                tip.0 + out_x * offset,
                tip.1 + out_y * offset + shift * text_band(style) / 2.0,
            );
            nodes.push(label_node(label, style, anchor, align, 0.0));
        }
    }
    nodes
}

/// The radius axis, on the panel's edge from the centre up to the ring: the usual
/// axis, drawn on the upper half of the panel.
fn radius_axis(
    warp: &Warp,
    layout: &PlotLayout,
    theme: &Theme,
    panel: &Panel,
    breaks: AxisBreaks,
    shown: AxisSides,
) -> Vec<Node> {
    let mut scale = panel.y_scale.clone();
    let (low, high) = scale.expanded_domain();
    scale.set_view_domain(low, low + (high - low) * Warp::RING);
    let height = warp.radius * Warp::RING;
    let rect = Rect {
        x: layout.panel.x,
        y: warp.center.1 - height,
        width: layout.panel.width,
        height,
    };
    let (values, labels) = breaks
        .values
        .iter()
        .zip(&breaks.labels)
        .filter(|(value, _)| (0.0..=1.0).contains(&scale.map(**value)))
        .map(|(value, label)| (*value, label.clone()))
        .unzip();
    let mut upper = layout.clone();
    upper.panel = rect;
    upper.y_breaks = AxisBreaks {
        values,
        labels,
        ..breaks
    };
    upper.secondary = [None, None];
    let mapper = Panel {
        rect,
        y_scale: &scale,
        ..*panel
    };
    let none = AxisSides {
        primary: false,
        secondary: false,
    };
    let sides = AxisSides {
        primary: shown.primary,
        secondary: shown.secondary && !shown.primary,
    };
    draw_axes(&upper, theme, &mapper, [none, sides])
}

/// One polar panel: background and grid, the bent `layers`, the border, then the axes
/// (the radius axis only where `shown` asks, the theta axis always).
pub fn draw_polar_tile(
    view: PolarView,
    layout: &PlotLayout,
    theme: &Theme,
    panel: &Panel,
    layers: Vec<Node>,
    shown: [AxisSides; 2],
) -> Vec<Node> {
    let warp = view.warp(layout.panel);
    let (theta, radius) = polar_breaks(panel, theme);
    let mut nodes = Vec::new();
    if let Some(style) = theme.panel_background {
        nodes.push(background(view, &warp, layout, style));
    }
    let lines = grid(&warp, panel, theme, &theta.values, &radius.values);
    if !lines.is_empty() {
        nodes.push(Node::Group(Group {
            key: None,
            clip: None,
            role: Some("grid"),
            anchor: None,
            nodes: lines,
        }));
    }
    nodes.push(Node::Group(Group {
        key: None,
        clip: Some(layout.panel),
        role: Some("panel"),
        anchor: None,
        nodes: warp.nodes(layers),
    }));
    if let Some(line) = theme.panel_border {
        nodes.push(match view.transform_background {
            true => ring(&warp, Warp::RING, line),
            false => Node::Rects(Rects {
                rects: vec![layout.panel],
                fill: PerItem::Uniform(None),
                stroke: Some(line.stroke()),
            }),
        });
    }
    nodes.extend(theta_axis(&warp, panel, theme, &theta));
    nodes.extend(radius_axis(&warp, layout, theme, panel, radius, shown[1]));
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_sit_away_from_the_circle() {
        assert_eq!(label_alignment(-90.0), (HorizontalAlign::Middle, -1.0));
        assert_eq!(label_alignment(90.0), (HorizontalAlign::Middle, 1.0));
        assert_eq!(label_alignment(0.0), (HorizontalAlign::Start, 0.0));
        assert_eq!(label_alignment(180.0), (HorizontalAlign::End, 0.0));
        assert_eq!(label_alignment(45.0), (HorizontalAlign::Start, 1.0));
        assert_eq!(label_alignment(135.0), (HorizontalAlign::End, 1.0));
        assert_eq!(label_alignment(-45.0), (HorizontalAlign::Start, -1.0));
        assert_eq!(label_alignment(-135.0), (HorizontalAlign::End, -1.0));
    }
}
