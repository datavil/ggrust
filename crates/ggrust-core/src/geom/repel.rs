//! `geom_text_repel` and `geom_label_repel`: `geom_text` / `geom_label` whose
//! labels are pushed apart (and off the points) by the force layout in
//! [`super::repel_layout`], each joined to its point by a leader segment.
//!
//! Follows lets-plot `TextRepelGeom`: every point inside the panel repels labels
//! with its marker radius (`point_size` 3, `point_stroke`, `shape`) plus
//! `point_padding` x 2.2 px; labels keep `box_padding` x 2.2 px apart; a label
//! starts at its point plus `nudge_x`/`nudge_y`. The segment runs from the
//! marker's edge to the centre of the nearest box edge it can reach, styled by
//! `segment_color` (`color` unless set), `segment_size` x 2.2 px, `segment_alpha`
//! (`alpha` unless set), `linetype` and `arrow`; it is dropped when shorter than
//! `min_segment_length` x 2.2 px or when the point is under the label. Each
//! label's text is drawn before its segment. Options: `seed`, `max_iter` (2000),
//! `max_time` (5 s), `direction` (`both`, `x`, `y`), `max_overlaps` (10).
//!
//! What it does not do (yet): unseeded layouts are not random (the seed defaults
//! to 0, as `position_jitter`), `hjust`/`vjust` `inward`/`outward`, `size_unit`,
//! `label_format`, `na_text`, `check_overlap`, `halo_*`, `lineheight`. Interactive
//! zoom moves each label with its point; it does not lay them out again.

use super::bracket::{UNSET_SEGMENT_COLOR, segment_stroke};
use super::lines::Arrow;
use super::point::{POINT_UNIT_SIZE, shape_radius};
use super::repel_layout::{
    self, Bounds, Circle, Direction, LabelInput, Options, Placement, Quad, Vector,
};
use super::text::{self, LABEL_BORDER, LABEL_PADDING, LABEL_RADIUS};
use super::{DrawLayer, Geom};
use crate::coord::Panel;
use crate::scene::{Color, Group, Node, Path, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};
use crate::tooltip::{HoverTargets, TooltipBehavior};
use crate::{Error, Result};

/// The text aesthetics, then the point and segment ones (lets-plot
/// `GeomMeta.TEXT_REPEL` order).
const TEXT_REPEL_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (Aesthetic::Label, StyleDefault::Text("")),
    (
        Aesthetic::Color,
        StyleDefault::Color(Color::rgb(0x47, 0x47, 0x47)),
    ),
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Size, StyleDefault::Number(7.0)),
    (Aesthetic::Family, StyleDefault::Text("sans")),
    (Aesthetic::Fontface, StyleDefault::Text("plain")),
    (Aesthetic::Angle, StyleDefault::Number(0.0)),
    (Aesthetic::Hjust, StyleDefault::Number(0.5)),
    (Aesthetic::Vjust, StyleDefault::Number(0.5)),
    (Aesthetic::PointSize, StyleDefault::Number(3.0)),
    (Aesthetic::PointStroke, StyleDefault::Number(0.5)),
    (Aesthetic::Shape, StyleDefault::Shape(16)),
    (
        Aesthetic::SegmentColor,
        StyleDefault::Color(UNSET_SEGMENT_COLOR),
    ),
    (Aesthetic::SegmentSize, StyleDefault::Number(0.75)),
    (Aesthetic::SegmentAlpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
];

const LABEL_REPEL_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (Aesthetic::Label, StyleDefault::Text("")),
    (
        Aesthetic::Color,
        StyleDefault::Color(Color::rgb(0x47, 0x47, 0x47)),
    ),
    (Aesthetic::Fill, StyleDefault::Color(Color::WHITE)),
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Size, StyleDefault::Number(7.0)),
    (Aesthetic::Family, StyleDefault::Text("sans")),
    (Aesthetic::Fontface, StyleDefault::Text("plain")),
    (Aesthetic::Angle, StyleDefault::Number(0.0)),
    (Aesthetic::Hjust, StyleDefault::Number(0.5)),
    (Aesthetic::Vjust, StyleDefault::Number(0.5)),
    (Aesthetic::PointSize, StyleDefault::Number(3.0)),
    (Aesthetic::PointStroke, StyleDefault::Number(0.5)),
    (Aesthetic::Shape, StyleDefault::Shape(16)),
    (
        Aesthetic::SegmentColor,
        StyleDefault::Color(UNSET_SEGMENT_COLOR),
    ),
    (Aesthetic::SegmentSize, StyleDefault::Number(0.75)),
    (Aesthetic::SegmentAlpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
];

/// lets-plot `textRepel` defaults in legends: label keys have no box fill.
const LABEL_LEGEND_DEFAULTS: &[(Aesthetic, StyleDefault)] =
    &[(Aesthetic::Fill, StyleDefault::Color(Color::TRANSPARENT))];

const REPEL_PARAMETERS: &[&str] = &[
    "nudge_x",
    "nudge_y",
    "seed",
    "max_iter",
    "max_time",
    "direction",
    "point_padding",
    "box_padding",
    "max_overlaps",
    "min_segment_length",
    "arrow",
];

const LABEL_REPEL_PARAMETERS: &[&str] = &[
    "nudge_x",
    "nudge_y",
    "label_padding",
    "label_r",
    "label_size",
    "seed",
    "max_iter",
    "max_time",
    "direction",
    "point_padding",
    "box_padding",
    "max_overlaps",
    "min_segment_length",
    "arrow",
];

/// Text labels that avoid each other and the points.
pub struct TextRepelGeom;

/// Boxed labels that avoid each other and the points.
pub struct LabelRepelGeom;

impl Geom for TextRepelGeom {
    fn name(&self) -> &'static str {
        "text_repel"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        TEXT_REPEL_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        REPEL_PARAMETERS
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        draw(layer, panel, false)
    }

    /// lets-plot: text layers have tooltips only when `layer_tooltips()` gives lines.
    fn tooltip_behavior(&self) -> TooltipBehavior {
        TooltipBehavior {
            needs_lines: true,
            ..TooltipBehavior::NEAREST_XY
        }
    }

    fn hover_targets(&self, layer: &DrawLayer, panel: &Panel) -> Result<Option<HoverTargets>> {
        hover_targets(layer, panel, false)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        text::key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        draw_key(styles, cell, false)
    }
}

impl Geom for LabelRepelGeom {
    fn name(&self) -> &'static str {
        "label_repel"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        LABEL_REPEL_DEFAULTS
    }

    fn legend_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        LABEL_LEGEND_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        LABEL_REPEL_PARAMETERS
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        draw(layer, panel, true)
    }

    /// lets-plot: text layers have tooltips only when `layer_tooltips()` gives lines.
    fn tooltip_behavior(&self) -> TooltipBehavior {
        TooltipBehavior {
            needs_lines: true,
            ..TooltipBehavior::NEAREST_XY
        }
    }

    fn hover_targets(&self, layer: &DrawLayer, panel: &Panel) -> Result<Option<HoverTargets>> {
        hover_targets(layer, panel, true)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        text::key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        draw_key(styles, cell, true)
    }
}

/// The layer's options besides the layout's own.
struct Settings {
    layout: Options,
    nudge: (f64, f64),
    point_padding: f64,
    box_padding: f64,
    min_segment_length: f64,
    label_padding: f64,
    label_radius: f64,
    label_border: f64,
    arrow: Option<Arrow>,
}

impl Settings {
    fn parse(layer: &DrawLayer) -> Result<Settings> {
        let parameters = &layer.parameters;
        let number = |name: &str, default: f64| -> Result<f64> {
            Ok(parameters.number(name)?.unwrap_or(default))
        };
        let defaults = Options::default();
        let direction = match parameters.string("direction")? {
            None => defaults.direction,
            Some(name) => Direction::from_name(name).ok_or_else(|| {
                Error::InvalidSpec(format!(
                    "at '{}': unknown direction '{name}'; expected 'both', 'x' or 'y'",
                    parameters.path_of("direction")
                ))
            })?,
        };
        Ok(Settings {
            layout: Options {
                max_overlaps: number("max_overlaps", defaults.max_overlaps as f64)? as i64,
                seed: number("seed", defaults.seed as f64)? as i64,
                // lets-plot reads an Int.
                max_iter: (number("max_iter", defaults.max_iter as f64)? as i64)
                    .min(i64::from(i32::MAX)),
                // lets-plot takes seconds.
                max_time: parameters
                    .number("max_time")?
                    .map_or(defaults.max_time, |seconds| seconds * 1000.0),
                direction,
            },
            nudge: (number("nudge_x", 0.0)?, number("nudge_y", 0.0)?),
            point_padding: number("point_padding", 0.0)? * POINT_UNIT_SIZE,
            box_padding: number("box_padding", 0.0)? * POINT_UNIT_SIZE,
            min_segment_length: number("min_segment_length", 0.0)? * POINT_UNIT_SIZE,
            label_padding: number("label_padding", LABEL_PADDING)?,
            label_radius: number("label_r", LABEL_RADIUS)?,
            label_border: number("label_size", LABEL_BORDER)?,
            arrow: Arrow::parse(parameters)?,
        })
    }
}

/// A placed label with what drawing it needs.
struct Placed {
    placement: Placement,
    look: text::Look,
    point: Vector,
    marker_radius: f64,
}

/// Lay out the layer's labels (lets-plot `TextRepelGeom.buildIntern`).
fn place(
    layer: &DrawLayer,
    panel: &Panel,
    label: bool,
    settings: &Settings,
) -> Result<Vec<Placed>> {
    let (Some(x), Some(y)) = (
        layer.data.continuous(Aesthetic::X),
        layer.data.continuous(Aesthetic::Y),
    ) else {
        return Ok(Vec::new());
    };
    let styles = layer.styles;
    let bounds = Bounds {
        left: panel.rect.x,
        top: panel.rect.y,
        right: panel.rect.x + panel.rect.width,
        bottom: panel.rect.y + panel.rect.height,
    };
    let mut inputs = Vec::new();
    let mut points = Vec::new();
    let mut looks = Vec::new();
    for row in 0..layer.data.row_count() {
        let (Some(value_x), Some(value_y)) = (x.get(row), y.get(row)) else {
            continue;
        };
        let (start_x, start_y) = panel.map(value_x + settings.nudge.0, value_y + settings.nudge.1);
        let point = {
            let (pixel_x, pixel_y) = panel.map(value_x, value_y);
            Vector::new(pixel_x, pixel_y)
        };
        if !(start_x.is_finite() && start_y.is_finite() && bounds.contains(point)) {
            continue;
        }
        let marker_radius = shape_radius(
            styles.shapes(Aesthetic::Shape)?.get(row),
            styles.numbers(Aesthetic::PointSize)?.get(row),
            styles.numbers(Aesthetic::PointStroke)?.get(row),
        );
        let circle = Circle {
            center: point,
            radius: marker_radius + settings.point_padding,
        };
        points.push((row, circle));
        let mut look = text::look(styles, row, label)?;
        // lets-plot `TextUtil.hAnchor`: hjust other than 0, 0.5 or 1 is the middle.
        if ![0.0, 0.5, 1.0].contains(&look.hjust) {
            look.hjust = 0.5;
        }
        if look.content.is_empty() || look.size.is_nan() || look.size <= 0.0 {
            continue;
        }
        let (text_width, text_height) = text::block_size(&look);
        let padding = if label {
            settings.label_padding * look.size
        } else {
            0.0
        };
        let (width, height) = (text_width + 2.0 * padding, text_height + 2.0 * padding);
        // lets-plot `TextUtil.rectangleForText`, turned by the SVG angle.
        let start = Vector::new(start_x, start_y);
        let svg_angle = if look.angle == 0.0 {
            0.0
        } else {
            360.0 - look.angle % 360.0
        };
        let quad = Quad::rotated(
            start_x - look.hjust * width,
            start_y + (look.vjust - 1.0) * height,
            width,
            height,
            svg_angle.to_radians(),
            start,
        );
        inputs.push(LabelInput {
            row,
            quad,
            padding: settings.box_padding,
            hjust: look.hjust,
            vjust: look.vjust,
            point: circle,
        });
        looks.push((look, point, marker_radius));
    }
    let placements = repel_layout::layout(&inputs, &points, bounds, &settings.layout);
    Ok(placements
        .into_iter()
        .zip(looks)
        .filter(|(placement, _)| !placement.hidden)
        .map(|(placement, (look, point, marker_radius))| Placed {
            placement,
            look,
            point,
            marker_radius,
        })
        .collect())
}

fn draw(layer: &DrawLayer, panel: &Panel, label: bool) -> Result<Vec<Node>> {
    let settings = Settings::parse(layer)?;
    let mut nodes = Vec::new();
    for placed in place(layer, panel, label, &settings)? {
        let position = placed.placement.position;
        let mut group = text::draw_one(
            &placed.look,
            (position.x, position.y),
            label,
            settings.label_border,
            settings.label_padding,
            settings.label_radius,
        );
        group.extend(segment(layer.styles, &placed, &settings)?);
        // Anchored at the point, so interactive zoom moves the label with it.
        nodes.push(Node::Group(Group {
            clip: None,
            role: None,
            anchor: Some((placed.point.x, placed.point.y)),
            nodes: group,
        }));
    }
    Ok(nodes)
}

/// The leader line from the marker's edge to the label box (lets-plot
/// `getSegmentLocation`), with its arrow heads.
fn segment(styles: &Styles, placed: &Placed, settings: &Settings) -> Result<Vec<Node>> {
    let Some(edge) = placed.placement.quad.segment_to_edge_center(placed.point) else {
        return Ok(Vec::new());
    };
    let end = edge.end;
    let (dx, dy) = (end.x - placed.point.x, end.y - placed.point.y);
    let length = (dx * dx + dy * dy).sqrt();
    let unit = if length == 0.0 {
        (0.0, 0.0)
    } else {
        (dx / length, dy / length)
    };
    let start = (
        placed.point.x + unit.0 * placed.marker_radius,
        placed.point.y + unit.1 * placed.marker_radius,
    );
    let (gap_x, gap_y) = (start.0 - end.x, start.1 - end.y);
    if (gap_x * gap_x + gap_y * gap_y).sqrt() < settings.min_segment_length {
        return Ok(Vec::new());
    }
    let Some(stroke) = segment_stroke(styles, placed.placement.row)? else {
        return Ok(Vec::new());
    };
    let mut points = vec![start, (end.x, end.y)];
    let heads = settings
        .arrow
        .as_ref()
        .map(|arrow| arrow.apply(&mut points, &stroke))
        .unwrap_or_default();
    let mut nodes = vec![Node::Path(Path {
        points,
        closed: false,
        fill: None,
        stroke: Some(stroke),
    })];
    nodes.extend(heads);
    Ok(nodes)
}

/// One circle per drawn label at its placed anchor, half its font size wide.
fn hover_targets(layer: &DrawLayer, panel: &Panel, label: bool) -> Result<Option<HoverTargets>> {
    let settings = Settings::parse(layer)?;
    let (mut x, mut y, mut radius, mut rows) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for placed in place(layer, panel, label, &settings)? {
        x.push(placed.placement.position.x);
        y.push(placed.placement.position.y);
        radius.push(placed.look.size / 2.0);
        rows.push(placed.placement.row as u32);
    }
    Ok(Some(HoverTargets::Points { x, y, radius, rows }))
}

/// lets-plot `CompositeLegendKeyElementFactory` (both repel geoms): the cell
/// filled with `fill` (labels; transparent unless mapped), the text key, then a
/// horizontal line in the segment style.
fn draw_key(styles: &Styles, cell: Rect, label: bool) -> Result<Vec<Node>> {
    let mut nodes = Vec::new();
    if label {
        let alpha = styles.numbers(Aesthetic::Alpha)?.get(0);
        let fill = styles.colors(Aesthetic::Fill)?.get(0).with_opacity(alpha);
        if fill.alpha > 0 {
            let (right, bottom) = (cell.x + cell.width, cell.y + cell.height);
            nodes.push(Node::Path(Path {
                points: vec![
                    (cell.x, cell.y),
                    (right, cell.y),
                    (right, bottom),
                    (cell.x, bottom),
                ],
                closed: true,
                fill: Some(fill),
                stroke: None,
            }));
        }
    }
    nodes.extend(text::draw_key(styles, cell, false)?);
    if let Some(stroke) = segment_stroke(styles, 0)? {
        let y = cell.y + cell.height / 2.0;
        nodes.push(Node::Path(Path {
            points: vec![(cell.x, y), (cell.x + cell.width, y)],
            closed: false,
            fill: None,
            stroke: Some(stroke),
        }));
    }
    Ok(nodes)
}
