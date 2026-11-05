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
//! A secondary axis (`sec_axis=`) keeps its breaks: they move with the zoom but no
//! new ones are generated.

use super::side::{AxisEdge, LabelPlacer, screen_sides};
use super::{AxisBreaks, PlotLayout, segment};
use crate::coord::Panel;
use crate::scale::format::Label;
use crate::scale::number_format::ExponentFormat;
use crate::scale::{AxisSides, PositionScale, Transform};
use crate::scene::{Group, Node, Rect};
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
            other => Err(crate::Error::spec(
                path,
                format!("unknown size_basis '{other}'; supported: 'x', 'y', 'min', 'max'"),
            )),
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
    /// Layer visibility toggles (`ggtb(layers=...)`): `None` without them,
    /// else the user's labels as (label, layer index), possibly empty.
    pub layer_labels: Option<Vec<(String, usize)>>,
}

/// One plot: its panels zoom together on the axes they share.
#[derive(Debug, Clone, PartialEq)]
pub struct ZoomPlot {
    /// One per panel, in drawing order (the order of their `tile` groups).
    pub tiles: Vec<ZoomTile>,
    /// Whether every panel has the same horizontal / vertical scale (facets with
    /// fixed scales): zooming one panel then zooms all of them on that axis.
    pub shared: (bool, bool),
    /// Whether the panels keep a fixed aspect ratio (`coord_fixed`): box zooms
    /// then widen the selection to the panel's shape so both axes zoom equally.
    pub fixed_aspect: bool,
    /// The geom of each layer, in layer order (the default toggle labels).
    pub geoms: Vec<&'static str>,
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
    /// Secondary axes (`sec_axis=`) of the horizontal and vertical axes: their
    /// breaks stay fixed while zoomed (ticks and labels only, no grid lines).
    pub secondary: [Option<ZoomAxis>; 2],
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
    /// The theme's `exponent_format` for generated labels.
    pub exponent: ExponentFormat,
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
    /// Minor tick template (between breaks, as the minor grid lines).
    pub minor_tick: Option<Node>,
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
            return Err(crate::Error::spec(
                format!("{path}.size_zoomin"),
                format!(
                    "{size_zoomin} is not accepted; use a number >= 0 \
                 (largest point growth, 0 keeps sizes) or -1 (no limit)"
                ),
            ));
        }
        Ok(Some(Zoom {
            size_basis,
            size_zoomin,
            plots: Vec::new(),
            layer_labels: parse_layer_labels(toolbar.layers.as_ref(), &format!("{path}.layers"))?,
        }))
    }

    /// The toggle label of every layer of `plot`, or `None` without toggles:
    /// the user's label, else the geom name, numbered from its second unlabelled
    /// use (`point`, `point 2`). `geom_imshow` layers (geom `image`) read `imshow`.
    pub fn layer_toggle_labels(&self, plot: &ZoomPlot) -> Option<Vec<String>> {
        let named = self.layer_labels.as_ref()?;
        let mut seen: Vec<(&str, usize)> = Vec::new();
        let labels = plot
            .geoms
            .iter()
            .enumerate()
            .map(|(index, geom)| {
                if let Some((label, _)) = named.iter().find(|(_, layer)| *layer == index) {
                    return label.clone();
                }
                let geom = if *geom == "image" { "imshow" } else { geom };
                let count = match seen.iter_mut().find(|(name, _)| *name == geom) {
                    Some((_, count)) => {
                        *count += 1;
                        *count
                    }
                    None => {
                        seen.push((geom, 1));
                        1
                    }
                };
                match count {
                    1 => geom.to_string(),
                    _ => format!("{geom} {count}"),
                }
            })
            .collect();
        Some(labels)
    }

    /// Check, once every plot is collected, that each labelled layer index
    /// exists in at least one plot; `path` locates `ggtb` in error messages.
    pub fn check_layer_labels(&self, path: &str) -> crate::Result<()> {
        let Some(named) = &self.layer_labels else {
            return Ok(());
        };
        let count = self
            .plots
            .iter()
            .map(|plot| plot.geoms.len())
            .max()
            .unwrap_or(0);
        match named.iter().find(|(_, layer)| *layer >= count) {
            Some((label, layer)) => Err(crate::Error::spec(
                format!("{path}.layers.{label}"),
                format!(
                    "layer index {layer} does not exist; the plot has {count} layer(s), \
                     indexed from 0 in the order they were added"
                ),
            )),
            None => Ok(()),
        }
    }
}

/// `ggtb(layers=...)`: `true` gives toggles with default labels, `false` or
/// nothing none, an object maps labels to layer indices.
fn parse_layer_labels(
    value: Option<&crate::spec::Value>,
    path: &str,
) -> crate::Result<Option<Vec<(String, usize)>>> {
    use crate::spec::Value;
    let accepted = "use true, false, or a dict of labels to layer indices such as {\"spots\": 1}";
    match value {
        None | Some(Value::Null) | Some(Value::Bool(false)) => Ok(None),
        Some(Value::Bool(true)) => Ok(Some(Vec::new())),
        Some(Value::Object(entries)) => {
            let mut labels: Vec<(String, usize)> = Vec::new();
            for (label, index) in entries {
                let Some(layer) = index.as_u64().and_then(|value| usize::try_from(value).ok())
                else {
                    return Err(crate::Error::spec(
                        format!("{path}.{label}"),
                        format!("{index} is not a layer index; use an integer >= 0"),
                    ));
                };
                if let Some((other, _)) = labels.iter().find(|(_, existing)| *existing == layer) {
                    return Err(crate::Error::spec(
                        format!("{path}.{label}"),
                        format!(
                            "layer {layer} is already labelled '{other}'; give each layer one label"
                        ),
                    ));
                }
                labels.push((label.clone(), layer));
            }
            Ok(Some(labels))
        }
        Some(other) => Err(crate::Error::spec(
            path,
            format!("{other} is not accepted; {accepted}"),
        )),
    }
}

/// Placeholder text of a generated label template.
pub const LABEL_PLACEHOLDER: &str = "@";
/// Placeholder superscript of a generated label template.
pub const SUPERSCRIPT_PLACEHOLDER: &str = "^";

/// The zoom description of one panel as `draw_axes` draws it: `layout` is the
/// panel's layout, `mapper` its screen scales, `shown` the sides its horizontal /
/// vertical axes are drawn on. An axis drawn on two sides has templates holding
/// both sides' nodes (the runtime translates them along the axis as one).
pub fn zoom_tile(
    layout: &PlotLayout,
    theme: &Theme,
    mapper: &Panel,
    shown: [AxisSides; 2],
) -> ZoomTile {
    let panel = layout.panel;
    // A secondary axis's side is drawn by its own (fixed) zoom axis.
    let split = |index: usize| match layout.secondary_far_side(index) {
        Some(far) => {
            let only = |keep_far: bool| AxisSides {
                primary: shown[index].primary && !keep_far,
                secondary: shown[index].secondary && keep_far,
            };
            (only(!far), Some(only(far)))
        }
        None => (shown[index], None),
    };
    let (x_primary, x_secondary) = split(0);
    let (y_primary, y_secondary) = split(1);
    let primary_shown = [x_primary, y_primary];
    let bottom = panel.y + panel.height;
    let horizontal = zoom_axis(
        layout,
        theme,
        mapper.x_scale,
        &layout.x_breaks,
        primary_shown[0],
        true,
        false,
        &|value| mapper.x_pixel(value),
        (
            theme
                .panel_grid_major_x
                .map(|line| segment((0.0, panel.y), (0.0, bottom), line)),
            theme
                .panel_grid_minor_x
                .map(|line| segment((0.0, panel.y), (0.0, bottom), line)),
        ),
    );
    let vertical = zoom_axis(
        layout,
        theme,
        mapper.y_scale,
        &layout.y_breaks,
        primary_shown[1],
        false,
        false,
        &|value| mapper.y_pixel(value),
        (
            theme
                .panel_grid_major_y
                .map(|line| segment((panel.x, 0.0), (panel.x + panel.width, 0.0), line)),
            theme
                .panel_grid_minor_y
                .map(|line| segment((panel.x, 0.0), (panel.x + panel.width, 0.0), line)),
        ),
    );
    let secondary_axis = |index: usize, sides: Option<AxisSides>| {
        let sides = sides.filter(|sides| sides.primary || sides.secondary)?;
        let breaks = layout.secondary[index].as_ref().map(|side| &side.breaks)?;
        let (scale, pixel): (&PositionScale, &dyn Fn(f64) -> f64) = if index == 0 {
            (mapper.x_scale, &|value| mapper.x_pixel(value))
        } else {
            (mapper.y_scale, &|value| mapper.y_pixel(value))
        };
        Some(zoom_axis(
            layout,
            theme,
            scale,
            breaks,
            sides,
            index == 0,
            true,
            pixel,
            (None, None),
        ))
    };
    ZoomTile {
        panel,
        horizontal,
        vertical,
        secondary: [
            secondary_axis(0, x_secondary),
            secondary_axis(1, y_secondary),
        ],
    }
}

/// One screen axis of [`zoom_tile`]: `grid` holds the major and minor grid line
/// templates; `fixed_breaks` keeps `breaks` fixed (secondary axes).
#[allow(clippy::too_many_arguments)]
fn zoom_axis(
    layout: &PlotLayout,
    theme: &Theme,
    scale: &PositionScale,
    breaks: &AxisBreaks,
    sides: AxisSides,
    horizontal: bool,
    fixed_breaks: bool,
    pixel: &dyn Fn(f64) -> f64,
    (major, minor): (Option<Node>, Option<Node>),
) -> ZoomAxis {
    let domain = scale.expanded_domain();
    let transform = match scale {
        PositionScale::Continuous(scale) => scale.transform,
        PositionScale::Discrete(_) => Transform::Identity,
    };
    let edges: Vec<AxisEdge> = screen_sides(sides, horizontal)
        .map(|side| AxisEdge::of(layout, side))
        .collect();
    let placers: Vec<LabelPlacer> = edges
        .iter()
        .filter_map(|edge| LabelPlacer::new(*edge, breaks, theme))
        .collect();
    let tick = |minor: bool| {
        joined(
            edges
                .iter()
                .filter_map(|edge| {
                    let tick = if minor {
                        edge.minor_tick(theme)
                    } else {
                        edge.major_tick(theme)
                    };
                    tick.map(|(line, length)| edge.tick(0.0, length, line))
                })
                .collect(),
        )
    };
    let template = |superscript: bool| {
        joined(
            placers
                .iter()
                .map(|placer| placer.template(&placeholder(superscript), 0.0))
                .collect(),
        )
    };
    let line = if horizontal {
        theme.axis_line_x
    } else {
        theme.axis_line_y
    };
    ZoomAxis {
        domain,
        pixels: (pixel(domain.0), pixel(domain.1)),
        transform,
        exponent: theme.exponent_format,
        count: breaks.target_count,
        // The HTML runtime generates number breaks only: date axes keep theirs.
        fixed: (fixed_breaks || scale.has_fixed_breaks() || scale.is_temporal()).then(|| {
            breaks
                .values
                .iter()
                .zip(&breaks.labels)
                .enumerate()
                .map(|(index, (&value, label))| {
                    let pixel = pixel(value);
                    FixedBreak {
                        position: value,
                        pixel,
                        label: joined(
                            placers
                                .iter()
                                .map(|placer| placer.node(label, index, pixel))
                                .collect(),
                        ),
                    }
                })
                .collect()
        }),
        major,
        minor,
        tick: tick(false),
        minor_tick: tick(true),
        label: template(false),
        label_superscript: template(true),
        line: line
            .and_then(|line| joined(edges.iter().map(|edge| edge.line(layout, line)).collect())),
    }
}

/// One node, a group of several, or none.
fn joined(mut nodes: Vec<Node>) -> Option<Node> {
    match nodes.len() {
        0 => None,
        1 => nodes.pop(),
        _ => Some(Node::Group(Group {
            clip: None,
            role: None,
            anchor: None,
            nodes,
        })),
    }
}

fn placeholder(superscript: bool) -> Label {
    Label {
        text: LABEL_PLACEHOLDER.into(),
        superscript: superscript.then(|| SUPERSCRIPT_PLACEHOLDER.into()),
        suffix: None,
    }
}
