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
//! A leaf may stand for a run of levels (`leaf_levels`, for example the cells of
//! one cluster): the runs follow the tree's order, each keeps its given level
//! order, and the leaf is drawn at the centre of its run.
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
    /// The axis levels of each leaf, indexed like `tree.labels` (`scale_*_dendro(
    /// leaf_levels=)`); None: each leaf is the level of its own name.
    pub leaf_levels: Option<Vec<Vec<String>>>,
}

impl AxisDendrogram {
    /// The levels of leaf `leaf` (an index into `tree.labels`).
    fn levels_of(&self, leaf: usize) -> &[String] {
        match &self.leaf_levels {
            Some(runs) => &runs[leaf],
            None => std::slice::from_ref(&self.tree.labels[leaf]),
        }
    }

    /// The axis levels in the tree's order: the leaves, or their runs end to end.
    pub fn ordered_levels(&self) -> Vec<String> {
        self.tree
            .order
            .iter()
            .flat_map(|leaf| self.levels_of(*leaf).iter().cloned())
            .collect()
    }

    /// Every level the tree places on the axis.
    pub fn level_set(&self) -> std::collections::HashSet<&str> {
        (0..self.tree.labels.len())
            .flat_map(|leaf| self.levels_of(leaf).iter().map(String::as_str))
            .collect()
    }
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
/// a leaf sits at the centre of its levels (one level, or a run). Trees with a
/// level not on the axis are not drawn (the build rejects such trees).
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
    let index: std::collections::HashMap<&str, usize> = levels
        .iter()
        .enumerate()
        .map(|(position, level)| (*level, position))
        .collect();
    let mut leaf_position = Vec::with_capacity(tree.labels.len());
    for leaf in 0..tree.labels.len() {
        let mut span: Option<(usize, usize)> = None;
        for level in dendrogram.levels_of(leaf) {
            let Some(&position) = index.get(level.as_str()) else {
                return Vec::new();
            };
            span = Some(span.map_or((position, position), |(low, high)| {
                (low.min(position), high.max(position))
            }));
        }
        let Some((low, high)) = span else {
            return Vec::new();
        };
        leaf_position.push((low + high) as f64 / 2.0);
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
        key: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn dendrogram(leaf_levels: Option<Vec<Vec<String>>>) -> AxisDendrogram {
        let labels = ["a", "b"].map(str::to_owned).to_vec();
        AxisDendrogram {
            tree: Tree::from_spec("tree", labels, vec![[-1, -2]], vec![1.0], vec![0, 1]).unwrap(),
            side: DendrogramSide::Axis,
            space: 10.0,
            kind: DendrogramType::Rectangle,
            leaf_levels,
        }
    }

    /// The along-axis pixels of the two leaf drops of a one-merge tree.
    fn leaf_pixels(dendrogram: &AxisDendrogram, levels: &[&str]) -> Vec<f64> {
        let paths = draw(
            dendrogram,
            levels,
            &|position| position * 10.0,
            0.0,
            Growth::Vertical(-1.0),
            None,
        );
        match paths.as_slice() {
            [Node::Path(path)] => vec![path.points[0].0, path.points[3].0],
            other => panic!("expected one link, got {other:?}"),
        }
    }

    #[test]
    fn leaves_sit_at_their_level_or_the_centre_of_their_run() {
        assert_eq!(leaf_pixels(&dendrogram(None), &["a", "b"]), [0.0, 10.0]);
        let runs = vec![
            vec!["a1".to_owned(), "a2".to_owned()],
            vec!["b1".to_owned(), "b2".to_owned(), "b3".to_owned()],
        ];
        let tree = dendrogram(Some(runs));
        assert_eq!(tree.ordered_levels(), ["a1", "a2", "b1", "b2", "b3"]);
        assert_eq!(
            leaf_pixels(&tree, &["a1", "a2", "b1", "b2", "b3"]),
            [5.0, 30.0]
        );
        // Reversed axis: the runs swap ends, the centres follow.
        assert_eq!(
            leaf_pixels(&tree, &["b3", "b2", "b1", "a2", "a1"]),
            [35.0, 10.0]
        );
        // A run level off the axis: no tree.
        assert!(
            draw(
                &tree,
                &["a1", "b1"],
                &|p| p,
                0.0,
                Growth::Vertical(-1.0),
                None
            )
            .is_empty()
        );
    }
}
