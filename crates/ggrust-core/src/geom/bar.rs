//! `geom_bar` and `geom_histogram`: one rectangle per row, from 0 to y.
//!
//! Setup (before the position adjustment) turns x into `xmin`/`xmax` spans of
//! `width` x the x resolution (lets-plot `BarGeom.getBinSpanCalculator`; a
//! histogram with `breaks` spans the bin between them) and y into `ymin` = 0,
//! `ymax` = y, so stacking and dodging move whole rectangles.
//!
//! Drawing follows lets-plot: rows in reverse order, zero-width or zero-height
//! rectangles widened to 0.1 px, `alpha` on the fill only.
//!
//! What it does not do (yet): `orientation="y"`, mapped `width`, bar labels.

use super::lines;
use super::point::POINT_UNIT_SIZE;
use super::{DrawLayer, Geom};
use crate::coord::Panel;
use crate::data::{Column, ContinuousColumn, LayerData, resolution};
use crate::parameters::Parameters;
use crate::position::DefaultPosition;
use crate::scene::{Color, Node, PerItem, Rect, Rects, Stroke};
use crate::spec::{Aesthetic, Value};
use crate::stat::bracketing_index;
use crate::style::{StyleDefault, Styles};
use crate::{Error, Result};

/// lets-plot bar style: blue fill, white outline (the paper color), size 0.75.
const BAR_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (Aesthetic::Color, StyleDefault::Color(Color::WHITE)),
    (
        Aesthetic::Fill,
        StyleDefault::Color(Color::rgb(0x11, 0x8e, 0xd8)),
    ),
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Size, StyleDefault::Number(0.75)),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
];

/// lets-plot histogram style: fill and outline in the pen color, thin outline.
const HISTOGRAM_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (
        Aesthetic::Color,
        StyleDefault::Color(Color::rgb(0x47, 0x47, 0x47)),
    ),
    (
        Aesthetic::Fill,
        StyleDefault::Color(Color::rgb(0x47, 0x47, 0x47)),
    ),
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Size, StyleDefault::Number(0.5)),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
];

/// Rectangle corners made by setup.
const RECTANGLE_AESTHETICS: &[Aesthetic] = &[
    Aesthetic::Xmin,
    Aesthetic::Xmax,
    Aesthetic::Ymin,
    Aesthetic::Ymax,
];

/// Bars, by default of row counts per x.
pub struct BarGeom;

impl Geom for BarGeom {
    fn name(&self) -> &'static str {
        "bar"
    }

    fn default_stat(&self) -> &'static str {
        "count"
    }

    fn default_position(&self) -> DefaultPosition {
        DefaultPosition::named("stack")
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        BAR_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["width"]
    }

    fn includes_zero_y(&self) -> bool {
        true
    }

    fn setup_aesthetics(&self) -> &'static [Aesthetic] {
        RECTANGLE_AESTHETICS
    }

    fn setup(&self, data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        let width = parameters.number("width")?.unwrap_or(0.9);
        rectangles(data, width, &[])
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        draw_rectangles(layer, panel)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        draw_key(styles, cell)
    }
}

/// Histogram bars over `stat="bin"`.
pub struct HistogramGeom;

impl Geom for HistogramGeom {
    fn name(&self) -> &'static str {
        "histogram"
    }

    fn default_stat(&self) -> &'static str {
        "bin"
    }

    fn default_position(&self) -> DefaultPosition {
        DefaultPosition::named("stack")
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        HISTOGRAM_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["width", "breaks"]
    }

    fn includes_zero_y(&self) -> bool {
        true
    }

    fn setup_aesthetics(&self) -> &'static [Aesthetic] {
        RECTANGLE_AESTHETICS
    }

    fn setup(&self, data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        let width = parameters.number("width")?.unwrap_or(1.0);
        rectangles(data, width, &histogram_breaks(parameters)?)
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        draw_rectangles(layer, panel)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        draw_key(styles, cell)
    }
}

/// `breaks=[...]` of a histogram: finite, distinct, sorted (lets-plot
/// `HistogramGeom.setBreaks`). Non-numbers are reported by the stat.
fn histogram_breaks(parameters: &Parameters) -> Result<Vec<f64>> {
    let mut breaks: Vec<f64> = match parameters.value("breaks") {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_f64)
            .filter(|value| value.is_finite())
            .collect(),
        _ => Vec::new(),
    };
    breaks.sort_by(f64::total_cmp);
    breaks.dedup();
    Ok(breaks)
}

/// Add `xmin`/`xmax` (x spans), `ymin` = 0, `ymax` = y and the `width` used.
fn rectangles(mut data: LayerData, width: f64, breaks: &[f64]) -> Result<LayerData> {
    let (Some(x), Some(y)) = (
        data.continuous(Aesthetic::X).cloned(),
        data.continuous(Aesthetic::Y).cloned(),
    ) else {
        return Ok(data);
    };
    let resolution = resolution(&x);
    let mut left = Vec::with_capacity(x.len());
    let mut right = Vec::with_capacity(x.len());
    for row in 0..x.len() {
        let location = x.get(row).unwrap_or(f64::NAN);
        let (to_left, to_right) = if breaks.is_empty() {
            (resolution / 2.0, resolution / 2.0)
        } else {
            match bracketing_index(breaks, location) {
                Some(index) => (location - breaks[index], breaks[index + 1] - location),
                None => (f64::NAN, f64::NAN),
            }
        };
        left.push(location - width * to_left);
        right.push(location + width * to_right);
    }
    let row_count = data.row_count();
    let continuous = |values: Vec<f64>| Column::Continuous(ContinuousColumn::from_values(values));
    data.insert(Aesthetic::Xmin, continuous(left))?;
    data.insert(Aesthetic::Xmax, continuous(right))?;
    data.insert(Aesthetic::Ymin, continuous(vec![0.0; row_count]))?;
    data.insert(Aesthetic::Ymax, Column::Continuous(y))?;
    data.insert(Aesthetic::Width, continuous(vec![width; row_count]))?;
    Ok(data)
}

/// Consecutive bars sharing an outline: one `Rects` node.
type Batch = (Option<Stroke>, Vec<Rect>, Vec<Option<Color>>);

fn draw_rectangles(layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
    let columns = [
        Aesthetic::Xmin,
        Aesthetic::Xmax,
        Aesthetic::Ymin,
        Aesthetic::Ymax,
    ]
    .map(|aesthetic| layer.data.continuous(aesthetic));
    let [Some(xmin), Some(xmax), Some(ymin), Some(ymax)] = columns else {
        return Err(Error::InvalidData(
            "bar rectangles were not set up (missing xmin, xmax, ymin or ymax)".to_owned(),
        ));
    };
    let fills = layer.styles.colors(Aesthetic::Fill)?;
    let alphas = layer.styles.numbers(Aesthetic::Alpha)?;
    let mut nodes: Vec<Node> = Vec::new();
    let mut batch: Option<Batch> = None;
    // lets-plot draws bars in reverse order (the first row ends up on top).
    for row in (0..layer.data.row_count()).rev() {
        let corners = [xmin.get(row), ymin.get(row), xmax.get(row), ymax.get(row)];
        let [Some(x0), Some(y0), Some(x1), Some(y1)] = corners else {
            continue;
        };
        let (left, top) = panel.map(x0, y0);
        let (right, bottom) = panel.map(x1, y1);
        if ![left, top, right, bottom]
            .iter()
            .all(|value| value.is_finite())
        {
            continue;
        }
        let mut rect = Rect {
            x: left.min(right),
            y: top.min(bottom),
            width: (right - left).abs(),
            height: (bottom - top).abs(),
        };
        // lets-plot `GeomHelper.toClient`: keep empty bars visible.
        if rect.width == 0.0 {
            rect.width = 0.1;
        }
        if rect.height == 0.0 {
            rect.height = 0.1;
        }
        let fill = fills.get(row).with_opacity(alphas.get(row));
        let stroke = lines::stroke_with_alpha(layer.styles, row, false)?;
        match &mut batch {
            Some((known, rects, fills)) if *known == stroke => {
                rects.push(rect);
                fills.push((fill.alpha > 0).then_some(fill));
            }
            _ => {
                if let Some(finished) = batch.take() {
                    nodes.push(rects_node(finished));
                }
                batch = Some((stroke, vec![rect], vec![(fill.alpha > 0).then_some(fill)]));
            }
        }
    }
    if let Some(finished) = batch {
        nodes.push(rects_node(finished));
    }
    Ok(nodes)
}

fn rects_node((stroke, rects, fills): Batch) -> Node {
    let fill = match fills.first() {
        Some(first) if fills.iter().all(|fill| fill == first) => PerItem::Uniform(*first),
        _ => PerItem::Varying(fills),
    };
    Node::Rects(Rects {
        rects,
        fill,
        stroke,
    })
}

fn key_size(styles: &Styles) -> Result<(f64, f64)> {
    let side = styles.numbers(Aesthetic::Size)?.get(0).max(0.0) * POINT_UNIT_SIZE + 2.0;
    Ok((side, side))
}

/// lets-plot's rect key: the cell inset by a pixel (so neighbouring keys do not
/// touch) and half the outline width.
fn draw_key(styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
    let alpha = styles.numbers(Aesthetic::Alpha)?.get(0);
    let fill = styles.colors(Aesthetic::Fill)?.get(0).with_opacity(alpha);
    let stroke = lines::stroke_with_alpha(styles, 0, false)?;
    let inset = 1.0 + stroke.as_ref().map_or(0.0, |stroke| stroke.width / 2.0);
    Ok(vec![Node::Rects(Rects {
        rects: vec![Rect {
            x: cell.x + inset,
            y: cell.y + inset,
            width: (cell.width - 2.0 * inset).max(0.0),
            height: (cell.height - 2.0 * inset).max(0.0),
        }],
        fill: PerItem::Uniform((fill.alpha > 0).then_some(fill)),
        stroke,
    })])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_use_resolution_or_breaks() {
        let mut data = LayerData::new(3);
        let column = |values: Vec<f64>| Column::Continuous(ContinuousColumn::from_values(values));
        data.insert(Aesthetic::X, column(vec![0.0, 2.0, 3.0]))
            .unwrap();
        data.insert(Aesthetic::Y, column(vec![1.0, -2.0, 3.0]))
            .unwrap();
        let bars = rectangles(data.clone(), 0.9, &[]).unwrap();
        let xmin = bars.continuous(Aesthetic::Xmin).unwrap();
        assert!((xmin.get(0).unwrap() + 0.45).abs() < 1e-12);
        assert_eq!(bars.continuous(Aesthetic::Ymax).unwrap().get(1), Some(-2.0));
        let bins = rectangles(data, 1.0, &[-1.0, 1.0, 2.5, 4.0]).unwrap();
        let (xmin, xmax) = (
            bins.continuous(Aesthetic::Xmin).unwrap(),
            bins.continuous(Aesthetic::Xmax).unwrap(),
        );
        assert_eq!((xmin.get(1), xmax.get(1)), (Some(1.0), Some(2.5)));
        assert_eq!((xmin.get(2), xmax.get(2)), (Some(2.5), Some(4.0)));
    }
}
