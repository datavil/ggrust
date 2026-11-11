//! `geom_segment`: a straight line from (`x`, `y`) to (`xend`, `yend`) per row,
//! optionally with `arrow=arrow(...)`. The ends stop short of the points they
//! connect by `size_start`/`size_end`, `stroke_start`/`stroke_end` and `spacer`
//! (see `end_padding`), as lets-plot.
//!
//! What it does not do (yet): `flat`/`geodesic`.

use super::end_padding::{self, PADDED_LINE_STYLE_DEFAULTS};
use super::lines::{self, Arrow};
use super::{DrawLayer, Geom};
use crate::Result;
use crate::coord::Panel;
use crate::scene::{Node, Path, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};

/// Line segments.
pub struct SegmentGeom;

impl Geom for SegmentGeom {
    fn name(&self) -> &'static str {
        "segment"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y, Aesthetic::Xend, Aesthetic::Yend]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        PADDED_LINE_STYLE_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["arrow", "spacer", "lineend", "linejoin"]
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let (Some(x), Some(y), Some(x_end), Some(y_end)) = (
            layer.data.continuous(Aesthetic::X),
            layer.data.continuous(Aesthetic::Y),
            layer.data.continuous(Aesthetic::Xend),
            layer.data.continuous(Aesthetic::Yend),
        ) else {
            return Ok(Vec::new());
        };
        let arrow = Arrow::parse(&layer.parameters)?;
        let spacer = end_padding::parse_spacer(&layer.parameters)?;
        let mut nodes = Vec::new();
        for row in 0..layer.data.row_count() {
            let value =
                |column: &crate::data::ContinuousColumn| column.get(row).unwrap_or(f64::NAN);
            let start = panel.map(value(x), value(y));
            let end = panel.map(value(x_end), value(y_end));
            let finite = [start.0, start.1, end.0, end.1]
                .iter()
                .all(|coordinate| coordinate.is_finite());
            let Some(stroke) = lines::stroke_at(layer.styles, row)?.filter(|_| finite) else {
                continue;
            };
            let (pad_start, pad_end) = end_padding::end_padding(layer.styles, row, spacer)?;
            let mut points = end_padding::pad_ends(&[start, end], pad_start, pad_end);
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
        lines::with_line_style(nodes, &layer.parameters)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        lines::key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        lines::key_line(styles, cell, false)
    }
}
