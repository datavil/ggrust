//! Dendrograms beside a discrete axis (legendry `scale_*_dendro` +
//! `guide_axis_dendro`).
//!
//! A discrete scale may carry an [`AxisDendrogram`]: the tree orders the axis
//! levels (see `scale::spec`) and is drawn in a band of its own. On the axis side
//! (`side = "axis"`, as legendry) the band is past the tick labels, the root
//! farthest from the panel; on the opposite side (`"opposite"`, as scanpy) it is
//! above or right of the panel, without labels. The band is part of the axis
//! area, so `gggrid(align=True)` aligns panels as usual. The paths sit in a
//! `dendrogram-x` / `dendrogram-y` group clipped to the panel's span along the axis,
//! which the HTML runtime moves with zoom and pan along that axis.
//!
//! What it does not do: facets (the build drops the tree with a warning),
//! labels inside the guide (legendry `labels=`; hide axis text with the theme instead).

use crate::cluster::{DendrogramType, Tree};
use crate::scene::{Color, Group, Node, Path, Rect};
use crate::theme::{Dash, LineStyle, Theme};

/// Where the tree goes relative to the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DendrogramSide {
    /// With the axis, past its labels (left of y labels, below x labels; right or
    /// above them for an axis at `position="right"` / `"top"`; the primary side
    /// for `"both"`).
    #[default]
    Axis,
    /// Across the panel from the axis (right for y, top for x by default).
    Opposite,
}

impl AxisDendrogram {
    /// Whether the tree goes on the secondary side (right, top) of an axis drawn
    /// on `sides`.
    pub fn on_secondary(&self, sides: crate::scale::AxisSides) -> bool {
        match self.side {
            DendrogramSide::Axis => !sides.primary,
            DendrogramSide::Opposite => sides.primary,
        }
    }
}

/// A tree drawn beside a discrete axis.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisDendrogram {
    /// The clustering; its leaves are the axis levels.
    pub tree: Tree,
    /// Side of the panel.
    pub side: DendrogramSide,
    /// Pixels from the leaves to the root.
    pub space: f64,
    /// Elbow or straight links.
    pub kind: DendrogramType,
}

/// Default `space` (px): legendry's `rel(10)` of the tick length, rounded.
pub const DEFAULT_SPACE: f64 = 40.0;

/// Line of the tree when the theme has no axis ticks to borrow from: lets-plot's
/// tick look (pen color, 1 px).
fn default_line() -> LineStyle {
    LineStyle {
        color: Color::rgb(0x47, 0x47, 0x47),
        width: 1.0,
        dash: Dash::default(),
    }
}

/// The band (px) a dendrogram takes on the `secondary` (or primary) side of an
/// axis drawn on `sides`: the gap to the axis labels (or the panel) plus the tree.
/// 0 without one there.
pub fn side_band(
    dendrogram: Option<&AxisDendrogram>,
    sides: crate::scale::AxisSides,
    secondary: bool,
    gap: f64,
) -> f64 {
    dendrogram
        .filter(|dendrogram| dendrogram.on_secondary(sides) == secondary && dendrogram.space > 0.0)
        .map_or(0.0, |dendrogram| gap + dendrogram.space)
}

/// Which way the tree grows from its leaves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Growth {
    /// Leaves along a horizontal axis, root above them (`-1` px per unit) or below.
    Vertical(f64),
    /// Leaves along a vertical axis, root left (`-1`) or right (`+1`).
    Horizontal(f64),
}

/// The tree's segments in scene pixels: `leaf_pixel` maps an axis position to a
/// pixel along the axis, `baseline` is the leaves' pixel across it and `growth`
/// the direction of the root. `levels` are the axis categories in axis order;
/// leaves not on the axis are not drawn (the build rejects such trees).
pub fn draw(
    dendrogram: &AxisDendrogram,
    levels: &[&str],
    leaf_pixel: &dyn Fn(f64) -> f64,
    baseline: f64,
    growth: Growth,
    theme_line: Option<LineStyle>,
) -> Vec<Node> {
    let tree = &dendrogram.tree;
    if dendrogram.space <= 0.0 {
        return Vec::new();
    }
    let mut leaf_position = Vec::with_capacity(tree.labels.len());
    for label in &tree.labels {
        match levels.iter().position(|level| level == label) {
            Some(position) => leaf_position.push(position as f64),
            None => return Vec::new(),
        }
    }
    let top = tree.max_height();
    let per_unit = if top > 0.0 {
        dendrogram.space.max(0.0) / top
    } else {
        0.0
    };
    let line = theme_line.unwrap_or_else(default_line);
    let segments = tree.segments_at(&leaf_position, dendrogram.kind);
    let place = |position: f64, height: f64| {
        let along = leaf_pixel(position);
        match growth {
            Growth::Vertical(sign) => (along, baseline + sign * height * per_unit),
            Growth::Horizontal(sign) => (baseline + sign * height * per_unit, along),
        }
    };
    let path = |points: Vec<(f64, f64)>| {
        Node::Path(Path {
            points,
            closed: false,
            fill: None,
            stroke: Some(line.stroke()),
        })
    };
    match dendrogram.kind {
        // One polyline per merge (left drop, across, right drop): no seam where
        // the two halves of the link meet.
        DendrogramType::Rectangle => segments
            .chunks_exact(4)
            .map(|link| {
                let (left, right) = (link[0], link[2]);
                path(vec![
                    place(left.position, left.height_end),
                    place(left.position, left.height),
                    place(right.position, right.height),
                    place(right.position, right.height_end),
                ])
            })
            .collect(),
        DendrogramType::Triangle => segments
            .into_iter()
            .map(|segment| {
                path(vec![
                    place(segment.position, segment.height),
                    place(segment.position_end, segment.height_end),
                ])
            })
            .collect(),
    }
}

/// The tree's paths in a group with `role` (`dendrogram-x` / `dendrogram-y`, which
/// the HTML runtime moves with the zoom along the axis), clipped to the panel's
/// span along the axis and to the tree's band (`baseline`, `space`, `growth`)
/// across it, plus the stroke. None without paths.
pub fn group(
    role: &'static str,
    paths: Vec<Node>,
    panel: Rect,
    (baseline, space, growth): (f64, f64, Growth),
    theme_line: Option<LineStyle>,
) -> Option<Node> {
    if paths.is_empty() {
        return None;
    }
    let pad = theme_line.unwrap_or_else(default_line).width.max(1.0);
    let across = |sign: f64| {
        let end = baseline + sign * space;
        (baseline.min(end) - pad, (baseline - end).abs() + 2.0 * pad)
    };
    let clip = match growth {
        Growth::Vertical(sign) => {
            let (y, height) = across(sign);
            Rect {
                x: panel.x,
                y,
                width: panel.width,
                height,
            }
        }
        Growth::Horizontal(sign) => {
            let (x, width) = across(sign);
            Rect {
                x,
                y: panel.y,
                width,
                height: panel.height,
            }
        }
    };
    Some(Node::Group(Group {
        clip: Some(clip),
        role: Some(role),
        anchor: None,
        nodes: paths,
    }))
}

/// Gap (px) between the tree and the labels or panel: the theme's label spacing
/// of that axis.
pub fn gap(theme: &Theme, horizontal: bool) -> f64 {
    if horizontal {
        theme.axis_text_spacing.0
    } else {
        theme.axis_text_spacing.1
    }
}
