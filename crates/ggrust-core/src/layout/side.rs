//! Axis sides: which sides of a panel an axis is drawn on (`position=` of a
//! position scale, see [`AxisSides`]) and the nodes of one side of an axis (line,
//! ticks, tick labels), shared by the static axes and the HTML zoom templates.
//!
//! A side is drawn from its `base`, the pixel across the axis where its line runs:
//! the panel edge, or past the facet strips on that side. Bottom and left axes are
//! drawn exactly as before sides existed; top and right axes mirror them: ticks
//! and labels grow outward, simple labels keep their offset from the axis, tilted
//! and vertical labels still end (start) at their tick, and labels with a theme
//! angle keep their box, mirrored across the axis line.
//!
//! What it does not do: give each side its own breaks (a secondary axis brings its
//! own [`AxisBreaks`] to the same functions) or side-specific theme elements
//! (`axis_text_x_top` and friends; lets-plot has none).

use super::rotated::{self, Placement};
use super::{
    AxisBreaks, LabelArrangement, PlotLayout, TILT_DEGREES, axis_line_node, label_node,
    label_sizes, placed_label_node, segment, x_label_node, x_tick_band, y_tick_band,
};
use crate::scale::AxisSides;
use crate::scale::format::Label;
use crate::scene::{HorizontalAlign, Node, VerticalAlign};
use crate::theme::{LineStyle, TextStyle, Theme};

/// A side of the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Below the panel (primary x).
    Bottom,
    /// Above the panel (secondary x).
    Top,
    /// Left of the panel (primary y).
    Left,
    /// Right of the panel (secondary y).
    Right,
}

impl Side {
    /// The side of a horizontal (x) or vertical (y) axis.
    pub fn of(horizontal: bool, secondary: bool) -> Side {
        match (horizontal, secondary) {
            (true, false) => Side::Bottom,
            (true, true) => Side::Top,
            (false, false) => Side::Left,
            (false, true) => Side::Right,
        }
    }

    /// Whether an axis on this side runs horizontally (bottom, top).
    pub fn is_horizontal(self) -> bool {
        matches!(self, Side::Bottom | Side::Top)
    }

    /// Whether this is the secondary side (top, right).
    pub fn is_secondary(self) -> bool {
        matches!(self, Side::Top | Side::Right)
    }

    /// Away from the panel in pixels: +1 down or right, -1 up or left.
    pub fn outward(self) -> f64 {
        if self.is_secondary() == self.is_horizontal() {
            -1.0
        } else {
            1.0
        }
    }
}

impl Side {
    /// Every side: bottom, top, left, right (the order of [`Side::index`]).
    pub const ALL: [Side; 4] = [Side::Bottom, Side::Top, Side::Left, Side::Right];

    /// Position in [`Side::ALL`].
    pub fn index(self) -> usize {
        match self {
            Side::Bottom => 0,
            Side::Top => 1,
            Side::Left => 2,
            Side::Right => 3,
        }
    }
}

/// What happens to an axis title a grid collected (`gggrid(axis_titles=)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TitleRoom {
    /// Drawn by the plot (not collected).
    #[default]
    Drawn,
    /// Keeps its room but is not drawn: another plot's title stands for it, or
    /// the grid draws it across a run of plots.
    Hidden,
    /// Removed with its room.
    Dropped,
}

/// Axes and axis titles a grid collected from one plot (`gggrid(axes=,
/// axis_titles=)`, patchwork's `plot_layout(axes=, axis_titles=)`), per screen
/// side in [`Side::ALL`] order. A collected axis (line, ticks, labels, and a
/// nested guide or tree on that side) takes no room and is not drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CollectedSides {
    /// Whether the axis on each side is collected.
    pub axes: [bool; 4],
    /// The axis title on each side.
    pub titles: [TitleRoom; 4],
}

impl CollectedSides {
    /// Whether the axis on `side` is collected (not drawn, no room).
    pub fn axis_hidden(&self, side: Side) -> bool {
        self.axes[side.index()]
    }

    /// The title on `side`.
    pub fn title(&self, side: Side) -> TitleRoom {
        self.titles[side.index()]
    }

    /// `sides` (horizontal, vertical) without the collected sides: what is drawn.
    pub fn drawn(&self, sides: [AxisSides; 2]) -> [AxisSides; 2] {
        let [horizontal, vertical] = sides;
        [
            AxisSides {
                primary: horizontal.primary && !self.axis_hidden(Side::Bottom),
                secondary: horizontal.secondary && !self.axis_hidden(Side::Top),
            },
            AxisSides {
                primary: vertical.primary && !self.axis_hidden(Side::Left),
                secondary: vertical.secondary && !self.axis_hidden(Side::Right),
            },
        ]
    }
}

/// The screen sides an axis of `sides` is drawn on, primary first.
pub fn screen_sides(sides: AxisSides, horizontal: bool) -> impl Iterator<Item = Side> {
    [(sides.primary, false), (sides.secondary, true)]
        .into_iter()
        .filter(|(shown, _)| *shown)
        .map(move |(_, secondary)| Side::of(horizontal, secondary))
}

/// Whether the axis title of `sides` goes on the secondary side: only when the
/// axis is there alone (`"both"` keeps the title at the primary side, as lets-plot).
pub fn title_on_secondary(sides: AxisSides) -> bool {
    sides.secondary && !sides.primary
}

/// One side of an axis: the side and the pixel across the axis its line runs at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct AxisEdge {
    /// The side.
    pub side: Side,
    /// The line's pixel: y for bottom/top, x for left/right.
    pub base: f64,
}

impl AxisEdge {
    /// The edge of `layout`'s panel on `side`, past the facet strips there.
    pub fn of(layout: &PlotLayout, side: Side) -> AxisEdge {
        let panel = layout.panel;
        // Past a side panel too (it sits between the panel and the axis).
        let band = layout.band_extent(side);
        let base = match side {
            Side::Bottom => panel.y + panel.height + band,
            Side::Top => panel.y - layout.top_band - band,
            Side::Left => panel.x - band,
            Side::Right => panel.x + panel.width + layout.right_band + band,
        };
        AxisEdge { side, base }
    }

    /// The point at `along` (pixel along the axis) on the line moved `across`
    /// pixels outward.
    pub(super) fn point(self, along: f64, across: f64) -> (f64, f64) {
        let across = self.base + self.side.outward() * across;
        if self.side.is_horizontal() {
            (along, across)
        } else {
            (across, along)
        }
    }

    /// A tick `length` long at `along`. Bottom and left ticks keep their
    /// historical point order (outer end last on the bottom, first on the left).
    pub fn tick(self, along: f64, length: f64, line: LineStyle) -> Node {
        match self.side {
            Side::Left => segment(self.point(along, length), self.point(along, 0.0), line),
            _ => segment(self.point(along, 0.0), self.point(along, length), line),
        }
    }

    /// The axis line of `layout`'s panel on this side.
    pub fn line(self, layout: &PlotLayout, line: crate::theme::AxisLine) -> Node {
        axis_line_node(layout.panel, line, self.side, self.base)
    }

    /// Major tick style and length on this side.
    pub fn major_tick(self, theme: &Theme) -> Option<(LineStyle, f64)> {
        if self.side.is_horizontal() {
            theme
                .axis_ticks_x
                .map(|line| (line, theme.axis_ticks_length.0))
        } else {
            theme
                .axis_ticks_y
                .map(|line| (line, theme.axis_ticks_length.1))
        }
    }

    /// Minor tick style and length on this side.
    pub fn minor_tick(self, theme: &Theme) -> Option<(LineStyle, f64)> {
        if self.side.is_horizontal() {
            theme
                .axis_minor_ticks_x
                .map(|line| (line, theme.axis_minor_ticks_length.0))
        } else {
            theme
                .axis_minor_ticks_y
                .map(|line| (line, theme.axis_minor_ticks_length.1))
        }
    }

    /// The pixel across the axis where the labels' band starts (past the ticks
    /// and the text spacing).
    fn label_base(self, theme: &Theme) -> f64 {
        match self.side {
            Side::Bottom => self.base + x_tick_band(theme) + theme.axis_text_spacing.0,
            Side::Top => self.base - x_tick_band(theme) - theme.axis_text_spacing.0,
            Side::Left => self.base - y_tick_band(theme) - theme.axis_text_spacing.1,
            Side::Right => self.base + y_tick_band(theme) + theme.axis_text_spacing.1,
        }
    }
}

/// Places the tick labels of one axis side.
pub(crate) struct LabelPlacer<'a> {
    edge: AxisEdge,
    text: TextStyle,
    arrangement: &'a LabelArrangement,
    /// The pixel the labels' band starts at.
    base: f64,
    /// Placements of the rotated layout, one per label (empty: not rotated).
    rotated: Vec<Placement>,
    /// Label sizes (unrotated), one per label; only kept for mirrored sides.
    sizes: Vec<(f64, f64)>,
}

impl<'a> LabelPlacer<'a> {
    /// The placer of `breaks` on `edge`, or `None` when the theme hides the labels.
    pub fn new(edge: AxisEdge, breaks: &'a AxisBreaks, theme: &Theme) -> Option<Self> {
        let horizontal = edge.side.is_horizontal();
        let text = if horizontal {
            theme.axis_text_x
        } else {
            theme.axis_text_y
        }?;
        let marked = match &breaks.arrangement {
            LabelArrangement::Mark(mark) => Some(mark.link_width),
            _ => None,
        };
        let rotates = if marked.is_some() {
            false
        } else if horizontal {
            breaks.arrangement == LabelArrangement::Rotated
        } else {
            rotated::rotates(text)
        };
        let sizes = if marked.is_none() && (rotates || edge.side.is_secondary()) {
            label_sizes(&breaks.labels, text)
        } else {
            Vec::new()
        };
        let rotated = match (rotates, horizontal) {
            (true, true) => rotated::bottom(&sizes, text).0,
            (true, false) => rotated::left(&sizes, text).0,
            (false, _) => Vec::new(),
        };
        Some(LabelPlacer {
            edge,
            text,
            arrangement: &breaks.arrangement,
            // Marked labels sit past their links.
            base: edge.label_base(theme) + edge.side.outward() * marked.unwrap_or(0.0),
            rotated,
            sizes,
        })
    }

    /// The label of break `index` at `along` (pixel along the axis).
    pub fn node(&self, label: &Label, index: usize, along: f64) -> Node {
        let size = self.sizes.get(index).copied();
        self.place(label, index, along, self.rotated.get(index), size)
    }

    /// A generated label (zoom template) at `along`: the first label's rotation,
    /// or the simple layout.
    pub fn template(&self, label: &Label, along: f64) -> Node {
        let size = self
            .edge
            .side
            .is_secondary()
            .then(|| label_sizes(std::slice::from_ref(label), self.text)[0]);
        let arrangement = match self.arrangement {
            LabelArrangement::Rotated => LabelArrangement::Rotated,
            LabelArrangement::Mark(mark) => LabelArrangement::Mark(*mark),
            _ => LabelArrangement::Simple,
        };
        let placer = LabelPlacer {
            arrangement: &arrangement,
            sizes: Vec::new(),
            rotated: Vec::new(),
            ..*self
        };
        placer.place(label, 0, along, self.rotated.first(), size)
    }

    fn place(
        &self,
        label: &Label,
        index: usize,
        along: f64,
        rotation: Option<&Placement>,
        size: Option<(f64, f64)>,
    ) -> Node {
        let text = self.text;
        if let LabelArrangement::Mark(_) = self.arrangement {
            // Marked labels start at their link: flat on left/right, reading up
            // on top/bottom (ComplexHeatmap `anno_mark`).
            let (anchor, horizontal, angle) = match self.edge.side {
                Side::Bottom => ((along, self.base), HorizontalAlign::End, 90.0),
                Side::Top => ((along, self.base), HorizontalAlign::Start, 90.0),
                Side::Left => ((self.base, along), HorizontalAlign::End, 0.0),
                Side::Right => ((self.base, along), HorizontalAlign::Start, 0.0),
            };
            return aligned(
                label_node(label, text, anchor, horizontal, angle),
                VerticalAlign::Middle,
            );
        }
        // The primary-side node, its labels' band starting at `base`.
        let primary = |base: f64| match (rotation, self.edge.side.is_horizontal()) {
            (Some(placement), true) => placed_label_node(label, text, (along, base), placement),
            (Some(placement), false) => placed_label_node(label, text, (base, along), placement),
            (None, true) => x_label_node(label, text, index, along, base, self.arrangement),
            (None, false) => label_node(label, text, (base, along), HorizontalAlign::End, 0.0),
        };
        match self.edge.side {
            Side::Bottom | Side::Left => primary(self.base),
            Side::Right if rotation.is_none() => {
                label_node(label, text, (self.base, along), HorizontalAlign::Start, 0.0)
            }
            // Tilted labels still end at their tick, turned the other way.
            Side::Top if rotation.is_none() && *self.arrangement == LabelArrangement::Tilted => {
                aligned(
                    label_node(
                        label,
                        text,
                        (along, self.base),
                        HorizontalAlign::End,
                        -TILT_DEGREES,
                    ),
                    VerticalAlign::Bottom,
                )
            }
            // Vertical labels start at their tick, reading up.
            Side::Top if rotation.is_none() && *self.arrangement == LabelArrangement::Vertical => {
                aligned(
                    label_node(
                        label,
                        text,
                        (along, self.base),
                        HorizontalAlign::Start,
                        90.0,
                    ),
                    VerticalAlign::Middle,
                )
            }
            // Everything else keeps its box, mirrored across the axis line.
            Side::Top | Side::Right => {
                let node = primary(0.0);
                let size =
                    size.unwrap_or_else(|| label_sizes(std::slice::from_ref(label), text)[0]);
                let Some((x, y)) = box_centre(&node, size) else {
                    return node;
                };
                let centre = if self.edge.side == Side::Top {
                    (x, self.base - y)
                } else {
                    (self.base - x, y)
                };
                let angle = text_angle(&node);
                aligned(
                    label_node(label, text, centre, HorizontalAlign::Middle, angle),
                    VerticalAlign::Middle,
                )
            }
        }
    }
}

/// `node` with its text's vertical alignment set (single-line labels).
fn aligned(mut node: Node, vertical: VerticalAlign) -> Node {
    if let Node::Text(text) = &mut node {
        text.vertical = vertical;
    }
    node
}

/// The angle of a label node (its first line for wrapped labels).
fn text_angle(node: &Node) -> f64 {
    match node {
        Node::Text(text) => text.angle,
        Node::Group(group) => group.nodes.first().map_or(0.0, text_angle),
        _ => 0.0,
    }
}

/// Centre of the rotated box of a label node of unrotated `size`. Wrapped labels
/// (groups) are centred on their anchor as a block.
fn box_centre(node: &Node, (width, height): (f64, f64)) -> Option<(f64, f64)> {
    let (anchor, horizontal, vertical, angle) = match node {
        Node::Text(text) => ((text.x, text.y), text.horizontal, text.vertical, text.angle),
        Node::Group(group) => {
            // Lines are offset from the block anchor along the text's down direction:
            // their mean is the anchor.
            let lines: Vec<&crate::scene::Text> = group
                .nodes
                .iter()
                .filter_map(|node| match node {
                    Node::Text(text) => Some(text),
                    _ => None,
                })
                .collect();
            let first = lines.first()?;
            let count = lines.len() as f64;
            let x = lines.iter().map(|text| text.x).sum::<f64>() / count;
            let y = lines.iter().map(|text| text.y).sum::<f64>() / count;
            ((x, y), first.horizontal, VerticalAlign::Middle, first.angle)
        }
        _ => return None,
    };
    let along = match horizontal {
        HorizontalAlign::Start => 0.0,
        HorizontalAlign::Middle => 0.5,
        HorizontalAlign::End => 1.0,
    };
    let across = match vertical {
        VerticalAlign::Top => 0.0,
        VerticalAlign::Middle => 0.5,
        VerticalAlign::Baseline | VerticalAlign::Bottom => 1.0,
    };
    // Height of one line for wrapped labels is irrelevant: the block is centred.
    let (dx, dy) = ((0.5 - along) * width, (0.5 - across) * height);
    let (sin, cos) = angle.to_radians().sin_cos();
    Some((
        anchor.0 + dx * cos + dy * sin,
        anchor.1 - dx * sin + dy * cos,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Text;

    #[test]
    fn sides_follow_the_axis_and_position() {
        let both = AxisSides {
            primary: true,
            secondary: true,
        };
        assert_eq!(
            screen_sides(both, true).collect::<Vec<_>>(),
            [Side::Bottom, Side::Top]
        );
        let right = AxisSides {
            primary: false,
            secondary: true,
        };
        assert_eq!(
            screen_sides(right, false).collect::<Vec<_>>(),
            [Side::Right]
        );
        assert!(title_on_secondary(right) && !title_on_secondary(both));
        assert_eq!(Side::Top.outward(), -1.0);
        assert_eq!(Side::Right.outward(), 1.0);
        assert_eq!(Side::Left.outward(), -1.0);
        assert_eq!(Side::Bottom.outward(), 1.0);
    }

    #[test]
    fn a_box_centre_follows_the_alignment_and_angle() {
        let text = |horizontal, vertical, angle| {
            Node::Text(Text {
                content: "a".into(),
                superscript: None,
                suffix: None,
                x: 10.0,
                y: 20.0,
                size: 10.0,
                font: Default::default(),
                color: crate::scene::Color::rgb(0, 0, 0),
                horizontal,
                vertical,
                angle,
                halo: None,
            })
        };
        let centre = |node| box_centre(&node, (40.0, 10.0)).unwrap();
        let close = |(x, y): (f64, f64), (a, b): (f64, f64)| {
            assert!((x - a).abs() < 1e-9 && (y - b).abs() < 1e-9, "{x},{y}")
        };
        close(
            centre(text(HorizontalAlign::Middle, VerticalAlign::Middle, 0.0)),
            (10.0, 20.0),
        );
        close(
            centre(text(HorizontalAlign::End, VerticalAlign::Top, 0.0)),
            (-10.0, 25.0),
        );
        // Reading up, ending at the anchor: the box hangs below it.
        close(
            centre(text(HorizontalAlign::End, VerticalAlign::Middle, 90.0)),
            (10.0, 40.0),
        );
    }
}
