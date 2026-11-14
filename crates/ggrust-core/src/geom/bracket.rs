//! `geom_bracket`: a labelled bracket per row from `xmin` to `xmax` at height `y`
//! (significance marks), with tips hanging `lenstart`/`lenend` below the bar.
//!
//! Follows lets-plot `BracketGeom`: the bar is a 4-point path (left tip, left end,
//! right end, right tip) stroked with `segment_color` (`color` unless set),
//! `segment_size` x 2.2 px and `segment_alpha` (`alpha` unless set); the label is
//! `geom_text` at the bar's middle, lifted by half its text height, turned with
//! the coord under `coord_flip`. Tip lengths are in `tiplength_unit`: `size`
//! (default, 2.2 px per unit), `px`, `identity` (data units) or `res` (the y
//! resolution). `bracket_shorten` pulls both ends toward the middle. The layer
//! does not inherit the plot mapping and has no legend unless asked
//! (`inherit_aes`, `show_legend`). All brackets are drawn before all labels.
//!
//! `geom_bracket_dodge` (`BracketDodgeGeom`) places the bracket ends on dodge
//! slots inside a category instead of `xmin`/`xmax`.
//!
//! What it does not do (yet): `orientation="y"` (vertical brackets from
//! `ymin`/`ymax`), `label_format`, `na_text`, `size_unit`,
//! `lineheight` (labels lift by one font size per line), mapping
//! columns to `segment_*` (warned; constants work). Mapped `lenstart`/`lenend`
//! numbers are drawn as they are.

use super::lines;
use super::nudge::Nudge;
use super::point::POINT_UNIT_SIZE;
use super::text;
use super::{DrawLayer, Geom};
use crate::coord::Panel;
use crate::data::{ContinuousColumn, LayerData};
use crate::parameters::Parameters;
use crate::scene::{Color, Group, Node, Path, Rect, Stroke};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};
use crate::{Error, Result};

/// lets-plot `DEFAULT_SEGMENT_COLOR`, "not set, use `color`": `rgba(255, 255, 255,
/// 0.999)`, whose alpha rounds to 255. So an explicit opaque white also means `color`.
pub(super) const UNSET_SEGMENT_COLOR: Color = Color::rgb(255, 255, 255);

/// The bracket style defaults, plus `extra` ones.
macro_rules! bracket_defaults {
    ($($extra:expr),* $(,)?) => {
        &[
    (Aesthetic::Label, StyleDefault::Text("")),
            (Aesthetic::Color, StyleDefault::PEN),
            (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
            (Aesthetic::Size, StyleDefault::Number(7.0)),
            (Aesthetic::Family, StyleDefault::Text("sans")),
            (Aesthetic::Fontface, StyleDefault::Text("plain")),
            (Aesthetic::Angle, StyleDefault::Number(0.0)),
            (Aesthetic::Hjust, StyleDefault::Number(0.5)),
            (Aesthetic::Vjust, StyleDefault::Number(0.5)),
            (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
            (Aesthetic::Lenstart, StyleDefault::Number(5.0)),
            (Aesthetic::Lenend, StyleDefault::Number(5.0)),
            (
                Aesthetic::SegmentColor,
                StyleDefault::Color(UNSET_SEGMENT_COLOR),
            ),
            (Aesthetic::SegmentSize, StyleDefault::Number(0.75)),
            (Aesthetic::SegmentAlpha, StyleDefault::Number(f64::NAN)),
            $($extra,)*
        ]
    };
}

const DEFAULTS: &[(Aesthetic, StyleDefault)] = bracket_defaults!();

/// `geom_bracket_dodge` also takes the dodge slots `istart` and `iend`.
const DODGE_DEFAULTS: &[(Aesthetic, StyleDefault)] = bracket_defaults!(
    (Aesthetic::Istart, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Iend, StyleDefault::Number(f64::NAN)),
);

/// lets-plot `DEF_DODGE_WIDTH` of `geom_bracket_dodge`.
const DEFAULT_DODGE_WIDTH: f64 = 0.95;

/// Labelled brackets.
pub struct BracketGeom;

impl Geom for BracketGeom {
    fn name(&self) -> &'static str {
        "bracket"
    }

    fn inherits_aes(&self) -> bool {
        false
    }

    fn shows_legend(&self) -> bool {
        false
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Xmin, Aesthetic::Xmax, Aesthetic::Y]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &[
            "nudge_x",
            "nudge_y",
            "nudge_unit",
            "bracket_shorten",
            "tiplength_unit",
            "orientation",
        ]
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let (Some(x_min), Some(x_max)) = (
            layer.data.continuous(Aesthetic::Xmin),
            layer.data.continuous(Aesthetic::Xmax),
        ) else {
            return Ok(Vec::new());
        };
        let value = |column: &ContinuousColumn, row: usize| column.get(row).unwrap_or(f64::NAN);
        draw_brackets(layer, panel, |row| {
            Ok(Some((value(x_min, row), value(x_max, row))))
        })
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        let width = styles.numbers(Aesthetic::SegmentSize)?.get(0).max(0.0) * POINT_UNIT_SIZE;
        Ok((width + 2.0, width + 2.0))
    }

    /// lets-plot `HLineLegendKeyElementFactory` with the bracket's line style.
    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        let Some(stroke) = segment_stroke(styles, 0)? else {
            return Ok(Vec::new());
        };
        let y = cell.y + cell.height / 2.0;
        Ok(vec![Node::Path(Path {
            points: vec![(cell.x, y), (cell.x + cell.width, y)],
            closed: false,
            fill: None,
            stroke: Some(stroke),
        })])
    }
}

/// Draw one bracket per row from the row's `limits` (data x, before nudging) at
/// height `y`; all bars first, then all labels.
fn draw_brackets(
    layer: &DrawLayer,
    panel: &Panel,
    limits: impl Fn(usize) -> Result<Option<(f64, f64)>>,
) -> Result<Vec<Node>> {
    let Some(y) = layer.data.continuous(Aesthetic::Y) else {
        return Ok(Vec::new());
    };
    let options = Options::parse(&layer.parameters, layer.data)?;
    let mut brackets = Vec::new();
    let mut labels = Vec::new();
    for row in 0..layer.data.row_count() {
        let Some((x_min, x_max)) = limits(row)? else {
            continue;
        };
        let (nudge_x, nudge_y) = options.nudge.data();
        let (x_min, x_max) = (x_min + nudge_x, x_max + nudge_x);
        let y = y.get(row).unwrap_or(f64::NAN) + nudge_y;
        if !(x_min.is_finite() && x_max.is_finite() && y.is_finite()) {
            continue;
        }
        let middle = (x_min + x_max) / 2.0;
        if let Some(bracket) = bracket(layer.styles, row, panel, &options, (x_min, x_max), y)? {
            brackets.push(bracket);
        }
        let anchor = panel.map(middle, y);
        let point = options.nudge.shift(anchor);
        labels.extend(label(layer.styles, row, (anchor, point), panel)?);
    }
    brackets.extend(labels);
    Ok(brackets)
}

/// Brackets between dodged groups inside each category: lets-plot
/// `BracketDodgeGeom`. Row `x` is the category; `istart`/`iend` are the slots
/// (0-based) of the dodged groups the bracket joins, out of `ngroup` slots (by
/// default the largest slot plus one, over the panel's rows, as lets-plot) in
/// `dodge_width`. Slots mapped to a text column draw nothing (a gap).
pub struct BracketDodgeGeom;

impl Geom for BracketDodgeGeom {
    fn name(&self) -> &'static str {
        "bracket_dodge"
    }

    fn inherits_aes(&self) -> bool {
        false
    }

    fn shows_legend(&self) -> bool {
        false
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        DODGE_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &[
            "nudge_x",
            "nudge_y",
            "nudge_unit",
            "bracket_shorten",
            "tiplength_unit",
            "orientation",
            "dodge_width",
            "ngroup",
        ]
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let Some(x) = layer.data.continuous(Aesthetic::X) else {
            return Ok(Vec::new());
        };
        let starts = layer.styles.numbers(Aesthetic::Istart)?;
        let ends = layer.styles.numbers(Aesthetic::Iend)?;
        let slots = |row: usize| {
            let (start, end) = (starts.get(row), ends.get(row));
            (start.is_finite() && end.is_finite()).then(|| (start.round(), end.round()))
        };
        let width = layer
            .parameters
            .number("dodge_width")?
            .unwrap_or(DEFAULT_DODGE_WIDTH);
        let groups = match layer.parameters.number("ngroup")? {
            Some(groups) if groups >= 1.0 && groups.fract() == 0.0 => Some(groups),
            Some(groups) => {
                return Err(Error::spec(
                    layer.parameters.path_of("ngroup"),
                    format!(
                        "ngroup={groups} is not a group count; expected a whole number \
                         >= 1 (the number of dodged groups per category), or leave it \
                         unset to count the slots in istart/iend"
                    ),
                ));
            }
            None => (0..layer.data.row_count())
                .filter_map(slots)
                .map(|(start, end)| start.max(end))
                .reduce(f64::max)
                .map(|largest| largest + 1.0),
        };
        let Some(groups) = groups else {
            return Ok(Vec::new());
        };
        if !width.is_finite() {
            return Ok(Vec::new());
        }
        let resolution = layer.data.resolution(Aesthetic::X);
        draw_brackets(layer, panel, |row| {
            let center = x.get(row).unwrap_or(f64::NAN);
            Ok(slots(row).map(|(start, end)| {
                let slot = |index: f64| dodge(center, index, groups, width, resolution);
                (slot(start), slot(end))
            }))
        })
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        BracketGeom.minimum_key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        BracketGeom.draw_key(styles, cell)
    }
}

/// The center of dodge slot `slot` of `groups` at category `center`: lets-plot
/// `BaseDodgePos.position` with the coordinate at the category center.
fn dodge(center: f64, slot: f64, groups: f64, width: f64, resolution: f64) -> f64 {
    center + (slot - (groups - 1.0) / 2.0) * resolution * width / groups
}

/// Layer options of a bracket.
struct Options {
    nudge: Nudge,
    shorten: f64,
    tip_unit: TipUnit,
    /// The y resolution, for `tiplength_unit="res"`.
    y_resolution: f64,
}

/// lets-plot `DimensionUnit` of the tip lengths.
#[derive(Clone, Copy, PartialEq)]
enum TipUnit {
    Resolution,
    Identity,
    Size,
    Pixel,
}

impl Options {
    fn parse(parameters: &Parameters, data: &LayerData) -> Result<Self> {
        let tip_unit = match parameters.string("tiplength_unit")? {
            None | Some("size") => TipUnit::Size,
            Some("res") => TipUnit::Resolution,
            Some("identity") => TipUnit::Identity,
            Some("px") => TipUnit::Pixel,
            Some(other) => {
                return Err(Error::spec(
                    parameters.path_of("tiplength_unit"),
                    format!(
                        "unknown tiplength_unit '{other}'; expected one of: res, \
                     identity, size, px"
                    ),
                ));
            }
        };
        match parameters.string("orientation")? {
            None | Some("x") => {}
            Some(other) => {
                return Err(Error::spec(
                    parameters.path_of("orientation"),
                    format!(
                        "orientation '{other}' is not supported by ggrust yet; \
                     expected 'x' (vertical brackets are planned)"
                    ),
                ));
            }
        }
        Ok(Options {
            nudge: Nudge::parse(parameters)?,
            shorten: parameters.number("bracket_shorten")?.unwrap_or(0.0),
            tip_unit,
            y_resolution: if tip_unit == TipUnit::Resolution {
                data.resolution(Aesthetic::Y)
            } else {
                1.0
            },
        })
    }

    /// Data units of y per tip length unit at height `y` (lets-plot
    /// `GeomHelper.getUnitResolution`).
    fn tip_scale(&self, panel: &Panel, x: f64, y: f64) -> f64 {
        let pixels_per_unit = || {
            let (start, end) = (panel.map(x, y), panel.map(x, y + 1.0));
            (end.0 - start.0).hypot(end.1 - start.1)
        };
        match self.tip_unit {
            TipUnit::Resolution => self.y_resolution,
            TipUnit::Identity => 1.0,
            TipUnit::Size => POINT_UNIT_SIZE / pixels_per_unit(),
            TipUnit::Pixel => 1.0 / pixels_per_unit(),
        }
    }
}

/// The bar and tips of one row, in pixels.
fn bracket(
    styles: &Styles,
    row: usize,
    panel: &Panel,
    options: &Options,
    (x_min, x_max): (f64, f64),
    y: f64,
) -> Result<Option<Node>> {
    let tip_start = styles.numbers(Aesthetic::Lenstart)?.get(row);
    let tip_end = styles.numbers(Aesthetic::Lenend)?.get(row);
    let Some(stroke) = segment_stroke(styles, row)? else {
        return Ok(None);
    };
    if !tip_start.is_finite() || !tip_end.is_finite() {
        return Ok(None);
    }
    let middle = (x_min + x_max) / 2.0;
    let half = (1.0 - options.shorten) * (x_max - x_min) / 2.0;
    let (start, end) = (middle - half, middle + half);
    let scale = options.tip_scale(panel, middle, y);
    let points = vec![
        panel.map(start, y - tip_start * scale),
        panel.map(start, y),
        panel.map(end, y),
        panel.map(end, y - tip_end * scale),
    ]
    .into_iter()
    .map(|point| options.nudge.shift(point))
    .collect::<Vec<_>>();
    if points.iter().any(|(x, y)| !x.is_finite() || !y.is_finite()) {
        return Ok(None);
    }
    Ok(Some(Node::Path(Path {
        points,
        closed: false,
        fill: None,
        stroke: Some(stroke),
    })))
}

/// The label of one row at `point` (pixels), lifted by half its height in its
/// own frame (lets-plot `BracketGeom.labelNudge`). Interactive zoom moves it
/// with `anchor`, the bar's middle before a pixel nudge.
fn label(
    styles: &Styles,
    row: usize,
    (anchor, point): ((f64, f64), (f64, f64)),
    panel: &Panel,
) -> Result<Vec<Node>> {
    let mut look = text::look(styles, row, false)?;
    if look.content.is_empty() || look.size <= 0.0 {
        return Ok(Vec::new());
    }
    if panel.flipped {
        look.angle -= 90.0;
    }
    let lift = look.content.split('\n').count() as f64 * look.size / 2.0;
    let (sin, cos) = look.angle.to_radians().sin_cos();
    let lifted = (point.0 - lift * sin, point.1 - lift * cos);
    // Anchored at the bar's middle, so interactive zoom moves the label with it.
    Ok(vec![Node::Group(Group {
        key: None,
        clip: None,
        role: None,
        anchor: Some(anchor),
        nodes: text::draw_one(&look, lifted, false, 0.0, 0.0, 0.0),
    })])
}

/// The line style of a row's bracket: lets-plot `TextUtil.toSegmentAes`.
pub(super) fn segment_stroke(styles: &Styles, row: usize) -> Result<Option<Stroke>> {
    let segment_color = styles.colors(Aesthetic::SegmentColor)?.get(row);
    let color = if segment_color == UNSET_SEGMENT_COLOR {
        styles.colors(Aesthetic::Color)?.get(row)
    } else {
        segment_color
    };
    let segment_alpha = styles.numbers(Aesthetic::SegmentAlpha)?.get(row);
    let alpha = if segment_alpha.is_nan() {
        styles.numbers(Aesthetic::Alpha)?.get(row)
    } else {
        segment_alpha
    };
    let width = styles.numbers(Aesthetic::SegmentSize)?.get(row) * POINT_UNIT_SIZE;
    let linetype = styles.linetypes(Aesthetic::Linetype)?.get_ref(row);
    Ok(lines::stroke(color.with_opacity(alpha), width, linetype))
}

#[cfg(test)]
mod tests {
    use super::dodge;

    #[test]
    fn dodge_slots_match_lets_plot() {
        // Two groups in 0.95 at category 1: centers at 1 -/+ 0.2375 (BaseDodgePos).
        assert!((dodge(1.0, 0.0, 2.0, 0.95, 1.0) - 0.7625).abs() < 1e-12);
        assert!((dodge(1.0, 1.0, 2.0, 0.95, 1.0) - 1.2375).abs() < 1e-12);
        // The middle of three slots is the category; resolution scales the spread.
        assert_eq!(dodge(3.0, 1.0, 3.0, 0.9, 2.0), 3.0);
        assert!((dodge(3.0, 2.0, 3.0, 0.9, 2.0) - 3.6).abs() < 1e-12);
    }
}
