//! `geom_abline`: one straight line `y = intercept + slope * x` per row, across
//! the whole panel, as lets-plot's `ABLineGeom`.
//!
//! `intercept` and `slope` are in axis units (after a scale transform) and
//! default to 0 and 1 (lets-plot `AesInitValue`). The line is cut where it
//! leaves the panel's expanded domain; lines that miss the panel are not drawn.
//! The layer does not train the axes.
//!
//! What it does not do: `coord_polar` (not in ggrust). Deviation: `geom_abline()`
//! without `intercept` or `slope` draws `y = x` as ggplot2 does (lets-plot draws
//! nothing).

use super::lines::{self, LINE_STYLE_DEFAULTS};
use super::{DrawLayer, Geom};
use crate::Result;
use crate::coord::Panel;
use crate::scene::{Node, Path, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};

/// Sloped reference lines.
pub struct AbLineGeom;

const STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    LINE_STYLE_DEFAULTS[0],
    LINE_STYLE_DEFAULTS[1],
    LINE_STYLE_DEFAULTS[2],
    LINE_STYLE_DEFAULTS[3],
    (Aesthetic::Intercept, StyleDefault::Number(0.0)),
    (Aesthetic::Slope, StyleDefault::Number(1.0)),
];

impl Geom for AbLineGeom {
    fn name(&self) -> &'static str {
        "abline"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        STYLE_DEFAULTS
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let intercepts = layer.styles.numbers(Aesthetic::Intercept)?;
        let slopes = layer.styles.numbers(Aesthetic::Slope)?;
        // Domains of data x and data y (swapped on screen by coord_flip).
        let (x_scale, y_scale) = match panel.flipped {
            false => (panel.x_scale, panel.y_scale),
            true => (panel.y_scale, panel.x_scale),
        };
        let x_domain = x_scale.expanded_domain();
        let y_domain = y_scale.expanded_domain();
        let mut nodes = Vec::new();
        for row in 0..layer.data.row_count() {
            let Some(((x0, y0), (x1, y1))) =
                clip_line(intercepts.get(row), slopes.get(row), x_domain, y_domain)
            else {
                continue;
            };
            let Some(stroke) = lines::stroke_at(layer.styles, row)? else {
                continue;
            };
            nodes.push(Node::Path(Path {
                points: vec![panel.map(x0, y0), panel.map(x1, y1)],
                closed: false,
                fill: None,
                stroke: Some(stroke),
            }));
        }
        Ok(nodes)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        lines::key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        // lets-plot uses the hline key.
        lines::key_line(styles, cell, false)
    }
}

/// The part of `y = intercept + slope * x` inside the box `x_domain` x
/// `y_domain`, as its two ends; `None` when the line misses the box or a value
/// is not finite.
fn clip_line(
    intercept: f64,
    slope: f64,
    x_domain: (f64, f64),
    y_domain: (f64, f64),
) -> Option<((f64, f64), (f64, f64))> {
    let finite = [
        intercept, slope, x_domain.0, x_domain.1, y_domain.0, y_domain.1,
    ];
    if !finite.iter().all(|value| value.is_finite()) {
        return None;
    }
    let (y_low, y_high) = (y_domain.0.min(y_domain.1), y_domain.0.max(y_domain.1));
    let (mut x_low, mut x_high) = (x_domain.0.min(x_domain.1), x_domain.0.max(x_domain.1));
    if slope == 0.0 {
        if intercept < y_low || intercept > y_high {
            return None;
        }
    } else {
        // x where the line crosses the bottom and top of the box.
        let at_low = (y_low - intercept) / slope;
        let at_high = (y_high - intercept) / slope;
        x_low = x_low.max(at_low.min(at_high));
        x_high = x_high.min(at_low.max(at_high));
        if x_low >= x_high {
            return None;
        }
    }
    let y = |x: f64| (intercept + slope * x).clamp(y_low, y_high);
    Some(((x_low, y(x_low)), (x_high, y(x_high))))
}

#[cfg(test)]
mod tests {
    use super::clip_line;

    #[test]
    fn clips_to_the_box() {
        let ends = clip_line(0.0, 1.0, (-1.0, 4.0), (0.5, 2.0));
        assert_eq!(ends, Some(((0.5, 0.5), (2.0, 2.0))));
        let flat = clip_line(1.0, 0.0, (0.0, 3.0), (0.0, 2.0));
        assert_eq!(flat, Some(((0.0, 1.0), (3.0, 1.0))));
        let falling = clip_line(2.0, -1.0, (0.0, 3.0), (0.0, 1.0));
        assert_eq!(falling, Some(((1.0, 1.0), (2.0, 0.0))));
    }

    #[test]
    fn lines_outside_the_box_are_dropped() {
        assert_eq!(clip_line(5.0, 0.0, (0.0, 1.0), (0.0, 1.0)), None);
        assert_eq!(clip_line(10.0, 1.0, (0.0, 1.0), (0.0, 1.0)), None);
        assert_eq!(clip_line(f64::NAN, 1.0, (0.0, 1.0), (0.0, 1.0)), None);
    }
}
