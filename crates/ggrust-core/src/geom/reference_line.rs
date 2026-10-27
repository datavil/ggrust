//! `geom_vline` (`xintercept`) and `geom_hline` (`yintercept`): a line across the
//! whole panel per row.
//!
//! What it does not do (yet): a list of intercepts in one option
//! (`xintercept=[1, 2]`); map them from a column or draw one layer each.

use super::lines::{self, LINE_STYLE_DEFAULTS};
use super::{DrawLayer, Geom};
use crate::Result;
use crate::coord::Panel;
use crate::scene::{Node, Path, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};

/// Vertical reference lines.
pub struct VerticalLineGeom;

/// Horizontal reference lines.
pub struct HorizontalLineGeom;

impl Geom for VerticalLineGeom {
    fn name(&self) -> &'static str {
        "vline"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Xintercept]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        LINE_STYLE_DEFAULTS
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        draw_lines(layer, panel, Aesthetic::Xintercept, true)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        lines::key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        lines::key_line(styles, cell, true)
    }
}

impl Geom for HorizontalLineGeom {
    fn name(&self) -> &'static str {
        "hline"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Yintercept]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        LINE_STYLE_DEFAULTS
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        draw_lines(layer, panel, Aesthetic::Yintercept, false)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        lines::key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        lines::key_line(styles, cell, false)
    }
}

fn draw_lines(
    layer: &DrawLayer,
    panel: &Panel,
    intercept: Aesthetic,
    vertical: bool,
) -> Result<Vec<Node>> {
    let Some(values) = layer.data.continuous(intercept) else {
        return Ok(Vec::new());
    };
    let rect = panel.rect;
    let mut nodes = Vec::new();
    for row in 0..layer.data.row_count() {
        let value = values.get(row).unwrap_or(f64::NAN);
        let position = if vertical {
            panel.x_pixel(value)
        } else {
            panel.y_pixel(value)
        };
        let Some(stroke) = lines::stroke_at(layer.styles, row)?.filter(|_| position.is_finite())
        else {
            continue;
        };
        let points = if vertical {
            vec![(position, rect.y + rect.height), (position, rect.y)]
        } else {
            vec![(rect.x, position), (rect.x + rect.width, position)]
        };
        nodes.push(Node::Path(Path {
            points,
            closed: false,
            fill: None,
            stroke: Some(stroke),
        }));
    }
    Ok(nodes)
}
