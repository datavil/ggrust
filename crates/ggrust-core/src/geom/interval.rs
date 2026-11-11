//! The interval family, as lets-plot's `ErrorBarGeom`, `LineRangeGeom`,
//! `PointRangeGeom` and `CrossBarGeom`: a vertical range from `ymin` to `ymax`
//! at `x`, drawn as an error bar (fences `width` wide, default 0.45), a line, a
//! line with a point at `y` (`size` x `fatten`, default 5), or a box with a
//! midline at `y` (outline width x `fatten`, default 2.5; box `width` default 0.9).
//!
//! Error bars and crossbars get `xmin`/`xmax` spans of `width` x the x
//! resolution in setup (as boxplots), so the span trains the x axis and dodging
//! moves whole bars; crossbars dodge by 0.95 by default, the others stay put.
//! Error bars and lines fade their strokes with `alpha`, crossbars their fill
//! only, point ranges both line and point.
//!
//! Horizontal intervals (`y`, `xmin`, `xmax`, or `orientation="y"`) are the same
//! code: the layer runs y-oriented, so these geoms see x and y swapped.
//!
//! What it does not do (yet): `width_unit` other than `res`, crossbar `labels`, a `linewidth` scale.
//! Point range legend keys fit the fattened point (lets-plot's are smaller). Point ranges draw every
//! line, then every point (lets-plot alternates per row).

use super::lines::{self, LINE_STYLE_DEFAULTS};
use super::point::{self, POINT_UNIT_SIZE};
use super::{DrawLayer, Geom};
use crate::coord::Panel;
use crate::data::{Column, ContinuousColumn, LayerData};
use crate::parameters::Parameters;
use crate::position::DefaultPosition;
use crate::scene::{Node, Path, PerItem, Rect, Rects, Stroke};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};
use crate::tooltip::{HoverTargets, TooltipBehavior};
use crate::{Error, Result};

/// lets-plot `AestheticsDefaults.errorBar`: `width` 0.45.
const ERRORBAR_WIDTH: f64 = 0.45;
/// lets-plot `AestheticsDefaults.crossBar`: `width` 0.9.
const CROSSBAR_WIDTH: f64 = 0.9;
/// lets-plot `CrossBarGeom.fattenMidline`.
const CROSSBAR_FATTEN: f64 = 2.5;
/// lets-plot `PointRangeGeom.DEF_FATTEN`.
const POINTRANGE_FATTEN: f64 = 5.0;
/// lets-plot `LineRangeGeom.MIN_TOOLTIP_RECTANGLE_WIDTH` (px).
const MIN_HOVER_WIDTH: f64 = 2.0;

/// lets-plot crossbar style: pen outline, paper fill, size 0.75.
const CROSSBAR_STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (Aesthetic::Color, StyleDefault::PEN),
    (Aesthetic::Fill, StyleDefault::PAPER),
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Size, StyleDefault::Number(0.75)),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
];

/// lets-plot point range style: the point's (`size` 0.75 before fattening) and
/// the line's (`linewidth` 0.75).
const POINTRANGE_STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Color, StyleDefault::PEN),
    (Aesthetic::Fill, StyleDefault::PAPER),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
    (Aesthetic::Shape, StyleDefault::Shape(16)),
    (Aesthetic::Size, StyleDefault::Number(0.75)),
    (Aesthetic::Stroke, StyleDefault::Number(0.75)),
    (Aesthetic::Linewidth, StyleDefault::Number(0.75)),
];

/// Tooltips of the family (lets-plot: hover along x, ranges as side hints).
const TOOLTIPS: TooltipBehavior = TooltipBehavior {
    side: &[Aesthetic::Ymax, Aesthetic::Ymin],
    ..TooltipBehavior::ALONG_X
};

/// Error bars.
pub struct ErrorBarGeom;

/// Vertical line ranges.
pub struct LineRangeGeom;

/// Line ranges with a point at `y`.
pub struct PointRangeGeom;

/// Boxes from `ymin` to `ymax` with a midline at `y`.
pub struct CrossBarGeom;

impl Geom for ErrorBarGeom {
    fn name(&self) -> &'static str {
        "errorbar"
    }

    fn horizontal_from_ranges(&self) -> bool {
        true
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Ymin, Aesthetic::Ymax]
    }

    fn optional_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Width]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        LINE_STYLE_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["width", "width_unit"]
    }

    fn setup_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Xmin, Aesthetic::Xmax]
    }

    fn span_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Xmin, Aesthetic::Xmax]
    }

    fn setup(&self, data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        width_spans(data, parameters, ERRORBAR_WIDTH)
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let [x, xmin, xmax, ymin, ymax] = columns(
            layer,
            [
                Aesthetic::X,
                Aesthetic::Xmin,
                Aesthetic::Xmax,
                Aesthetic::Ymin,
                Aesthetic::Ymax,
            ],
        )?;
        let mut nodes = Vec::new();
        for row in 0..layer.data.row_count() {
            let values = [x, xmin, xmax, ymin, ymax].map(|column| finite(column, row));
            let [Some(x), Some(x0), Some(x1), Some(y0), Some(y1)] = values else {
                continue;
            };
            let stroke = lines::stroke_at(layer.styles, row)?;
            // lets-plot `errorBarShapeSegments`: top fence, bottom fence, range.
            for (from, to) in [
                ((x0, y1), (x1, y1)),
                ((x0, y0), (x1, y0)),
                ((x, y1), (x, y0)),
            ] {
                nodes.extend(line(panel, from, to, stroke.clone()));
            }
        }
        Ok(nodes)
    }

    fn tooltip_behavior(&self) -> TooltipBehavior {
        TOOLTIPS
    }

    fn hover_targets(&self, layer: &DrawLayer, panel: &Panel) -> Result<Option<HoverTargets>> {
        Ok(super::hover::rectangles(
            layer,
            panel,
            (Aesthetic::Ymin, Aesthetic::Ymax),
        ))
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        lines::key_size(styles)
    }

    /// lets-plot `ErrorBarLegendKeyElementFactory`: fences of the default width
    /// share of the key, inset by half the stroke.
    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        let Some(stroke) = lines::stroke_at(styles, 0)? else {
            return Ok(Vec::new());
        };
        let width = ERRORBAR_WIDTH * (cell.width - stroke.width);
        let left = cell.x + (cell.width - width) / 2.0;
        let right = left + width;
        let top = cell.y + stroke.width / 2.0;
        let bottom = cell.y + cell.height - stroke.width / 2.0;
        let center = cell.x + cell.width / 2.0;
        Ok([
            ((left, top), (right, top)),
            ((left, bottom), (right, bottom)),
            ((center, top), (center, bottom)),
        ]
        .into_iter()
        .map(|(from, to)| pixel_line(from, to, stroke.clone()))
        .collect())
    }
}

impl Geom for LineRangeGeom {
    fn name(&self) -> &'static str {
        "linerange"
    }

    fn horizontal_from_ranges(&self) -> bool {
        true
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Ymin, Aesthetic::Ymax]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        LINE_STYLE_DEFAULTS
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let [x, ymin, ymax] = columns(layer, [Aesthetic::X, Aesthetic::Ymin, Aesthetic::Ymax])?;
        let mut nodes = Vec::new();
        for row in 0..layer.data.row_count() {
            let [Some(x), Some(y0), Some(y1)] = [x, ymin, ymax].map(|column| finite(column, row))
            else {
                continue;
            };
            nodes.extend(line(
                panel,
                (x, y0),
                (x, y1),
                lines::stroke_at(layer.styles, row)?,
            ));
        }
        Ok(nodes)
    }

    fn tooltip_behavior(&self) -> TooltipBehavior {
        TOOLTIPS
    }

    fn hover_targets(&self, layer: &DrawLayer, panel: &Panel) -> Result<Option<HoverTargets>> {
        let sizes = layer.styles.numbers(Aesthetic::Size)?;
        Ok(range_rectangles(layer, panel, |row| {
            sizes.get(row) * POINT_UNIT_SIZE
        }))
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        lines::key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        lines::key_line(styles, cell, true)
    }
}

impl Geom for PointRangeGeom {
    fn name(&self) -> &'static str {
        "pointrange"
    }

    fn horizontal_from_ranges(&self) -> bool {
        true
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Ymin, Aesthetic::Ymax]
    }

    fn optional_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Y]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        POINTRANGE_STYLE_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["fatten"]
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let fatten = fatten(&layer.parameters, POINTRANGE_FATTEN)?;
        draw_point_ranges(layer, panel, fatten)
    }

    fn tooltip_behavior(&self) -> TooltipBehavior {
        TOOLTIPS
    }

    fn hover_targets(&self, layer: &DrawLayer, panel: &Panel) -> Result<Option<HoverTargets>> {
        let fatten = fatten(&layer.parameters, POINTRANGE_FATTEN)?;
        Ok(point_range_targets(layer, panel, fatten))
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        point_range_key_size(styles, POINTRANGE_FATTEN)
    }

    /// lets-plot: a vertical line key under a point key fattened by 5.
    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        draw_point_range_key(styles, cell, POINTRANGE_FATTEN)
    }
}

/// Point ranges with points `fatten` x `size` (also ggdist's point intervals).
pub(super) fn draw_point_ranges(
    layer: &DrawLayer,
    panel: &Panel,
    fatten: f64,
) -> Result<Vec<Node>> {
    let [x, ymin, ymax] = columns(layer, [Aesthetic::X, Aesthetic::Ymin, Aesthetic::Ymax])?;
    let y = layer.data.continuous(Aesthetic::Y);
    let row_count = layer.data.row_count();
    let mut nodes = Vec::new();
    let (mut pixel_x, mut pixel_y) = (Vec::with_capacity(row_count), Vec::new());
    for row in 0..row_count {
        let [Some(x), Some(y0), Some(y1)] = [x, ymin, ymax].map(|column| finite(column, row))
        else {
            // No range, no point (lets-plot skips the row).
            pixel_x.push(f64::NAN);
            pixel_y.push(f64::NAN);
            continue;
        };
        nodes.extend(line(
            panel,
            (x, y0),
            (x, y1),
            range_stroke(layer.styles, row)?,
        ));
        let middle = y.and_then(|y| finite(y, row)).unwrap_or(f64::NAN);
        let (px, py) = panel.map(x, middle);
        pixel_x.push(px);
        pixel_y.push(py);
    }
    if y.is_some() {
        let mut points = point::markers(layer.styles, row_count, false, fatten)?;
        points.x = pixel_x;
        points.y = pixel_y;
        nodes.push(Node::Points(points));
    }
    Ok(nodes)
}

/// Hover rectangles of point ranges with points `fatten` x `size`.
pub(super) fn point_range_targets(
    layer: &DrawLayer,
    panel: &Panel,
    fatten: f64,
) -> Option<HoverTargets> {
    range_rectangles(layer, panel, |row| {
        point::marker_extent(layer.styles, row, fatten) * 2.0
    })
}

/// Legend key size of point ranges with points `fatten` x `size`.
pub(super) fn point_range_key_size(styles: &Styles, fatten: f64) -> Result<(f64, f64)> {
    let side = point::marker_extent(styles, 0, fatten) * 2.0 + 2.0;
    let (line_width, line_height) = lines::key_size(styles)?;
    Ok((side.max(line_width), side.max(line_height)))
}

/// A point range legend key: a vertical line under a point `fatten` x `size`.
pub(super) fn draw_point_range_key(styles: &Styles, cell: Rect, fatten: f64) -> Result<Vec<Node>> {
    let mut nodes = Vec::new();
    if let Some(stroke) = range_stroke(styles, 0)? {
        let x = cell.x + cell.width / 2.0;
        nodes.push(pixel_line((x, cell.y), (x, cell.y + cell.height), stroke));
    }
    let mut points = point::markers(styles, 1, true, fatten)?;
    points.x = vec![cell.x + cell.width / 2.0];
    points.y = vec![cell.y + cell.height / 2.0];
    nodes.push(Node::Points(points));
    Ok(nodes)
}

impl Geom for CrossBarGeom {
    fn name(&self) -> &'static str {
        "crossbar"
    }

    fn horizontal_from_ranges(&self) -> bool {
        true
    }

    fn default_position(&self) -> DefaultPosition {
        DefaultPosition::dodge(0.95)
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Ymin, Aesthetic::Ymax]
    }

    fn optional_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Y, Aesthetic::Width]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        CROSSBAR_STYLE_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["width", "width_unit", "fatten"]
    }

    fn setup_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Xmin, Aesthetic::Xmax]
    }

    fn span_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Xmin, Aesthetic::Xmax]
    }

    fn setup(&self, data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        width_spans(data, parameters, CROSSBAR_WIDTH)
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let fatten = fatten(&layer.parameters, CROSSBAR_FATTEN)?;
        let [xmin, xmax, ymin, ymax] = columns(
            layer,
            [
                Aesthetic::Xmin,
                Aesthetic::Xmax,
                Aesthetic::Ymin,
                Aesthetic::Ymax,
            ],
        )?;
        let fills = layer.styles.colors(Aesthetic::Fill)?;
        let alphas = layer.styles.numbers(Aesthetic::Alpha)?;
        let rows = 0..layer.data.row_count();
        // lets-plot `CrossBarGeom`: every box, then every midline.
        let mut nodes = Vec::new();
        for row in rows.clone() {
            let values = [xmin, ymin, xmax, ymax].map(|column| finite(column, row));
            let [Some(x0), Some(y0), Some(x1), Some(y1)] = values else {
                continue;
            };
            let (left, bottom) = panel.map(x0, y0);
            let (right, top) = panel.map(x1, y1);
            if ![left, bottom, right, top]
                .iter()
                .all(|value| value.is_finite())
            {
                continue;
            }
            // lets-plot `GeomHelper.toClient`: keep flat boxes visible.
            let rect = Rect {
                x: left.min(right),
                y: top.min(bottom),
                width: (right - left).abs().max(0.1),
                height: (bottom - top).abs().max(0.1),
            };
            let fill = fills.get(row).with_opacity(alphas.get(row));
            nodes.push(Node::Rects(Rects {
                rects: vec![rect],
                fill: PerItem::Uniform((fill.alpha > 0).then_some(fill)),
                stroke: lines::stroke_with_alpha(layer.styles, row, false)?,
            }));
        }
        if let Some(y) = layer.data.continuous(Aesthetic::Y)
            && fatten != 0.0
        {
            for row in rows {
                let [Some(x0), Some(x1), Some(y)] =
                    [xmin, xmax, y].map(|column| finite(column, row))
                else {
                    continue;
                };
                let stroke = lines::stroke_with_alpha(layer.styles, row, false)?;
                nodes.extend(line(panel, (x0, y), (x1, y), fattened(stroke, fatten)));
            }
        }
        Ok(nodes)
    }

    /// lets-plot also hints `y` beside the range.
    fn tooltip_behavior(&self) -> TooltipBehavior {
        TooltipBehavior {
            side: &[Aesthetic::Ymax, Aesthetic::Y, Aesthetic::Ymin],
            ..TOOLTIPS
        }
    }

    fn hover_targets(&self, layer: &DrawLayer, panel: &Panel) -> Result<Option<HoverTargets>> {
        Ok(super::hover::rectangles(
            layer,
            panel,
            (Aesthetic::Ymin, Aesthetic::Ymax),
        ))
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        let side = styles.numbers(Aesthetic::Size)?.get(0).max(0.0) * POINT_UNIT_SIZE + 2.0;
        Ok((side, side))
    }

    /// lets-plot `BoxHelper.legendFactory(whiskers = false)`: a box 80% of the key
    /// wide and full height (inset by the stroke) with an unfattened midline.
    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        let stroke = lines::stroke_with_alpha(styles, 0, false)?;
        let stroke_width = stroke.as_ref().map_or(0.0, |stroke| stroke.width);
        let width = (cell.width - stroke_width) * 0.8;
        let height = cell.height - stroke_width;
        let x = cell.x + (cell.width - width) / 2.0;
        let y = cell.y + stroke_width / 2.0;
        let alpha = styles.numbers(Aesthetic::Alpha)?.get(0);
        let fill = styles.colors(Aesthetic::Fill)?.get(0).with_opacity(alpha);
        let mut nodes = vec![Node::Rects(Rects {
            rects: vec![Rect {
                x,
                y,
                width,
                height,
            }],
            fill: PerItem::Uniform((fill.alpha > 0).then_some(fill)),
            stroke: stroke.clone(),
        })];
        if let Some(stroke) = stroke {
            let middle = y + height / 2.0;
            nodes.push(pixel_line((x, middle), (x + width, middle), stroke));
        }
        Ok(nodes)
    }
}

/// Setup of error bars and crossbars: `xmin`/`xmax` spans of the mapped
/// `width`, else the `width` option, else `default_width`, times the x resolution.
fn width_spans(
    mut data: LayerData,
    parameters: &Parameters,
    default_width: f64,
) -> Result<LayerData> {
    if let Some(unit) = parameters.string("width_unit")?
        && unit != "res"
    {
        return Err(Error::spec(
            parameters.path_of("width_unit"),
            format!("width_unit '{unit}' is not supported by ggrust yet; supported: res"),
        ));
    }
    let Some(x) = data.continuous(Aesthetic::X).cloned() else {
        return Ok(data);
    };
    let constant = parameters.number("width")?.unwrap_or(default_width);
    let widths = data.continuous(Aesthetic::Width).cloned();
    let resolution = data.resolution(Aesthetic::X);
    let width_of = |row: usize| match &widths {
        Some(widths) => widths.get(row).unwrap_or(f64::NAN),
        None => constant,
    };
    let half = |row: usize| width_of(row) * resolution / 2.0;
    let location = |row: usize| x.get(row).unwrap_or(f64::NAN);
    let left = (0..x.len()).map(|row| location(row) - half(row)).collect();
    let right = (0..x.len()).map(|row| location(row) + half(row)).collect();
    let continuous = |values| Column::Continuous(ContinuousColumn::from_values(values));
    data.insert(Aesthetic::Xmin, continuous(left))?;
    data.insert(Aesthetic::Xmax, continuous(right))?;
    // Dodging reads the bar width from here (else 1).
    let widths: Vec<f64> = (0..x.len()).map(width_of).collect();
    data.insert(Aesthetic::Width, continuous(widths))?;
    Ok(data)
}

/// The layer's continuous columns for `aesthetics`, or an error naming the
/// missing one.
fn columns<'a, const N: usize>(
    layer: &DrawLayer<'a>,
    aesthetics: [Aesthetic; N],
) -> Result<[&'a ContinuousColumn; N]> {
    let mut found = Vec::with_capacity(N);
    for aesthetic in aesthetics {
        found.push(layer.data.continuous(aesthetic).ok_or_else(|| {
            Error::data(format!("interval geom is missing its '{aesthetic}' column"))
        })?);
    }
    found
        .try_into()
        .map_err(|_| Error::data("interval geom columns".to_string()))
}

/// The `fatten` option, else `default`; must be finite and not negative.
fn fatten(parameters: &Parameters, default: f64) -> Result<f64> {
    let value = parameters.number("fatten")?.unwrap_or(default);
    if !value.is_finite() || value < 0.0 {
        return Err(Error::spec(
            parameters.path_of("fatten"),
            format!("fatten must be a number of 0 or more, got {value}"),
        ));
    }
    Ok(value)
}

/// The value of `row` when finite.
fn finite(column: &ContinuousColumn, row: usize) -> Option<f64> {
    column.get(row).filter(|value| value.is_finite())
}

/// Point range line: `linewidth` (not `size`) wide, faded by `alpha`.
fn range_stroke(styles: &Styles, row: usize) -> Result<Option<Stroke>> {
    let color = styles.colors(Aesthetic::Color)?.get(row);
    let alpha = styles.numbers(Aesthetic::Alpha)?.get(row);
    let width = styles.numbers(Aesthetic::Linewidth)?.get(row) * POINT_UNIT_SIZE;
    let linetype = styles.linetypes(Aesthetic::Linetype)?.get_ref(row);
    Ok(lines::stroke(color.with_opacity(alpha), width, linetype))
}

/// A stroke `fatten` times wider, dashes included (lets-plot `BoxHelper`).
fn fattened(stroke: Option<Stroke>, fatten: f64) -> Option<Stroke> {
    stroke.map(|stroke| Stroke {
        width: stroke.width * fatten,
        dash: stroke.dash.iter().map(|length| length * fatten).collect(),
        ..stroke
    })
}

/// Hover rectangles of line and point ranges: `width(row)` px (at least 2)
/// around `x`, from `ymin` to `ymax`.
fn range_rectangles(
    layer: &DrawLayer,
    panel: &Panel,
    width: impl Fn(usize) -> f64,
) -> Option<HoverTargets> {
    let [x_column, ymin, ymax] = [Aesthetic::X, Aesthetic::Ymin, Aesthetic::Ymax]
        .map(|aesthetic| layer.data.continuous(aesthetic));
    let (x_column, ymin, ymax) = (x_column?, ymin?, ymax?);
    let (mut x, mut y, mut widths, mut heights, mut rows) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for row in 0..layer.data.row_count() {
        let [Some(center), Some(y0), Some(y1)] =
            [x_column, ymin, ymax].map(|column| finite(column, row))
        else {
            continue;
        };
        let (left, bottom) = panel.map(center, y0);
        let (right, top) = panel.map(center, y1);
        if ![left, bottom, right, top]
            .iter()
            .all(|value| value.is_finite())
        {
            continue;
        }
        let side = width(row).max(MIN_HOVER_WIDTH);
        // Vertical in data space; `coord_flip` turns it horizontal on screen.
        let (pixel_width, pixel_height) = if left == right {
            (side, (bottom - top).abs().max(0.1))
        } else {
            ((right - left).abs().max(0.1), side)
        };
        x.push((left + right) / 2.0 - pixel_width / 2.0);
        y.push((top + bottom) / 2.0 - pixel_height / 2.0);
        widths.push(pixel_width);
        heights.push(pixel_height);
        rows.push(row as u32);
    }
    Some(HoverTargets::Rects {
        x,
        y,
        width: widths,
        height: heights,
        rows,
    })
}

/// A two-point line in data units; `None` when invisible or off the coord.
fn line(panel: &Panel, from: (f64, f64), to: (f64, f64), stroke: Option<Stroke>) -> Option<Node> {
    let stroke = stroke?;
    let (from, to) = (panel.map(from.0, from.1), panel.map(to.0, to.1));
    [from.0, from.1, to.0, to.1]
        .iter()
        .all(|value| value.is_finite())
        .then(|| pixel_line(from, to, stroke))
}

/// A two-point line in pixels.
fn pixel_line(from: (f64, f64), to: (f64, f64), stroke: Stroke) -> Node {
    Node::Path(Path {
        points: vec![from, to],
        closed: false,
        fill: None,
        stroke: Some(stroke),
    })
}
