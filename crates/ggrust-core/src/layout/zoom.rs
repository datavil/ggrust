//! What the HTML runtime needs to zoom and pan a panel (`ggtb`, plan.md 4.3).
//!
//! What it does: for each panel, both screen axes' domains (transformed units at
//! two pixels), their break rules (the scale's transform and target break count,
//! or the fixed breaks with their drawn labels) and templates of the grid lines,
//! ticks and labels, drawn at pixel 0 exactly as the static axes draw them. The
//! runtime computes breaks for a zoomed domain (lets-plot's linear breaks and
//! default number format, ported) and stamps translated copies of the templates.
//!
//! What it does not do: redo the layout. The panel keeps its size and place while
//! zoomed (lets-plot re-lays out, so its axis bands can change width), and a
//! user `format` on generated breaks falls back to the default number format.

use super::rotated;
use super::{
    LabelArrangement, PlotLayout, label_node, label_sizes, placed_label_node, segment, tick_band,
    x_label_node,
};
use crate::coord::Panel;
use crate::scale::format::Label;
use crate::scale::{PositionScale, Transform};
use crate::scene::{HorizontalAlign, Node, Rect};
use crate::spec::ToolbarSpec;
use crate::theme::Theme;

/// How point sizes follow the zoom (lets-plot `ggtb(size_basis=...)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SizeBasis {
    /// The x zoom factor.
    X,
    /// The y zoom factor.
    Y,
    /// The smaller of both.
    Min,
    /// The larger of both (lets-plot's default).
    #[default]
    Max,
}

impl SizeBasis {
    /// Parse a `size_basis` value; `path` locates it in error messages.
    pub fn parse(name: &str, path: &str) -> crate::Result<Self> {
        match name {
            "x" => Ok(SizeBasis::X),
            "y" => Ok(SizeBasis::Y),
            "min" => Ok(SizeBasis::Min),
            "max" => Ok(SizeBasis::Max),
            other => Err(crate::Error::InvalidSpec(format!(
                "at '{path}': unknown size_basis '{other}'; supported: 'x', 'y', 'min', 'max'"
            ))),
        }
    }

    /// The name the runtime reads.
    pub fn as_str(self) -> &'static str {
        match self {
            SizeBasis::X => "x",
            SizeBasis::Y => "y",
            SizeBasis::Min => "min",
            SizeBasis::Max => "max",
        }
    }
}

/// The runtime's zoom payload for a whole figure.
#[derive(Debug, Clone, PartialEq)]
pub struct Zoom {
    /// Which zoom factor point sizes follow.
    pub size_basis: SizeBasis,
    /// Largest point growth: 0 keeps sizes, -1 is unlimited (lets-plot `size_zoomin`).
    pub size_zoomin: f64,
    /// Every plot of the figure (one, or the plots of a grid), in drawing order.
    pub plots: Vec<ZoomPlot>,
}

/// One plot: its panels zoom together on the axes they share.
#[derive(Debug, Clone, PartialEq)]
pub struct ZoomPlot {
    /// One per panel, in drawing order (the order of their `tile` groups).
    pub tiles: Vec<ZoomTile>,
    /// Whether every panel has the same horizontal / vertical scale (facets with
    /// fixed scales): zooming one panel then zooms all of them on that axis.
    pub shared: (bool, bool),
}

/// One panel.
#[derive(Debug, Clone, PartialEq)]
pub struct ZoomTile {
    /// The panel in scene pixels.
    pub panel: Rect,
    /// The axis drawn horizontally.
    pub horizontal: ZoomAxis,
    /// The axis drawn vertically.
    pub vertical: ZoomAxis,
}

/// One screen axis of a panel.
#[derive(Debug, Clone, PartialEq)]
pub struct ZoomAxis {
    /// The domain in transformed units: `domain.0` sits at `pixels.0`, `domain.1`
    /// at `pixels.1`.
    pub domain: (f64, f64),
    /// Pixels of the domain ends (x for the horizontal axis, y for the vertical).
    pub pixels: (f64, f64),
    /// The scale's transform; identity for discrete axes.
    pub transform: Transform,
    /// The break count asked of the scale for generated breaks.
    pub count: usize,
    /// Fixed breaks (categories, `breaks=[...]`): position, the pixel it was drawn
    /// at, and its label as drawn there. `None`: breaks are generated.
    pub fixed: Option<Vec<FixedBreak>>,
    /// Templates at pixel 0 (`None`: not drawn).
    pub major: Option<Node>,
    /// Minor grid line template.
    pub minor: Option<Node>,
    /// Tick template.
    pub tick: Option<Node>,
    /// Generated label template: text `@`.
    pub label: Option<Node>,
    /// Generated label template with a superscript: text `@`, superscript `^`.
    pub label_superscript: Option<Node>,
    /// The axis line, drawn unchanged.
    pub line: Option<Node>,
}

/// A fixed break.
#[derive(Debug, Clone, PartialEq)]
pub struct FixedBreak {
    /// Position in transformed units (category index on discrete axes).
    pub position: f64,
    /// The pixel it is drawn at without zoom.
    pub pixel: f64,
    /// Its label as drawn there.
    pub label: Option<Node>,
}

impl Zoom {
    /// The empty payload of `ggtb(...)` (`toolbar`, at spec path `path`), or `None`
    /// without a toolbar. Bad options are errors, as in lets-plot.
    pub fn from_spec(toolbar: Option<&ToolbarSpec>, path: &str) -> crate::Result<Option<Zoom>> {
        let Some(toolbar) = toolbar else {
            return Ok(None);
        };
        let size_basis = match &toolbar.size_basis {
            Some(name) => SizeBasis::parse(name, &format!("{path}.size_basis"))?,
            None => SizeBasis::default(),
        };
        let size_zoomin = toolbar.size_zoomin.unwrap_or(0.0);
        if !(size_zoomin.is_finite() && (size_zoomin >= 0.0 || size_zoomin == -1.0)) {
            return Err(crate::Error::InvalidSpec(format!(
                "at '{path}.size_zoomin': {size_zoomin} is not accepted; use a number >= 0 \
                 (largest point growth, 0 keeps sizes) or -1 (no limit)"
            )));
        }
        Ok(Some(Zoom {
            size_basis,
            size_zoomin,
            plots: Vec::new(),
        }))
    }
}

/// Placeholder text of a generated label template.
pub const LABEL_PLACEHOLDER: &str = "@";
/// Placeholder superscript of a generated label template.
pub const SUPERSCRIPT_PLACEHOLDER: &str = "^";

/// The zoom description of one panel as `draw_axes` draws it: `layout` is the
/// panel's layout, `mapper` its screen scales, `shown` whether its x / y axes are
/// drawn.
pub fn zoom_tile(
    layout: &PlotLayout,
    theme: &Theme,
    mapper: &Panel,
    (show_x, show_y): (bool, bool),
) -> ZoomTile {
    let panel = layout.panel;
    let bottom = panel.y + panel.height;
    let domain = |scale: &PositionScale| scale.expanded_domain();
    let transform = |scale: &PositionScale| match scale {
        PositionScale::Continuous(scale) => scale.transform,
        PositionScale::Discrete(_) => Transform::Identity,
    };

    // Horizontal axis.
    let x_domain = domain(mapper.x_scale);
    let x_label_top = bottom + tick_band(theme.axis_ticks_x, theme) + theme.axis_text_spacing;
    let x_breaks = &layout.x_breaks;
    let x_rotated = match (theme.axis_text_x, &x_breaks.arrangement) {
        (Some(text), LabelArrangement::Rotated) => {
            rotated::bottom(&label_sizes(&x_breaks.labels, text), text).0
        }
        _ => Vec::new(),
    };
    let x_label = |label: &Label, index: usize, x: f64| {
        theme.axis_text_x.map(|text| match x_rotated.get(index) {
            Some(placement) => placed_label_node(label, text, (x, x_label_top), placement),
            None => x_label_node(label, text, index, x, x_label_top, &x_breaks.arrangement),
        })
    };
    // Generated labels take the first label's rotation, or the simple layout.
    let x_template = |label: &Label| {
        theme.axis_text_x.map(|text| match x_rotated.first() {
            Some(placement) => placed_label_node(label, text, (0.0, x_label_top), placement),
            None => x_label_node(label, text, 0, 0.0, x_label_top, &LabelArrangement::Simple),
        })
    };
    let horizontal = ZoomAxis {
        domain: x_domain,
        pixels: (mapper.x_pixel(x_domain.0), mapper.x_pixel(x_domain.1)),
        transform: transform(mapper.x_scale),
        count: x_breaks.target_count,
        fixed: mapper.x_scale.has_fixed_breaks().then(|| {
            x_breaks
                .values
                .iter()
                .zip(&x_breaks.labels)
                .enumerate()
                .map(|(index, (&value, label))| {
                    let pixel = mapper.x_pixel(value);
                    FixedBreak {
                        position: value,
                        pixel,
                        label: show_x.then(|| x_label(label, index, pixel)).flatten(),
                    }
                })
                .collect()
        }),
        major: theme
            .panel_grid_major_x
            .map(|line| segment((0.0, panel.y), (0.0, bottom), line)),
        minor: theme
            .panel_grid_minor_x
            .map(|line| segment((0.0, panel.y), (0.0, bottom), line)),
        tick: theme
            .axis_ticks_x
            .filter(|_| show_x)
            .map(|tick| segment((0.0, bottom), (0.0, bottom + theme.axis_ticks_length), tick)),
        label: show_x.then(|| x_template(&placeholder(false))).flatten(),
        label_superscript: show_x.then(|| x_template(&placeholder(true))).flatten(),
        line: theme
            .axis_line_x
            .filter(|_| show_x)
            .map(|line| segment((panel.x, bottom), (panel.x + panel.width, bottom), line)),
    };

    // Vertical axis.
    let y_domain = domain(mapper.y_scale);
    let y_label_right = panel.x - tick_band(theme.axis_ticks_y, theme) - theme.axis_text_spacing;
    let y_breaks = &layout.y_breaks;
    let y_rotated = match theme.axis_text_y {
        Some(text) if rotated::rotates(text) => {
            rotated::left(&label_sizes(&y_breaks.labels, text), text).0
        }
        _ => Vec::new(),
    };
    let y_label = |label: &Label, placement: Option<&rotated::Placement>, y: f64| {
        theme.axis_text_y.map(|text| match placement {
            Some(placement) => placed_label_node(label, text, (y_label_right, y), placement),
            None => label_node(label, text, (y_label_right, y), HorizontalAlign::End, 0.0),
        })
    };
    let vertical = ZoomAxis {
        domain: y_domain,
        pixels: (mapper.y_pixel(y_domain.0), mapper.y_pixel(y_domain.1)),
        transform: transform(mapper.y_scale),
        count: y_breaks.target_count,
        fixed: mapper.y_scale.has_fixed_breaks().then(|| {
            y_breaks
                .values
                .iter()
                .zip(&y_breaks.labels)
                .enumerate()
                .map(|(index, (&value, label))| {
                    let pixel = mapper.y_pixel(value);
                    FixedBreak {
                        position: value,
                        pixel,
                        label: show_y
                            .then(|| y_label(label, y_rotated.get(index), pixel))
                            .flatten(),
                    }
                })
                .collect()
        }),
        major: theme
            .panel_grid_major_y
            .map(|line| segment((panel.x, 0.0), (panel.x + panel.width, 0.0), line)),
        minor: theme
            .panel_grid_minor_y
            .map(|line| segment((panel.x, 0.0), (panel.x + panel.width, 0.0), line)),
        tick: theme.axis_ticks_y.filter(|_| show_y).map(|tick| {
            segment(
                (panel.x - theme.axis_ticks_length, 0.0),
                (panel.x, 0.0),
                tick,
            )
        }),
        label: show_y
            .then(|| y_label(&placeholder(false), y_rotated.first(), 0.0))
            .flatten(),
        label_superscript: show_y
            .then(|| y_label(&placeholder(true), y_rotated.first(), 0.0))
            .flatten(),
        line: theme
            .axis_line_y
            .filter(|_| show_y)
            .map(|line| segment((panel.x, panel.y), (panel.x, bottom), line)),
    };
    ZoomTile {
        panel,
        horizontal,
        vertical,
    }
}

fn placeholder(superscript: bool) -> Label {
    Label {
        text: LABEL_PLACEHOLDER.into(),
        superscript: superscript.then(|| SUPERSCRIPT_PLACEHOLDER.into()),
        suffix: None,
    }
}
