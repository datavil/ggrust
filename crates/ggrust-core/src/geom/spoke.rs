//! `geom_spoke`: a segment of length `radius` at `angle` radians through each
//! (`x`, `y`), for vector fields (lets-plot `SpokeGeom`). `pivot` says which part of
//! the segment sits on the point: `tail`, `middle` (or `mid`) or `tip`. Lengths and
//! angles are in axis units, so a stretched panel stretches the spokes.
//!
//! The segment's ends train the axes through `xmin`/`xmax`/`ymin`/`ymax` spans.

use super::lines::{self, Arrow, LINE_STYLE_DEFAULTS};
use super::{DrawLayer, Geom, SegmentGeom};
use crate::coord::Panel;
use crate::data::{Column, ContinuousColumn, LayerData};
use crate::parameters::Parameters;
use crate::scene::{Node, Path, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};
use crate::{Error, Result};

fn pivot(parameters: &Parameters) -> Result<f64> {
    match parameters
        .string("pivot")?
        .map(str::to_lowercase)
        .as_deref()
    {
        None | Some("tail") => Ok(0.0),
        Some("middle" | "mid") => Ok(0.5),
        Some("tip") => Ok(1.0),
        Some(other) => Err(Error::spec(
            parameters.path_of("pivot"),
            format!("expected one of 'tail', 'middle', 'mid', 'tip', got '{other}'"),
        )),
    }
}

/// The ends of the spoke of a point, or `None` when a value is not finite.
fn ends(point: (f64, f64), angle: f64, radius: f64, pivot: f64) -> Option<[(f64, f64); 2]> {
    let (dx, dy) = (radius * angle.cos(), radius * angle.sin());
    let start = (point.0 - dx * pivot, point.1 - dy * pivot);
    let end = (point.0 + dx * (1.0 - pivot), point.1 + dy * (1.0 - pivot));
    [start.0, start.1, end.0, end.1]
        .iter()
        .all(|value| value.is_finite())
        .then_some([start, end])
}

/// A row's `angle` or `radius`: its column, else the layer's constant, else 0.
struct Length<'a> {
    column: Option<&'a ContinuousColumn>,
    constant: f64,
}

impl<'a> Length<'a> {
    fn of(data: &'a LayerData, parameters: &Parameters, aesthetic: Aesthetic) -> Result<Self> {
        let name = if aesthetic == Aesthetic::Angle {
            "angle"
        } else {
            "radius"
        };
        Ok(Length {
            column: data.continuous(aesthetic),
            constant: parameters.number(name)?.unwrap_or(0.0),
        })
    }

    fn at(&self, row: usize) -> f64 {
        match self.column {
            Some(column) => column.get(row).unwrap_or(f64::NAN),
            None => self.constant,
        }
    }
}

/// Spokes.
pub struct SpokeGeom;

impl Geom for SpokeGeom {
    fn name(&self) -> &'static str {
        "spoke"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn optional_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Angle, Aesthetic::Radius]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        LINE_STYLE_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["arrow", "pivot", "angle", "radius"]
    }

    fn setup_aesthetics(&self) -> &'static [Aesthetic] {
        &[
            Aesthetic::Xmin,
            Aesthetic::Xmax,
            Aesthetic::Ymin,
            Aesthetic::Ymax,
        ]
    }

    fn span_aesthetics(&self) -> &'static [Aesthetic] {
        self.setup_aesthetics()
    }

    fn setup(&self, mut data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        let pivot = pivot(parameters)?;
        let (Some(x), Some(y)) = (
            data.continuous(Aesthetic::X).cloned(),
            data.continuous(Aesthetic::Y).cloned(),
        ) else {
            return Ok(data);
        };
        let row_count = data.row_count();
        let mut columns = [const { Vec::new() }; 4];
        {
            let angle = Length::of(&data, parameters, Aesthetic::Angle)?;
            let radius = Length::of(&data, parameters, Aesthetic::Radius)?;
            for row in 0..row_count {
                let point = (
                    x.get(row).unwrap_or(f64::NAN),
                    y.get(row).unwrap_or(f64::NAN),
                );
                let [start, end] = ends(point, angle.at(row), radius.at(row), pivot)
                    .unwrap_or([(f64::NAN, f64::NAN); 2]);
                for (column, value) in columns.iter_mut().zip([start.0, end.0, start.1, end.1]) {
                    column.push(value);
                }
            }
        }
        let [xmin, xmax, ymin, ymax] = columns;
        for (aesthetic, values) in [
            (Aesthetic::Xmin, xmin),
            (Aesthetic::Xmax, xmax),
            (Aesthetic::Ymin, ymin),
            (Aesthetic::Ymax, ymax),
        ] {
            data.insert(
                aesthetic,
                Column::Continuous(ContinuousColumn::from_values(values)),
            )?;
        }
        Ok(data)
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let (Some(x), Some(y)) = (
            layer.data.continuous(Aesthetic::X),
            layer.data.continuous(Aesthetic::Y),
        ) else {
            return Ok(Vec::new());
        };
        let pivot = pivot(&layer.parameters)?;
        let angle = Length::of(layer.data, &layer.parameters, Aesthetic::Angle)?;
        let radius = Length::of(layer.data, &layer.parameters, Aesthetic::Radius)?;
        let arrow = Arrow::parse(&layer.parameters)?;
        let mut nodes = Vec::new();
        for row in 0..layer.data.row_count() {
            let point = (
                x.get(row).unwrap_or(f64::NAN),
                y.get(row).unwrap_or(f64::NAN),
            );
            let Some([start, end]) = ends(point, angle.at(row), radius.at(row), pivot) else {
                continue;
            };
            let Some(stroke) = lines::stroke_at(layer.styles, row)? else {
                continue;
            };
            let mut points = vec![panel.map(start.0, start.1), panel.map(end.0, end.1)];
            let heads = arrow
                .map(|arrow| arrow.apply(&mut points, &stroke))
                .unwrap_or_default();
            nodes.push(Node::Path(Path {
                points,
                closed: false,
                fill: None,
                stroke: Some(stroke),
            }));
            nodes.extend(heads);
        }
        Ok(nodes)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        SegmentGeom.minimum_key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        SegmentGeom.draw_key(styles, cell)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;

    fn close(left: (f64, f64), right: (f64, f64)) -> bool {
        (left.0 - right.0).abs() < 1e-9 && (left.1 - right.1).abs() < 1e-9
    }

    #[test]
    fn the_tail_sits_on_the_point() {
        let [start, end] = ends((1.0, 1.0), 0.0, 2.0, 0.0).unwrap();
        assert!(close(start, (1.0, 1.0)));
        assert!(close(end, (3.0, 1.0)));
    }

    #[test]
    fn the_middle_and_tip_move_the_segment_back() {
        let [start, end] = ends((0.0, 0.0), FRAC_PI_2, 2.0, 0.5).unwrap();
        assert!(close(start, (0.0, -1.0)));
        assert!(close(end, (0.0, 1.0)));
        let [start, end] = ends((0.0, 0.0), 0.0, 2.0, 1.0).unwrap();
        assert!(close(start, (-2.0, 0.0)));
        assert!(close(end, (0.0, 0.0)));
    }

    #[test]
    fn a_missing_value_draws_nothing() {
        assert!(ends((0.0, 0.0), f64::NAN, 1.0, 0.0).is_none());
    }
}
