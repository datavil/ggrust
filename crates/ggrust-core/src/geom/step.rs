//! `geom_step`: connect the rows of each group, sorted by x, with stairs.
//!
//! Ported from lets-plot's `StepGeom` (`LinesHelper.createSteps`): `direction`
//! `"hv"` (default) moves along x first, `"vh"` along y first; `pad=True`
//! draws rows at x = -inf / +inf at the panel edges. `"mid"` steps halfway
//! between adjacent x values, as ggplot2's `stairstep` (lets-plot lacks it,
//! #838). Corners take the style of the row ggplot2 gives them, so varying
//! styles change where ggplot2 changes them.
//!
//! Supports `arrow=arrow(...)`. A transform or limits on x drop the infinite
//! rows before drawing (reported as missing).
//!
//! What it does not do (yet): `color_by`.

use super::lines::{self, LINE_STYLE_DEFAULTS};
use super::path::draw_paths;
use super::{DrawLayer, Geom};
use crate::coord::Panel;
use crate::parameters::Parameters;
use crate::scene::{Node, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};
use crate::{Error, Result};

/// Stairs through the rows sorted by x.
pub struct StepGeom;

/// Where a step turns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Along x, then y.
    HorizontalVertical,
    /// Along y, then x.
    VerticalHorizontal,
    /// Along x to halfway, along y, then along x (ggplot2 `"mid"`).
    Middle,
}

/// How [`draw_paths`] turns a run of rows into stairs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Steps {
    pub direction: Direction,
    /// Rows at x = -inf / +inf are drawn at the panel's x edges.
    pub padded: bool,
}

impl Steps {
    /// Read `direction` and `pad` from the layer options.
    pub fn parse(parameters: &Parameters) -> Result<Steps> {
        let direction = match parameters.string("direction")? {
            None => Direction::HorizontalVertical,
            Some(text) => match text.to_ascii_lowercase().as_str() {
                "hv" => Direction::HorizontalVertical,
                "vh" => Direction::VerticalHorizontal,
                "mid" => Direction::Middle,
                _ => {
                    return Err(Error::spec(
                        parameters.path_of("direction"),
                        format!(
                            "unknown step direction '{text}'; expected 'hv', 'vh' or 'mid'{}",
                            crate::suggest::did_you_mean(text, ["hv", "vh", "mid"])
                        ),
                    ));
                }
            },
        };
        let padded = parameters.boolean("pad")?.unwrap_or(false);
        Ok(Steps { direction, padded })
    }

    /// The x a row is drawn at: infinite x moves to the panel edge when padded.
    pub fn padded_x(&self, x: f64, panel: &Panel) -> f64 {
        if !self.padded || !x.is_infinite() {
            return x;
        }
        let scale = if panel.flipped {
            panel.y_scale
        } else {
            panel.x_scale
        };
        let (low, high) = scale.expanded_domain();
        if x < 0.0 { low } else { high }
    }

    /// Stair vertices of one run (data units), with the row each vertex takes
    /// its style from (ggplot2 `stairstep`).
    pub fn stairs(
        &self,
        rows: &[usize],
        locations: &[(f64, f64)],
    ) -> (Vec<usize>, Vec<(f64, f64)>) {
        let count = locations.len();
        if count < 2 {
            return (rows.to_vec(), locations.to_vec());
        }
        let mut vertex_rows = Vec::with_capacity(2 * count);
        let mut vertices = Vec::with_capacity(2 * count);
        vertex_rows.push(rows[0]);
        vertices.push(locations[0]);
        for index in 1..count {
            let (previous_x, previous_y) = locations[index - 1];
            let (x, y) = locations[index];
            match self.direction {
                Direction::HorizontalVertical => {
                    vertex_rows.push(rows[index]);
                    vertices.push((x, previous_y));
                }
                Direction::VerticalHorizontal => {
                    vertex_rows.push(rows[index - 1]);
                    vertices.push((previous_x, y));
                }
                Direction::Middle => {
                    let middle = previous_x + (x - previous_x) / 2.0;
                    vertex_rows.push(rows[index - 1]);
                    vertices.push((middle, previous_y));
                    vertex_rows.push(rows[index - 1]);
                    vertices.push((middle, y));
                    // ggplot2 skips the data points in between (collinear).
                    if index + 1 < count {
                        continue;
                    }
                }
            }
            vertex_rows.push(rows[index]);
            vertices.push((x, y));
        }
        (vertex_rows, vertices)
    }
}

impl Geom for StepGeom {
    fn tooltip_behavior(&self) -> crate::tooltip::TooltipBehavior {
        crate::tooltip::TooltipBehavior {
            marker_is_stroke: true,
            ..crate::tooltip::TooltipBehavior::ALONG_X
        }
    }

    fn hover_targets(
        &self,
        layer: &DrawLayer,
        panel: &Panel,
    ) -> Result<Option<crate::tooltip::HoverTargets>> {
        Ok(super::hover::vertices(layer, panel))
    }

    fn name(&self) -> &'static str {
        "step"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        LINE_STYLE_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["direction", "pad", "arrow"]
    }

    fn draws_infinite_x(&self, parameters: &Parameters) -> Result<bool> {
        Ok(parameters.boolean("pad")?.unwrap_or(false))
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let steps = Steps::parse(&layer.parameters)?;
        draw_paths(layer, panel, true, Some(steps))
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        lines::key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        lines::key_line(styles, cell, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stairs(direction: Direction, locations: &[(f64, f64)]) -> (Vec<usize>, Vec<(f64, f64)>) {
        let rows: Vec<usize> = (0..locations.len()).collect();
        Steps {
            direction,
            padded: false,
        }
        .stairs(&rows, locations)
    }

    const POINTS: [(f64, f64); 3] = [(0.0, 1.0), (2.0, 3.0), (3.0, 0.0)];

    #[test]
    fn horizontal_first() {
        let (rows, vertices) = stairs(Direction::HorizontalVertical, &POINTS);
        assert_eq!(
            vertices,
            vec![(0.0, 1.0), (2.0, 1.0), (2.0, 3.0), (3.0, 3.0), (3.0, 0.0)]
        );
        assert_eq!(rows, vec![0, 1, 1, 2, 2]);
    }

    #[test]
    fn vertical_first() {
        let (rows, vertices) = stairs(Direction::VerticalHorizontal, &POINTS);
        assert_eq!(
            vertices,
            vec![(0.0, 1.0), (0.0, 3.0), (2.0, 3.0), (2.0, 0.0), (3.0, 0.0)]
        );
        assert_eq!(rows, vec![0, 0, 1, 1, 2]);
    }

    #[test]
    fn middle_as_ggplot2() {
        // ggplot2 stairstep(direction = "mid").
        let (rows, vertices) = stairs(Direction::Middle, &POINTS);
        assert_eq!(
            vertices,
            vec![
                (0.0, 1.0),
                (1.0, 1.0),
                (1.0, 3.0),
                (2.5, 3.0),
                (2.5, 0.0),
                (3.0, 0.0)
            ]
        );
        assert_eq!(rows, vec![0, 0, 0, 1, 1, 2]);
    }

    #[test]
    fn single_row_unchanged() {
        let (_, vertices) = stairs(Direction::Middle, &[(1.0, 1.0)]);
        assert_eq!(vertices, vec![(1.0, 1.0)]);
    }
}
