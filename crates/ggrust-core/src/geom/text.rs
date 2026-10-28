//! `geom_text` and `geom_label`: one text per row at (`x`, `y`); the label also
//! draws a rounded box behind it.
//!
//! Follows lets-plot: font px = `size` x 2 (default size 7 gives 14 px), the
//! label box pads the text by `label_padding` (0.25) x font size, has corner radius
//! `label_r` (0.15) x box height and a `label_size` (1.0) px border in `color`, filled
//! with `fill`. `hjust`/`vjust` 0..1 place the text relative to the point, `angle`
//! turns it counter-clockwise, `nudge_x`/`nudge_y` shift it in data units.
//!
//! What it does not do (yet): `label_format`, `na_text`, `check_overlap`,
//! `halo_*`, `lineheight` (lines are one font size apart), `size_unit`,
//! `hjust`/`vjust` `inward`/`outward`.

use super::{DrawLayer, Geom};
use crate::Result;
use crate::coord::Panel;
use crate::scale::format::Label;
use crate::scale::label_pattern::power_label;
use crate::scene::{Color, HorizontalAlign, Node, Path, Rect, Stroke, Text, VerticalAlign};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};
use crate::text::{self, Font, FontFace, FontFamily};

/// lets-plot `AesScaling.textSize`: pixels of font size per unit of `size`.
const TEXT_UNIT_SIZE: f64 = 2.0;
const LABEL_PADDING: f64 = 0.25;
const LABEL_RADIUS: f64 = 0.15;
const LABEL_BORDER: f64 = 1.0;
/// Segments per rounded corner.
const CORNER_STEPS: usize = 6;
/// lets-plot moves the baseline 0.7 em from `vjust = 0` to `vjust = 1`.
const VJUST_SPAN: f64 = 0.7;

const TEXT_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
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
];

const LABEL_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
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
];

const PARAMETERS: &[&str] = &[
    "nudge_x",
    "nudge_y",
    "label_padding",
    "label_r",
    "label_size",
];

/// Plain text.
pub struct TextGeom;

/// Text in a rounded box.
pub struct LabelGeom;

/// Reach of a text or label hover target around its anchor, in pixels.
const TEXT_HOVER_RADIUS: f64 = 4.0;

impl Geom for TextGeom {
    /// lets-plot: text layers have tooltips only when `layer_tooltips()` gives lines.
    fn tooltip_behavior(&self) -> crate::tooltip::TooltipBehavior {
        crate::tooltip::TooltipBehavior {
            needs_lines: true,
            ..crate::tooltip::TooltipBehavior::NEAREST_XY
        }
    }

    fn hover_targets(
        &self,
        layer: &DrawLayer,
        panel: &Panel,
    ) -> Result<Option<crate::tooltip::HoverTargets>> {
        Ok(super::hover::circles(layer, panel, |_| TEXT_HOVER_RADIUS))
    }

    fn name(&self) -> &'static str {
        "text"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        TEXT_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["nudge_x", "nudge_y"]
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        draw_texts(layer, panel, false)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        draw_key(styles, cell, false)
    }
}

impl Geom for LabelGeom {
    /// lets-plot: text layers have tooltips only when `layer_tooltips()` gives lines.
    fn tooltip_behavior(&self) -> crate::tooltip::TooltipBehavior {
        crate::tooltip::TooltipBehavior {
            needs_lines: true,
            ..crate::tooltip::TooltipBehavior::NEAREST_XY
        }
    }

    fn hover_targets(
        &self,
        layer: &DrawLayer,
        panel: &Panel,
    ) -> Result<Option<crate::tooltip::HoverTargets>> {
        Ok(super::hover::circles(layer, panel, |_| TEXT_HOVER_RADIUS))
    }

    fn name(&self) -> &'static str {
        "label"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        LABEL_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        PARAMETERS
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        draw_texts(layer, panel, true)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        draw_key(styles, cell, true)
    }
}

/// Resolved look of one text.
pub(super) struct Look {
    pub(super) content: String,
    pub(super) size: f64,
    font: Font,
    color: Color,
    pub(super) angle: f64,
    hjust: f64,
    vjust: f64,
    fill: Color,
    alpha: f64,
}

pub(super) fn look(styles: &Styles, row: usize, label: bool) -> Result<Look> {
    let alpha = styles.numbers(Aesthetic::Alpha)?.get(row);
    let family = FontFamily::from_name(styles.texts(Aesthetic::Family)?.get_ref(row)).0;
    let face = FontFace::from_name(styles.texts(Aesthetic::Fontface)?.get_ref(row));
    let color = styles.colors(Aesthetic::Color)?.get(row);
    Ok(Look {
        content: styles.texts(Aesthetic::Label)?.get_ref(row).clone(),
        size: styles.numbers(Aesthetic::Size)?.get(row) * TEXT_UNIT_SIZE,
        font: Font { family, face },
        // A label's alpha applies to its box fill only (lets-plot).
        color: if label {
            color
        } else {
            color.with_opacity(alpha)
        },
        angle: styles.numbers(Aesthetic::Angle)?.get(row),
        hjust: styles.numbers(Aesthetic::Hjust)?.get(row),
        vjust: styles.numbers(Aesthetic::Vjust)?.get(row),
        fill: if label {
            styles.colors(Aesthetic::Fill)?.get(row)
        } else {
            Color::WHITE
        },
        alpha,
    })
}

fn key_size(styles: &Styles) -> Result<(f64, f64)> {
    let size = styles.numbers(Aesthetic::Size)?.get(0).max(0.0) * TEXT_UNIT_SIZE;
    Ok((size, size))
}

fn draw_key(styles: &Styles, cell: Rect, label: bool) -> Result<Vec<Node>> {
    let mut look = look(styles, 0, label)?;
    look.content = "a".to_owned();
    look.hjust = 0.5;
    look.vjust = 0.5;
    look.angle = 0.0;
    let centre = (cell.x + cell.width / 2.0, cell.y + cell.height / 2.0);
    Ok(draw_one(
        &look,
        centre,
        label,
        1.0,
        LABEL_PADDING,
        LABEL_RADIUS,
    ))
}

fn draw_texts(layer: &DrawLayer, panel: &Panel, label: bool) -> Result<Vec<Node>> {
    let (Some(x), Some(y)) = (
        layer.data.continuous(Aesthetic::X),
        layer.data.continuous(Aesthetic::Y),
    ) else {
        return Ok(Vec::new());
    };
    let nudge_x = layer.parameters.number("nudge_x")?.unwrap_or(0.0);
    let nudge_y = layer.parameters.number("nudge_y")?.unwrap_or(0.0);
    let padding = layer
        .parameters
        .number("label_padding")?
        .unwrap_or(LABEL_PADDING);
    let radius = layer.parameters.number("label_r")?.unwrap_or(LABEL_RADIUS);
    let border = layer
        .parameters
        .number("label_size")?
        .unwrap_or(LABEL_BORDER);
    let mut nodes = Vec::new();
    for row in 0..layer.data.row_count() {
        let (px, py) = panel.map(
            x.get(row).unwrap_or(f64::NAN) + nudge_x,
            y.get(row).unwrap_or(f64::NAN) + nudge_y,
        );
        if !px.is_finite() || !py.is_finite() {
            continue;
        }
        let look = look(layer.styles, row, label)?;
        if look.content.is_empty() || look.size <= 0.0 {
            continue;
        }
        nodes.extend(draw_one(&look, (px, py), label, border, padding, radius));
    }
    Ok(nodes)
}

/// The text (and box) of one row, placed so that the point sits at (`hjust`,
/// `vjust`) of the text block. Lines are one font size apart; `vjust` spans
/// 0.7 em of the block's height (lets-plot's baseline placement). A power of ten
/// in the text (`·\(10^{8}\)`) is drawn as a superscript.
pub(super) fn draw_one(
    look: &Look,
    point: (f64, f64),
    label: bool,
    border: f64,
    padding: f64,
    radius: f64,
) -> Vec<Node> {
    let lines: Vec<Label> = look
        .content
        .split('\n')
        .map(|line| power_label(line.to_owned()))
        .collect();
    let widths: Vec<f64> = lines
        .iter()
        .map(|line| {
            text::measure_label(
                &line.text,
                line.superscript.as_deref(),
                line.suffix.as_deref(),
                look.size,
                look.font,
            )
        })
        .collect();
    let width = widths.iter().copied().fold(0.0, f64::max);
    let line_count = lines.len() as f64;
    let (sin, cos) = look.angle.to_radians().sin_cos();
    // Offset of the block centre from the point, in the text's own frame (y down).
    let block_height = line_count * look.size;
    let pad = if label { padding * look.size } else { 0.0 };
    // A label is placed by its box (its top at the point for vjust = 1, as
    // lets-plot); plain text by its baselines.
    let centre_offset = if label {
        (
            (0.5 - look.hjust) * (width + 2.0 * pad),
            (look.vjust - 0.5) * (block_height + 2.0 * pad),
        )
    } else {
        (
            (0.5 - look.hjust) * width,
            (look.vjust - 0.5) * VJUST_SPAN * look.size
                - (1.0 - look.vjust) * (line_count - 1.0) * look.size
                + (line_count - 1.0) * look.size / 2.0,
        )
    };
    let rotate =
        |(dx, dy): (f64, f64)| (point.0 + dx * cos + dy * sin, point.1 - dx * sin + dy * cos);
    let mut nodes = Vec::new();
    if label {
        let (half_width, half_height) = (width / 2.0 + pad, block_height / 2.0 + pad);
        let ring = rounded_box(half_width, half_height, radius * 2.0 * half_height);
        let points = ring
            .into_iter()
            .map(|(dx, dy)| rotate((centre_offset.0 + dx, centre_offset.1 + dy)))
            .collect();
        let fill = look.fill.with_opacity(look.alpha);
        nodes.push(Node::Path(Path {
            points,
            closed: true,
            fill: (fill.alpha > 0).then_some(fill),
            stroke: (border > 0.0).then(|| Stroke::solid(look.color, border)),
        }));
    }
    for (index, (line, line_width)) in lines.into_iter().zip(widths).enumerate() {
        // Line centres, one font size apart, around the block centre.
        let line_offset = (index as f64 - (line_count - 1.0) / 2.0) * look.size;
        // Each line aligned within the block by hjust (left-aligned for 0).
        let line_shift = (look.hjust - 0.5) * (width - line_width);
        let (x, y) = rotate((centre_offset.0 + line_shift, centre_offset.1 + line_offset));
        nodes.push(Node::Text(Text {
            content: line.text,
            superscript: line.superscript,
            suffix: line.suffix,
            x,
            y,
            size: look.size,
            font: look.font,
            color: look.color,
            horizontal: HorizontalAlign::Middle,
            vertical: VerticalAlign::Middle,
            angle: look.angle,
        }));
    }
    nodes
}

/// Outline of a box centred on the origin with rounded corners.
fn rounded_box(half_width: f64, half_height: f64, radius: f64) -> Vec<(f64, f64)> {
    let radius = radius.min(half_width).min(half_height).max(0.0);
    let mut points = Vec::with_capacity(4 * (CORNER_STEPS + 1));
    // Corner centres with the angle where each arc starts (y down, clockwise).
    let corners = [
        (half_width - radius, -half_height + radius, -90.0),
        (half_width - radius, half_height - radius, 0.0),
        (-half_width + radius, half_height - radius, 90.0),
        (-half_width + radius, -half_height + radius, 180.0),
    ];
    for (centre_x, centre_y, start) in corners {
        for step in 0..=CORNER_STEPS {
            let angle = (start + 90.0 * step as f64 / CORNER_STEPS as f64).to_radians();
            points.push((
                centre_x + radius * angle.cos(),
                centre_y + radius * angle.sin(),
            ));
        }
    }
    points
}
