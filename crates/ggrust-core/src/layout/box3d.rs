//! The axis box of a `coord_3d` panel (see [`crate::coord3d`]).
//!
//! Back panes: the three box faces away from the viewer, filled with
//! `panel_background` and gridded at the breaks with `panel_grid_major_*`
//! (x breaks with `_x`, y and z breaks with `_y`). Labelled edges, as matplotlib
//! `mplot3d`: x along the floor's front edge in y, y along the floor's front edge
//! in x, z up the side corner on the left. Each edge gets its axis line
//! (`axis_line_x` for x, `axis_line_y` for y and z, or the grid style when blank),
//! ticks, labels and title (`axis_text_*`, `axis_ticks_*`, `axis_title_*`, x
//! elements for x, y elements for y and z), all pointing away from the box.
//!
//! What it does not do: minor grid lines, label overlap removal, secondary axes.

use super::text_node;
use crate::coord3d::{Projector, Scales3d};
use crate::scale::PositionScale;
use crate::scale::format::Label;
use crate::scene::{HorizontalAlign, Node, Path, Rect, Text, VerticalAlign};
use crate::text::{measure, measure_label};
use crate::theme::{LineStyle, TextStyle, Theme};

/// Breaks asked of each axis scale at most.
const TARGET_BREAKS: usize = 5;
/// Pixels of edge length per break asked (fewer breaks on a small box).
const PIXELS_PER_BREAK: f64 = 30.0;
/// Gap between parts (labels, title) in pixels.
const GAP: f64 = 4.0;

/// Axis titles of a 3D panel (`None`: no title).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Titles3d {
    /// x.
    pub x: Option<String>,
    /// y.
    pub y: Option<String>,
    /// z.
    pub z: Option<String>,
}

/// Scaled breaks inside 0..1 and their labels, about `target` of them.
fn breaks(scale: &PositionScale, target: usize) -> Vec<(f64, Label)> {
    let breaks = scale.breaks(target);
    breaks
        .positions
        .iter()
        .zip(breaks.labels)
        .map(|(position, label)| (scale.map(*position), label))
        .filter(|(at, _)| (-1e-9..=1.0 + 1e-9).contains(at))
        .collect()
}

fn label_width(label: &Label, style: TextStyle) -> f64 {
    measure_label(
        &label.text,
        label.superscript.as_deref(),
        label.suffix.as_deref(),
        style.size,
        style.font,
    )
}

fn line_height(style: TextStyle) -> f64 {
    measure("", style.size, style.font).height()
}

/// Room kept around the box on every side for tick labels and titles: the box
/// is fitted into the panel shrunk by it.
pub fn label_room(scales: &Scales3d, theme: &Theme, titles: &Titles3d) -> f64 {
    let mut room: f64 = GAP;
    let axes = [
        (scales.x, theme.axis_text_x, theme.axis_title_x, &titles.x),
        (scales.y, theme.axis_text_y, theme.axis_title_y, &titles.y),
        (scales.z, theme.axis_text_y, theme.axis_title_y, &titles.z),
    ];
    for (scale, text, title_style, title) in axes {
        let mut extent = tick_length(theme) + GAP;
        if let Some(text) = text {
            let widest = breaks(scale, TARGET_BREAKS)
                .iter()
                .map(|(_, label)| label_width(label, text))
                .fold(0.0, f64::max);
            extent += widest.max(line_height(text)) + GAP;
        }
        if let (Some(style), Some(_)) = (title_style, title.as_deref().filter(|t| !t.is_empty())) {
            extent += line_height(style) + GAP;
        }
        room = room.max(extent);
    }
    room
}

fn tick_length(theme: &Theme) -> f64 {
    theme.axis_ticks_length.0.max(theme.axis_ticks_length.1)
}

/// Breaks of x, y and z, about one per [`PIXELS_PER_BREAK`] of their labelled
/// edge (3 to [`TARGET_BREAKS`]; none on an edge shorter than one break).
fn axis_breaks(projector: &Projector, scales: &Scales3d) -> [Vec<(f64, Label)>; 3] {
    let [x, y, z] = edges(projector).map(|(edge, _)| {
        let (start, end) = (
            projector.pixel(edge.position(0.0)),
            projector.pixel(edge.position(1.0)),
        );
        let length = (end.0 - start.0).hypot(end.1 - start.1);
        // An edge seen end-on (or nearly) has no room: no breaks, labels would pile up.
        if length.is_nan() || length < PIXELS_PER_BREAK {
            return 0;
        }
        ((length / PIXELS_PER_BREAK) as usize).clamp(3, TARGET_BREAKS)
    });
    let some = |scale: &PositionScale, target: usize| match target {
        0 => Vec::new(),
        target => breaks(scale, target),
    };
    [some(scales.x, x), some(scales.y, y), some(scales.z, z)]
}

/// Which side of each axis is far from the viewer (scaled position 0 or 1).
fn far_sides(projector: &Projector) -> [f64; 3] {
    projector
        .view()
        .eye()
        .map(|toward| if toward >= 0.0 { 0.0 } else { 1.0 })
}

fn line(points: Vec<(f64, f64)>, style: LineStyle) -> Node {
    Node::Path(Path {
        points,
        closed: false,
        fill: None,
        stroke: Some(style.stroke()),
    })
}

/// The back panes with their grid lines, drawn under the data.
pub fn draw_back(projector: &Projector, scales: &Scales3d, theme: &Theme) -> Vec<Node> {
    let far = far_sides(projector);
    let axis_breaks = axis_breaks(projector, scales);
    let grid = [
        theme.panel_grid_major_x,
        theme.panel_grid_major_y,
        theme.panel_grid_major_y,
    ];
    let mut nodes = Vec::new();
    // One pane per axis: the face at that axis's far side.
    for normal in 0..3 {
        let (first, second) = ((normal + 1) % 3, (normal + 2) % 3);
        let at = |a: f64, b: f64| {
            let mut position = [0.0; 3];
            position[normal] = far[normal];
            position[first] = a;
            position[second] = b;
            projector.pixel(position)
        };
        let corners = vec![at(0.0, 0.0), at(1.0, 0.0), at(1.0, 1.0), at(0.0, 1.0)];
        // The pane: its background, outlined by its border, else the grid line.
        let outline = theme
            .panel_background
            .and_then(|background| background.border)
            .or(grid[first])
            .or(grid[second]);
        nodes.push(Node::Path(Path {
            points: corners,
            closed: true,
            fill: theme.panel_background.map(|background| background.fill),
            stroke: outline.map(LineStyle::stroke),
        }));
        for axis in [first, second] {
            let Some(style) = grid[axis] else { continue };
            for (value, _) in &axis_breaks[axis] {
                let ends = if axis == first {
                    (at(*value, 0.0), at(*value, 1.0))
                } else {
                    (at(0.0, *value), at(1.0, *value))
                };
                nodes.push(line(vec![ends.0, ends.1], style));
            }
        }
    }
    nodes
}

/// One labelled edge: the axis it runs along and the fixed positions of the two
/// other axes.
struct Edge {
    axis: usize,
    fixed: [f64; 3],
}

impl Edge {
    fn position(&self, value: f64) -> [f64; 3] {
        let mut position = self.fixed;
        position[self.axis] = value;
        position
    }
}

/// The labelled edge of each axis and the 3D direction its labels point to.
fn edges(projector: &Projector) -> [(Edge, [f64; 3]); 3] {
    let far = far_sides(projector);
    let near = far.map(|side| 1.0 - side);
    let floor = far[2];
    let away = |side: f64| if side >= 0.5 { 1.0 } else { -1.0 };
    let x_edge = Edge {
        axis: 0,
        fixed: [0.0, near[1], floor],
    };
    let y_edge = Edge {
        axis: 1,
        fixed: [near[0], 0.0, floor],
    };
    // z: of the two side corners (one near and one far side), the leftmost.
    let corners = [(far[0], near[1]), (near[0], far[1])];
    let (corner_x, corner_y) = corners
        .into_iter()
        .min_by(|left, right| {
            let screen = |(x, y): (f64, f64)| projector.pixel([x, y, 0.5]).0;
            screen(*left).total_cmp(&screen(*right))
        })
        .unwrap_or(corners[0]);
    let z_edge = Edge {
        axis: 2,
        fixed: [corner_x, corner_y, 0.0],
    };
    [
        (x_edge, [0.0, away(near[1]), 0.0]),
        (y_edge, [away(near[0]), 0.0, 0.0]),
        (z_edge, [away(corner_x), away(corner_y), 0.0]),
    ]
}

/// Screen direction (unit) of a 3D direction at a point; `fallback` when the
/// direction points at the viewer.
fn screen_direction(
    projector: &Projector,
    at: [f64; 3],
    direction: [f64; 3],
    fallback: (f64, f64),
) -> (f64, f64) {
    let start = projector.pixel(at);
    let end = projector.pixel([0, 1, 2].map(|axis| at[axis] + direction[axis] * 0.1));
    let (dx, dy) = (end.0 - start.0, end.1 - start.1);
    let length = dx.hypot(dy);
    if length < 1e-6 || !length.is_finite() {
        fallback
    } else {
        (dx / length, dy / length)
    }
}

/// Text alignment for text placed beyond an anchor in screen direction `(dx, dy)`.
fn alignment((dx, dy): (f64, f64)) -> (HorizontalAlign, VerticalAlign) {
    let horizontal = if dx > 0.4 {
        HorizontalAlign::Start
    } else if dx < -0.4 {
        HorizontalAlign::End
    } else {
        HorizontalAlign::Middle
    };
    let vertical = if dy > 0.4 {
        VerticalAlign::Top
    } else if dy < -0.4 {
        VerticalAlign::Bottom
    } else {
        VerticalAlign::Middle
    };
    (horizontal, vertical)
}

/// Edges, ticks, labels and titles, drawn over the data.
pub fn draw_front(
    projector: &Projector,
    scales: &Scales3d,
    theme: &Theme,
    titles: &Titles3d,
) -> Vec<Node> {
    let title_of = [&titles.x, &titles.y, &titles.z];
    let x_elements = (
        theme.axis_line_x.map(|line| line.style),
        theme.axis_ticks_x,
        theme.axis_text_x,
        theme.axis_title_x,
        theme.panel_grid_major_x,
    );
    let y_elements = (
        theme.axis_line_y.map(|line| line.style),
        theme.axis_ticks_y,
        theme.axis_text_y,
        theme.axis_title_y,
        theme.panel_grid_major_y,
    );
    let fallback = [(0.0, 1.0), (0.0, 1.0), (-1.0, 0.0)];
    let axis_breaks = axis_breaks(projector, scales);
    let mut nodes = Vec::new();
    for (index, (edge, outward)) in edges(projector).into_iter().enumerate() {
        let (axis_line, ticks, text, title_style, grid) =
            if index == 0 { x_elements } else { y_elements };
        if let Some(style) = axis_line.or(grid) {
            nodes.push(line(
                vec![
                    projector.pixel(edge.position(0.0)),
                    projector.pixel(edge.position(1.0)),
                ],
                style,
            ));
        }
        let middle = edge.position(0.5);
        let direction = screen_direction(projector, middle, outward, fallback[index]);
        let (horizontal, vertical) = alignment(direction);
        let tick = tick_length(theme);
        let mut label_extent: f64 = 0.0;
        for (value, label) in &axis_breaks[index] {
            let value = *value;
            let (x, y) = projector.pixel(edge.position(value));
            if let Some(style) = ticks {
                nodes.push(Node::Path(Path {
                    points: vec![(x, y), (x + direction.0 * tick, y + direction.1 * tick)],
                    closed: false,
                    fill: None,
                    stroke: Some(style.stroke()),
                }));
            }
            let Some(style) = text else { continue };
            let offset = tick + GAP;
            let anchor = (x + direction.0 * offset, y + direction.1 * offset);
            let mut node = text_node(&label.text, style, anchor, horizontal, 0.0);
            if let Node::Text(Text {
                superscript,
                suffix,
                vertical: text_vertical,
                ..
            }) = &mut node
            {
                *superscript = label.superscript.clone();
                *suffix = label.suffix.clone();
                *text_vertical = vertical;
            }
            nodes.push(node);
            let (width, height) = (label_width(label, style), line_height(style));
            label_extent = label_extent.max(direction.0.abs() * width + direction.1.abs() * height);
        }
        let (Some(style), Some(title)) = (
            title_style,
            title_of[index].as_deref().filter(|title| !title.is_empty()),
        ) else {
            continue;
        };
        let offset = tick + GAP + label_extent + GAP;
        let (x, y) = projector.pixel(middle);
        let anchor = (x + direction.0 * offset, y + direction.1 * offset);
        let mut node = text_node(title, style, anchor, horizontal, 0.0);
        if let Node::Text(text) = &mut node {
            text.vertical = vertical;
        }
        nodes.push(node);
    }
    nodes
}

/// The panel rectangle shrunk by [`label_room`] (never below 1 px).
pub fn box_rect(panel: Rect, room: f64) -> Rect {
    let room_x = room.min((panel.width - 1.0).max(0.0) / 2.0);
    let room_y = room.min((panel.height - 1.0).max(0.0) / 2.0);
    Rect {
        x: panel.x + room_x,
        y: panel.y + room_y,
        width: panel.width - 2.0 * room_x,
        height: panel.height - 2.0 * room_y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coord3d::View;
    use crate::scale::ContinuousPositionScale;
    use crate::spec::CoordSpec;

    fn projector(azimuth: f64, elevation: f64) -> Projector {
        let spec = CoordSpec {
            name: "3d".into(),
            xlim: None,
            ylim: None,
            ratio: None,
            flip: false,
            azimuth: Some(azimuth),
            elevation: Some(elevation),
            perspective: None,
            unsupported: Default::default(),
        };
        let view = View::from_spec(&spec).unwrap();
        Projector::fit(
            view,
            Rect {
                x: 0.0,
                y: 0.0,
                width: 300.0,
                height: 300.0,
            },
        )
    }

    #[test]
    fn back_panes_face_away_from_the_viewer() {
        // Azimuth -60, elevation 30 looks from +x, -y, +z.
        let far = far_sides(&projector(-60.0, 30.0));
        assert_eq!(far, [0.0, 1.0, 0.0]);
        let far = far_sides(&projector(120.0, -30.0));
        assert_eq!(far, [1.0, 0.0, 1.0]);
    }

    #[test]
    fn labels_point_away_from_the_box() {
        let projector = projector(-60.0, 30.0);
        let centre = projector.pixel([0.5, 0.5, 0.5]);
        for (edge, outward) in edges(&projector) {
            let middle = edge.position(0.5);
            let (x, y) = projector.pixel(middle);
            let direction = screen_direction(&projector, middle, outward, (0.0, 1.0));
            let towards_centre = (centre.0 - x) * direction.0 + (centre.1 - y) * direction.1;
            assert!(towards_centre < 0.0, "axis {} points inward", edge.axis);
        }
    }

    #[test]
    fn breaks_stay_inside_the_box() {
        let mut scale = ContinuousPositionScale::default();
        scale.train_range(0.0, 10.0);
        let scale = PositionScale::Continuous(scale);
        let found = breaks(&scale, 5);
        assert!(!found.is_empty());
        assert!(found.iter().all(|(at, _)| (0.0..=1.0).contains(at)));
    }
}
