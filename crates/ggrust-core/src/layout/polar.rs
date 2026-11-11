//! The panel of a `coord_polar` plot (lets-plot `PolarFrameOfReference`,
//! `PolarAxisUtil`): the background and border as a circle, the grid as spokes and
//! circles, and the layers bent by [`Warp`].
//!
//! What it does not do: axis lines, ticks and labels (see `draw_axes` below).

use super::axis::{Orientation, choose_breaks};
use super::{AxisBreaks, PlotLayout, minor_breaks, segment};
use crate::coord::{Panel, PolarView, Warp};
use crate::scene::{Group, Node, Path, PerItem, Rects};
use crate::theme::{LineStyle, RectStyle, Theme};

/// Points of a circle's outline; at a radius of 300 px a chord leaves it by 0.01 px.
const CIRCLE_POINTS: usize = 720;

/// Breaks of a polar panel: theta along its circumference (lets-plot
/// `PolarTileLayout`: 1.5 x the width), the radius along half the height.
fn polar_breaks(panel: &Panel, theme: &Theme) -> (AxisBreaks, AxisBreaks) {
    let rect = panel.rect;
    (
        choose_breaks(
            panel.x_scale,
            rect.width * 1.5,
            Orientation::Horizontal,
            None,
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

/// One polar panel: background and grid, the bent `layers`, then the border.
pub fn draw_polar_tile(
    view: PolarView,
    layout: &PlotLayout,
    theme: &Theme,
    panel: &Panel,
    layers: Vec<Node>,
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
    nodes
}
