//! `geom_lollipop`: a stick from a baseline to a marker at (`x`, `y`) per row
//! (lets-plot `LollipopGeom`). The baseline is `y = slope * x + intercept`; `dir` says
//! how sticks reach it: `v` vertical, `h` horizontal, `s` perpendicular. Longer
//! lollipops draw first, so short ones stay visible where they cross.
//!
//! The base trains the axes through `xmin`/`ymin` spans.
//!
//! What it does not do: dodge the base separately from the head (positions move the
//! head and the base follows from it).

use super::point;
use super::{DrawLayer, Geom, interval};
use crate::coord::Panel;
use crate::data::{Column, ContinuousColumn, LayerData};
use crate::parameters::Parameters;
use crate::scene::{Node, Path, Rect, Stroke};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};
use crate::tooltip::{HoverTargets, TooltipBehavior};
use crate::{Error, Result};

/// lets-plot `LollipopGeom.DEF_FATTEN`.
const FATTEN: f64 = 2.5;
/// lets-plot `LollipopLegendKeyElementFactory.stickLength`, in pixels.
const KEY_STICK: f64 = 5.0;

/// lets-plot `AestheticsDefaults.lollipop`: a hollow circle of size 2 (fattened).
const STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (Aesthetic::Size, StyleDefault::Number(2.0)),
    (Aesthetic::Stroke, StyleDefault::Number(0.75)),
    (Aesthetic::Linewidth, StyleDefault::Number(0.75)),
    (Aesthetic::Color, StyleDefault::PEN),
    (Aesthetic::Fill, StyleDefault::PAPER),
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Shape, StyleDefault::Shape(1)),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
];

/// lets-plot point legend keys use size 5 unless `size` is mapped to that legend.
const LEGEND_DEFAULTS: &[(Aesthetic, StyleDefault)] =
    &[(Aesthetic::Size, StyleDefault::Number(5.0))];

/// How sticks reach the baseline.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Direction {
    Vertical,
    Horizontal,
    Perpendicular,
}

/// A baseline and the direction of the sticks.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Baseline {
    slope: f64,
    intercept: f64,
    direction: Direction,
}

impl Baseline {
    fn parse(parameters: &Parameters) -> Result<Baseline> {
        let direction = match parameters.string("dir")?.map(str::to_lowercase).as_deref() {
            None | Some("v") => Direction::Vertical,
            Some("h") => Direction::Horizontal,
            Some("s") => Direction::Perpendicular,
            Some(other) => {
                return Err(Error::spec(
                    parameters.path_of("dir"),
                    format!("expected one of 'v', 'h', 's', got '{other}'"),
                ));
            }
        };
        Ok(Baseline {
            slope: parameters.number("slope")?.unwrap_or(0.0),
            intercept: parameters.number("intercept")?.unwrap_or(0.0),
            direction,
        })
    }

    /// Where the stick of a head at (`x`, `y`) meets the baseline.
    fn base(&self, x: f64, y: f64) -> (f64, f64) {
        let Baseline {
            slope, intercept, ..
        } = *self;
        match self.direction {
            Direction::Vertical => (x, slope * x + intercept),
            Direction::Horizontal if slope == 0.0 => (intercept, y),
            Direction::Horizontal => ((y - intercept) / slope, y),
            Direction::Perpendicular => {
                let base_x = (x + slope * (y - intercept)) / (1.0 + slope * slope);
                (base_x, slope * base_x + intercept)
            }
        }
    }
}

fn fatten(parameters: &Parameters) -> Result<f64> {
    Ok(parameters.number("fatten")?.unwrap_or(FATTEN))
}

/// Lollipops.
pub struct LollipopGeom;

impl Geom for LollipopGeom {
    fn name(&self) -> &'static str {
        "lollipop"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        STYLE_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["dir", "fatten", "slope", "intercept"]
    }

    /// lets-plot `rangeIncludesZero`: pinned when the baseline is the axis and the
    /// sticks are not parallel to it.
    fn includes_zero_with(&self, parameters: &Parameters) -> Result<bool> {
        let baseline = Baseline::parse(parameters)?;
        Ok(baseline.slope == 0.0
            && baseline.intercept == 0.0
            && baseline.direction != Direction::Horizontal)
    }

    fn setup_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Xmin, Aesthetic::Ymin]
    }

    fn span_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Xmin, Aesthetic::Ymin]
    }

    fn setup(&self, mut data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        let baseline = Baseline::parse(parameters)?;
        let (Some(x), Some(y)) = (
            data.continuous(Aesthetic::X).cloned(),
            data.continuous(Aesthetic::Y).cloned(),
        ) else {
            return Ok(data);
        };
        let (base_x, base_y) = (0..data.row_count())
            .map(|row| {
                baseline.base(
                    x.get(row).unwrap_or(f64::NAN),
                    y.get(row).unwrap_or(f64::NAN),
                )
            })
            .unzip();
        let continuous = |values| Column::Continuous(ContinuousColumn::from_values(values));
        data.insert(Aesthetic::Xmin, continuous(base_x))?;
        data.insert(Aesthetic::Ymin, continuous(base_y))?;
        Ok(data)
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let (Some(x), Some(y)) = (
            layer.data.continuous(Aesthetic::X),
            layer.data.continuous(Aesthetic::Y),
        ) else {
            return Ok(Vec::new());
        };
        let baseline = Baseline::parse(&layer.parameters)?;
        let fatten = fatten(&layer.parameters)?;
        let row_count = layer.data.row_count();
        let mut heads = vec![(f64::NAN, f64::NAN); row_count];
        let mut order = Vec::with_capacity(row_count);
        for (row, head) in heads.iter_mut().enumerate() {
            let (Some(head_x), Some(head_y)) = (x.get(row), y.get(row)) else {
                continue;
            };
            if !(head_x.is_finite() && head_y.is_finite()) {
                continue;
            }
            *head = (head_x, head_y);
            let (base_x, base_y) = baseline.base(head_x, head_y);
            order.push((row, (head_x - base_x).hypot(head_y - base_y)));
        }
        order.sort_by(|left, right| right.1.total_cmp(&left.1));
        let mut candies = point::markers(layer.styles, row_count, false, fatten)?;
        (candies.x, candies.y) = heads
            .iter()
            .map(|&(head_x, head_y)| panel.map(head_x, head_y))
            .unzip();
        let mut nodes = Vec::with_capacity(order.len() * 2);
        for (row, _) in order {
            let (head_x, head_y) = heads[row];
            let (base_x, base_y) = baseline.base(head_x, head_y);
            let base = panel.map(base_x, base_y);
            let head = (candies.x[row], candies.y[row]);
            let length = (head.0 - base.0).hypot(head.1 - base.1);
            let radius = point::stick_radius(layer.styles, row, fatten)?;
            if radius <= length
                && length > 0.0
                && base.0.is_finite()
                && base.1.is_finite()
                && let Some(stroke) = interval::range_stroke(layer.styles, row)?
            {
                let share = radius / length;
                let neck = (
                    head.0 + (base.0 - head.0) * share,
                    head.1 + (base.1 - head.1) * share,
                );
                nodes.push(Node::Path(Path {
                    points: vec![base, neck],
                    closed: false,
                    fill: None,
                    stroke: Some(stroke),
                }));
            }
            nodes.push(Node::Points(candies.slice(row..row + 1)));
        }
        Ok(nodes)
    }

    fn tooltip_behavior(&self) -> TooltipBehavior {
        TooltipBehavior::NEAREST_XY
    }

    fn hover_targets(&self, layer: &DrawLayer, panel: &Panel) -> Result<Option<HoverTargets>> {
        let fatten = fatten(&layer.parameters)?;
        Ok(super::hover::circles(layer, panel, |row| {
            point::marker_extent(layer.styles, row, fatten)
        }))
    }

    fn legend_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        LEGEND_DEFAULTS
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        let side = point::marker_extent(styles, 0, 1.0) * 2.0 + 2.0;
        Ok((side, side))
    }

    /// lets-plot `LollipopLegendKeyElementFactory`: an unfattened marker above a
    /// 5 px stick.
    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        let x = cell.x + cell.width / 2.0;
        let y = cell.y + (cell.height - KEY_STICK) / 2.0;
        let mut points = point::markers(styles, 1, true, 1.0)?;
        points.x = vec![x];
        points.y = vec![y];
        let mut nodes = vec![Node::Points(points)];
        // lets-plot decorates the key's stick without `applyAlphaToAll`.
        let color = styles.colors(Aesthetic::Color)?.get(0);
        if let Some(stroke) =
            interval::range_stroke(styles, 0)?.map(|stroke| Stroke { color, ..stroke })
        {
            let top = y + point::stick_radius(styles, 0, 1.0)?;
            nodes.push(Node::Path(Path {
                points: vec![(x, top), (x, top + KEY_STICK)],
                closed: false,
                fill: None,
                stroke: Some(stroke),
            }));
        }
        Ok(nodes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline(slope: f64, intercept: f64, direction: Direction) -> Baseline {
        Baseline {
            slope,
            intercept,
            direction,
        }
    }

    #[test]
    fn vertical_sticks_drop_onto_the_line() {
        let line = baseline(2.0, 1.0, Direction::Vertical);
        assert_eq!(line.base(3.0, 20.0), (3.0, 7.0));
    }

    #[test]
    fn horizontal_sticks_reach_the_line_or_the_intercept() {
        assert_eq!(
            baseline(2.0, 1.0, Direction::Horizontal).base(9.0, 5.0),
            (2.0, 5.0)
        );
        assert_eq!(
            baseline(0.0, 4.0, Direction::Horizontal).base(9.0, 5.0),
            (4.0, 5.0)
        );
    }

    #[test]
    fn perpendicular_sticks_meet_the_line_at_a_right_angle() {
        let line = baseline(1.0, 0.0, Direction::Perpendicular);
        assert_eq!(line.base(0.0, 2.0), (1.0, 1.0));
    }
}
