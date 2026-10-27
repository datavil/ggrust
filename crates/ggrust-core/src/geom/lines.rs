//! Shared by the line-like geoms (`path`, `line`, `segment`, `hline`, `vline`,
//! `polygon`): stroke styles from `color`, `size`, `alpha`, `linetype`, runs of
//! finite points, and arrow heads.
//!
//! Follows lets-plot: line width = `size` x 2.2 px, dashes are in units of line
//! width, `alpha` replaces the alpha of the color.
//!
//! What it does not do: caps, joins or miter limits (renderers use their defaults).

use super::point::POINT_UNIT_SIZE;
use crate::Error;
use crate::Result;
use crate::parameters::Parameters;
use crate::scene::{Color, Node, Path, Rect, Stroke};
use crate::spec::{Aesthetic, Value};
use crate::style::{StyleDefault, Styles};

/// Style defaults of line geoms (lets-plot: color `#474747`, size 0.75, solid).
pub(super) const LINE_STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (
        Aesthetic::Color,
        StyleDefault::Color(Color::rgb(0x47, 0x47, 0x47)),
    ),
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Size, StyleDefault::Number(0.75)),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
];

/// Whether every stroke aesthetic has one value for all rows.
pub(super) fn is_uniform(styles: &Styles) -> bool {
    [
        Aesthetic::Color,
        Aesthetic::Alpha,
        Aesthetic::Size,
        Aesthetic::Linetype,
    ]
    .iter()
    .all(|aesthetic| styles.is_uniform(*aesthetic))
}

/// The line style of one row; `None` when nothing is visible (width 0, blank
/// linetype, alpha 0).
pub(super) fn stroke_at(styles: &Styles, row: usize) -> Result<Option<Stroke>> {
    stroke_with_alpha(styles, row, true)
}

/// [`stroke_at`], with `alpha` optionally left out of the outline (polygons
/// apply it to the fill only).
pub(super) fn stroke_with_alpha(
    styles: &Styles,
    row: usize,
    use_alpha: bool,
) -> Result<Option<Stroke>> {
    let color = styles.colors(Aesthetic::Color)?.get(row);
    let alpha = if use_alpha {
        styles.numbers(Aesthetic::Alpha)?.get(row)
    } else {
        f64::NAN
    };
    let width = styles.numbers(Aesthetic::Size)?.get(row) * POINT_UNIT_SIZE;
    let linetype = styles.linetypes(Aesthetic::Linetype)?.get_ref(row);
    let color = color.with_opacity(alpha);
    // lets-plot's "blank" linetype is the pattern [0, 1].
    let blank = linetype.0 == [0.0, 1.0];
    if blank || width <= 0.0 || color.alpha == 0 {
        return Ok(None);
    }
    Ok(Some(Stroke {
        color,
        width,
        dash: linetype.0.iter().map(|length| length * width).collect(),
    }))
}

/// A polyline whose rows have these strokes: consecutive rows with the same
/// stroke form one path, joined at the shared vertex. Rows with no visible
/// stroke leave a gap. `strokes[i]` styles the segment starting at `points[i]`.
pub(super) fn styled_paths(points: &[(f64, f64)], strokes: &[Option<Stroke>]) -> Vec<Path> {
    let mut paths: Vec<Path> = Vec::new();
    let mut current: Option<Path> = None;
    for (index, point) in points.iter().enumerate() {
        let stroke = strokes[index].clone();
        match current.as_mut() {
            Some(path) if path.stroke == stroke => path.points.push(*point),
            _ => {
                if let Some(mut finished) = current.take() {
                    finished.points.push(*point);
                    if finished.stroke.is_some() && finished.points.len() > 1 {
                        paths.push(finished);
                    }
                }
                current = Some(Path {
                    points: vec![*point],
                    closed: false,
                    fill: None,
                    stroke,
                });
            }
        }
    }
    if let Some(path) = current
        && path.stroke.is_some()
        && path.points.len() > 1
    {
        paths.push(path);
    }
    paths
}

/// lets-plot `arrow()` options.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Arrow {
    angle: f64,
    length: f64,
    first: bool,
    last: bool,
    closed: bool,
}

impl Arrow {
    /// The `arrow` option of a layer (`arrow(angle, length, ends, type)`), if any.
    pub(super) fn parse(parameters: &Parameters) -> Result<Option<Arrow>> {
        let Some(value) = parameters.value("arrow") else {
            return Ok(None);
        };
        let path = parameters.path_of("arrow");
        let fail = |message: String| Error::InvalidSpec(format!("at '{path}': {message}"));
        let Value::Object(fields) = value else {
            return Err(fail(format!("expected arrow(), got {value}")));
        };
        let number = |name: &str, default: f64| match fields.get(name) {
            None | Some(Value::Null) => Ok(default),
            Some(value) => value
                .as_f64()
                .filter(|number| number.is_finite() && *number >= 0.0)
                .ok_or_else(|| {
                    fail(format!(
                        "{name}: expected a non-negative number, got {value}"
                    ))
                }),
        };
        let text = |name: &str, default: &str, accepted: &[&str]| match fields.get(name) {
            None | Some(Value::Null) => Ok(default.to_owned()),
            Some(Value::String(text)) if accepted.contains(&text.as_str()) => Ok(text.clone()),
            Some(value) => Err(fail(format!(
                "{name}: expected one of {}, got {value}",
                accepted.join(", ")
            ))),
        };
        let ends = text("ends", "last", &["last", "first", "both"])?;
        Ok(Some(Arrow {
            angle: number("angle", 30.0)?,
            length: number("length", 10.0)?,
            first: ends != "last",
            last: ends != "first",
            closed: text("type", "open", &["open", "closed"])? == "closed",
        }))
    }

    /// Draw heads at the requested ends of `points` and pull those ends back by
    /// the line width so the line does not poke through the tip (as lets-plot).
    pub(super) fn apply(&self, points: &mut [(f64, f64)], stroke: &Stroke) -> Vec<Node> {
        self.apply_at(points, stroke, true, true)
    }

    /// [`Arrow::apply`] limited to the ends that are ends of the whole line:
    /// a line split into styled pieces gets heads only at its outer ends.
    pub(super) fn apply_at(
        &self,
        points: &mut [(f64, f64)],
        stroke: &Stroke,
        at_start: bool,
        at_end: bool,
    ) -> Vec<Node> {
        let mut heads = Vec::new();
        if points.len() < 2 {
            return heads;
        }
        // Both heads are aimed from the original vertices, before any pull-back.
        let original = points.to_vec();
        let last = points.len() - 1;
        if self.last
            && at_end
            && let Some((head, pulled)) = self.head(&original, last, last - 1, stroke)
        {
            points[last] = pulled;
            heads.push(head);
        }
        if self.first
            && at_start
            && let Some((head, pulled)) = self.head(&original, 0, 1, stroke)
        {
            points[0] = pulled;
            heads.push(head);
        }
        heads
    }

    fn head(
        &self,
        points: &[(f64, f64)],
        tip_index: usize,
        neighbour_index: usize,
        stroke: &Stroke,
    ) -> Option<(Node, (f64, f64))> {
        let (tip_x, tip_y) = points[tip_index];
        let (neighbour_x, neighbour_y) = points[neighbour_index];
        let (dx, dy) = (neighbour_x - tip_x, neighbour_y - tip_y);
        let distance = dx.hypot(dy);
        if distance == 0.0 {
            return None;
        }
        let (back_x, back_y) = (dx / distance, dy / distance);
        let barb = |sign: f64| {
            let angle = sign * self.angle.to_radians();
            (
                tip_x + self.length * (back_x * angle.cos() - back_y * angle.sin()),
                tip_y + self.length * (back_x * angle.sin() + back_y * angle.cos()),
            )
        };
        let tip = (tip_x, tip_y);
        // At most half the line, so pulling back both ends cannot cross over.
        let pull = stroke.width.min(distance / 2.0);
        let pulled = (tip_x + back_x * pull, tip_y + back_y * pull);
        let head = Node::Path(Path {
            points: vec![barb(1.0), tip, barb(-1.0)],
            closed: self.closed,
            fill: self.closed.then_some(stroke.color),
            stroke: Some(Stroke::solid(stroke.color, stroke.width)),
        });
        Some((head, pulled))
    }
}

/// A horizontal key line through the middle of `cell`, or a vertical one.
pub(super) fn key_line(styles: &Styles, cell: Rect, vertical: bool) -> Result<Vec<Node>> {
    let Some(stroke) = stroke_at(styles, 0)? else {
        return Ok(Vec::new());
    };
    let points = if vertical {
        let x = cell.x + cell.width / 2.0;
        vec![(x, cell.y), (x, cell.y + cell.height)]
    } else {
        let y = cell.y + cell.height / 2.0;
        vec![(cell.x, y), (cell.x + cell.width, y)]
    };
    Ok(vec![Node::Path(Path {
        points,
        closed: false,
        fill: None,
        stroke: Some(stroke),
    })])
}

/// Smallest legend key for a line: its width plus lets-plot's 2 px of room.
pub(super) fn key_size(styles: &Styles) -> Result<(f64, f64)> {
    let width = styles.numbers(Aesthetic::Size)?.get(0).max(0.0) * POINT_UNIT_SIZE;
    Ok((width + 2.0, width + 2.0))
}
