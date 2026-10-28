//! `geom_path` and `geom_line`: connect the rows of each group in order (`line`
//! sorts them by x first).
//!
//! A missing value breaks the line. When color, size, alpha or linetype vary
//! along a group, each segment takes the style of its starting row. Supports
//! `arrow=arrow(...)`.
//!
//! What it does not do (yet): `color_by`, maps (`flat`, `geodesic`).

use super::lines::{self, Arrow, LINE_STYLE_DEFAULTS};
use super::{DrawLayer, Geom};
use crate::Result;
use crate::coord::Panel;
use crate::scene::{Node, Path, Rect, Stroke};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};

/// Lines in data order.
pub struct PathGeom;

/// Lines sorted by x within each group.
pub struct LineGeom;

impl Geom for PathGeom {
    /// lets-plot `bivariateFunction(area = true)`: hover near the path.
    fn tooltip_behavior(&self) -> crate::tooltip::TooltipBehavior {
        crate::tooltip::TooltipBehavior {
            lookup: Some(crate::tooltip::Lookup::Nearest),
            marker_is_stroke: true,
            ..crate::tooltip::TooltipBehavior::NONE
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
        "path"
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
        draw_paths(layer, panel, false)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        lines::key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        lines::key_line(styles, cell, false)
    }
}

impl Geom for LineGeom {
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
        "line"
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
        draw_paths(layer, panel, true)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        lines::key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        lines::key_line(styles, cell, false)
    }
}

fn draw_paths(layer: &DrawLayer, panel: &Panel, sort_by_x: bool) -> Result<Vec<Node>> {
    let (Some(x), Some(y)) = (
        layer.data.continuous(Aesthetic::X),
        layer.data.continuous(Aesthetic::Y),
    ) else {
        return Ok(Vec::new());
    };
    let arrow = Arrow::parse(&layer.parameters)?;
    let row_count = layer.data.row_count();
    let group_index = layer.data.group_index();
    let group_count = group_index
        .map(|groups| {
            groups
                .iter()
                .copied()
                .max()
                .map_or(0, |last| last as usize + 1)
        })
        .unwrap_or(1);
    let mut rows_by_group = vec![Vec::new(); group_count];
    for row in 0..row_count {
        let group = group_index.map_or(0, |groups| groups[row] as usize);
        rows_by_group[group].push(row);
    }
    let uniform = lines::is_uniform(layer.styles);
    let uniform_stroke = if uniform {
        lines::stroke_at(layer.styles, 0)?
    } else {
        None
    };
    let mut nodes = Vec::new();
    for mut rows in rows_by_group {
        if sort_by_x {
            rows.sort_by(|left, right| {
                let key = |row: usize| x.get(row).filter(|value| value.is_finite());
                // Missing x last, as a stable sort keeps data order for ties.
                match (key(*left), key(*right)) {
                    (Some(left), Some(right)) => left.total_cmp(&right),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            });
        }
        // A missing coordinate breaks the line: carry the row with the point.
        let mut run_rows: Vec<usize> = Vec::new();
        let mut run_points: Vec<(f64, f64)> = Vec::new();
        let flush = |run_rows: &mut Vec<usize>,
                     run_points: &mut Vec<(f64, f64)>,
                     nodes: &mut Vec<Node>|
         -> Result<()> {
            if run_points.len() >= 2 {
                if uniform {
                    if let Some(stroke) = &uniform_stroke {
                        let mut points = std::mem::take(run_points);
                        let heads = arrow
                            .map(|arrow| arrow.apply(&mut points, stroke))
                            .unwrap_or_default();
                        nodes.push(Node::Path(Path {
                            points,
                            closed: false,
                            fill: None,
                            stroke: Some(stroke.clone()),
                        }));
                        nodes.extend(heads);
                    }
                } else {
                    let strokes = run_rows
                        .iter()
                        .map(|&row| lines::stroke_at(layer.styles, row))
                        .collect::<Result<Vec<Option<Stroke>>>>()?;
                    let pieces = lines::styled_paths(run_points, &strokes);
                    let piece_count = pieces.len();
                    for (index, mut path) in pieces.into_iter().enumerate() {
                        if let (Some(arrow), Some(stroke)) = (arrow, path.stroke.clone()) {
                            // Heads only at the outer ends of the whole line.
                            let at_start = index == 0 && path.points[0] == run_points[0];
                            let at_end =
                                index + 1 == piece_count && path.points.last() == run_points.last();
                            let heads = arrow.apply_at(&mut path.points, &stroke, at_start, at_end);
                            nodes.push(Node::Path(path));
                            nodes.extend(heads);
                        } else {
                            nodes.push(Node::Path(path));
                        }
                    }
                }
            }
            run_rows.clear();
            run_points.clear();
            Ok(())
        };
        for row in rows {
            let (px, py) = panel.map(
                x.get(row).unwrap_or(f64::NAN),
                y.get(row).unwrap_or(f64::NAN),
            );
            if px.is_finite() && py.is_finite() {
                run_rows.push(row);
                run_points.push((px, py));
            } else {
                flush(&mut run_rows, &mut run_points, &mut nodes)?;
            }
        }
        flush(&mut run_rows, &mut run_points, &mut nodes)?;
    }
    Ok(nodes)
}
