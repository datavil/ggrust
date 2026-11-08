//! `geom_streamline` (ggrust extension; R: metR's `geom_streamline`): the lines
//! of `stat="streamline"`, drawn as `geom_path`, with one arrowhead per line
//! where matplotlib's `streamplot` puts it (half way along the line, pointing
//! along the flow; [`crate::streamplot::Streamline::arrow`]).
//!
//! `arrow=arrow(...)` styles the heads (null: no heads); `ends="first"` or
//! `"both"` turn or double the head at that spot. Heads take the line's color
//! and width at the arrow's starting row.
//!
//! What it does not do: heads at the line ends (use `geom_path(arrow=...)` over
//! `stat="streamline"` for those), or several heads per line.

use super::lines::{self, Arrow, LINE_STYLE_DEFAULTS};
use super::path::draw_paths_with;
use super::{DrawLayer, Geom};
use crate::Result;
use crate::coord::Panel;
use crate::scene::{Node, Rect};
use crate::spec::Aesthetic;
use crate::streamplot::Streamline;
use crate::style::{StyleDefault, Styles};

/// Streamlines with mid-line arrowheads.
pub struct StreamlineGeom;

impl Geom for StreamlineGeom {
    fn name(&self) -> &'static str {
        "streamline"
    }

    fn default_stat(&self) -> &'static str {
        "streamline"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        LINE_STYLE_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["arrow"]
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let mut nodes = draw_paths_with(layer, panel, false, None, None)?;
        if let Some(arrow) = Arrow::parse(&layer.parameters)? {
            nodes.extend(heads(layer, panel, arrow)?);
        }
        Ok(nodes)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        lines::key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        lines::key_line(styles, cell, false)
    }
}

/// One head per line (a group's run of finite rows), at matplotlib's arrow of
/// its vertices in data units, as matplotlib measures the arc length.
fn heads(layer: &DrawLayer, panel: &Panel, arrow: Arrow) -> Result<Vec<Node>> {
    let (Some(x), Some(y)) = (
        layer.data.continuous(Aesthetic::X),
        layer.data.continuous(Aesthetic::Y),
    ) else {
        return Ok(Vec::new());
    };
    let mut nodes = Vec::new();
    let mut add_head = |rows: &[usize]| -> Result<()> {
        let line = Streamline {
            x: rows
                .iter()
                .map(|&row| x.get(row).unwrap_or(f64::NAN))
                .collect(),
            y: rows
                .iter()
                .map(|&row| y.get(row).unwrap_or(f64::NAN))
                .collect(),
        };
        let Some(head) = line.arrow() else {
            return Ok(());
        };
        // The row the arrow starts at carries the head's style.
        let start = line
            .x
            .iter()
            .zip(&line.y)
            .position(|point| point == (&head.x, &head.y))
            .unwrap_or(0);
        let Some(stroke) = lines::stroke_at(layer.styles, rows[start])? else {
            return Ok(());
        };
        let mut points = vec![panel.map(head.x, head.y), panel.map(head.x_end, head.y_end)];
        if points
            .iter()
            .all(|(px, py)| px.is_finite() && py.is_finite())
        {
            nodes.extend(arrow.apply(&mut points, &stroke));
        }
        Ok(())
    };
    let group_index = layer.data.group_index();
    let group_count = group_index
        .and_then(|groups| groups.iter().max())
        .map_or(1, |last| *last as usize + 1);
    let mut rows_by_group = vec![Vec::new(); group_count];
    for row in 0..layer.data.row_count() {
        rows_by_group[group_index.map_or(0, |groups| groups[row] as usize)].push(row);
    }
    for rows in rows_by_group {
        // A missing vertex ends the line, as geom_path breaks it there.
        for run in rows.split(|&row| {
            !(x.get(row).is_some_and(f64::is_finite) && y.get(row).is_some_and(f64::is_finite))
        }) {
            add_head(run)?;
        }
    }
    Ok(nodes)
}
