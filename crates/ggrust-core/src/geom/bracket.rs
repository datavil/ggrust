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
//! What it does not do (yet): `orientation="y"` (vertical brackets from
//! `ymin`/`ymax`), `geom_bracket_dodge`, `label_format`, `na_text`, `size_unit`,
//! `nudge_unit`, `lineheight` (labels lift by one font size per line), mapping
//! columns to `lenstart`/`lenend`/`segment_*` (warned; constants work).

use super::lines;
use super::point::POINT_UNIT_SIZE;
use super::text;
use super::{DrawLayer, Geom};
use crate::coord::Panel;
use crate::data::{ContinuousColumn, resolution};
use crate::parameters::Parameters;
use crate::scene::{Color, Node, Path, Rect, Stroke};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};
use crate::{Error, Result};

/// lets-plot `DEFAULT_SEGMENT_COLOR`, "not set, use `color`": `rgba(255, 255, 255,
/// 0.999)`, whose alpha rounds to 255. So an explicit opaque white also means `color`.
const UNSET_SEGMENT_COLOR: Color = Color::rgb(255, 255, 255);

const DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
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
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
    (Aesthetic::Lenstart, StyleDefault::Number(5.0)),
    (Aesthetic::Lenend, StyleDefault::Number(5.0)),
    (
        Aesthetic::SegmentColor,
        StyleDefault::Color(UNSET_SEGMENT_COLOR),
    ),
    (Aesthetic::SegmentSize, StyleDefault::Number(0.75)),
    (Aesthetic::SegmentAlpha, StyleDefault::Number(f64::NAN)),
];

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
            "bracket_shorten",
            "tiplength_unit",
            "orientation",
        ]
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let (Some(x_min), Some(x_max), Some(y)) = (
            layer.data.continuous(Aesthetic::Xmin),
            layer.data.continuous(Aesthetic::Xmax),
            layer.data.continuous(Aesthetic::Y),
        ) else {
            return Ok(Vec::new());
        };
        let options = Options::parse(&layer.parameters, y)?;
        let value = |column: &ContinuousColumn, row: usize| column.get(row).unwrap_or(f64::NAN);
        let mut brackets = Vec::new();
        let mut labels = Vec::new();
        for row in 0..layer.data.row_count() {
            let (x_min, x_max) = (
                value(x_min, row) + options.nudge_x,
                value(x_max, row) + options.nudge_x,
            );
            let y = value(y, row) + options.nudge_y;
            if !(x_min.is_finite() && x_max.is_finite() && y.is_finite()) {
                continue;
            }
            let middle = (x_min + x_max) / 2.0;
            if let Some(bracket) = bracket(layer.styles, row, panel, &options, (x_min, x_max), y)? {
                brackets.push(bracket);
            }
            labels.extend(label(layer.styles, row, panel.map(middle, y), panel)?);
        }
        brackets.extend(labels);
        Ok(brackets)
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

/// Layer options of a bracket.
struct Options {
    nudge_x: f64,
    nudge_y: f64,
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
    fn parse(parameters: &Parameters, y: &ContinuousColumn) -> Result<Self> {
        let tip_unit = match parameters.string("tiplength_unit")? {
            None | Some("size") => TipUnit::Size,
            Some("res") => TipUnit::Resolution,
            Some("identity") => TipUnit::Identity,
            Some("px") => TipUnit::Pixel,
            Some(other) => {
                return Err(Error::InvalidSpec(format!(
                    "at '{}': unknown tiplength_unit '{other}'; expected one of: res, \
                     identity, size, px",
                    parameters.path_of("tiplength_unit")
                )));
            }
        };
        match parameters.string("orientation")? {
            None | Some("x") => {}
            Some(other) => {
                return Err(Error::InvalidSpec(format!(
                    "at '{}': orientation '{other}' is not supported by ggrust yet; \
                     expected 'x' (vertical brackets are planned)",
                    parameters.path_of("orientation")
                )));
            }
        }
        Ok(Options {
            nudge_x: parameters.number("nudge_x")?.unwrap_or(0.0),
            nudge_y: parameters.number("nudge_y")?.unwrap_or(0.0),
            shorten: parameters.number("bracket_shorten")?.unwrap_or(0.0),
            tip_unit,
            y_resolution: if tip_unit == TipUnit::Resolution {
                resolution(y)
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
    ];
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
/// own frame (lets-plot `BracketGeom.labelNudge`).
fn label(styles: &Styles, row: usize, point: (f64, f64), panel: &Panel) -> Result<Vec<Node>> {
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
    Ok(text::draw_one(&look, lifted, false, 0.0, 0.0, 0.0))
}

/// The line style of a row's bracket: lets-plot `TextUtil.toSegmentAes`.
fn segment_stroke(styles: &Styles, row: usize) -> Result<Option<Stroke>> {
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
