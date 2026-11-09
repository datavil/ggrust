//! End padding of `geom_segment` and `geom_curve`: the line stops short of the
//! points it connects, by `size_start`/`size_end` (point sizes), `stroke_start`/
//! `stroke_end` (point outlines) and the `spacer` parameter (pixels).
//!
//! Follows lets-plot (measured on 4.11): the padding at an end is
//! `size x 2.2 / 2 + stroke x 2.2 + spacer` pixels, so a line to a point of the
//! same `size` and `stroke` touches its outline. The start is cut first, then the
//! end of what is left; a cut longer than the (remaining) line leaves it whole, as
//! lets-plot `PolylinePadding`. Distances are straight-line from the end point,
//! so on a curve they are measured on the control polygon before smoothing.
//!
//! What it does not do: negative sizes or strokes extend the line in lets-plot;
//! here they count as 0 (see deviations.md).

use super::point::POINT_UNIT_SIZE;
use crate::Error;
use crate::Result;
use crate::parameters::Parameters;
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};

type Point = (f64, f64);

/// Style defaults of padded lines: the line defaults (`lines::LINE_STYLE_DEFAULTS`)
/// and no padding.
pub(super) const PADDED_LINE_STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (Aesthetic::Color, StyleDefault::PEN),
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Size, StyleDefault::Number(0.75)),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
    (Aesthetic::SizeStart, StyleDefault::Number(0.0)),
    (Aesthetic::SizeEnd, StyleDefault::Number(0.0)),
    (Aesthetic::StrokeStart, StyleDefault::Number(0.0)),
    (Aesthetic::StrokeEnd, StyleDefault::Number(0.0)),
];

/// The `spacer` parameter: pixels of extra padding at both ends (default 0).
pub(super) fn parse_spacer(parameters: &Parameters) -> Result<f64> {
    match parameters.number("spacer")? {
        None => Ok(0.0),
        Some(value) if value >= 0.0 => Ok(value),
        Some(value) => Err(Error::spec(
            parameters.path_of("spacer"),
            format!("expected a non-negative number of pixels, got {value}; try spacer=0"),
        )),
    }
}

/// The padding in pixels at the start and the end of one row's line.
pub(super) fn end_padding(styles: &Styles, row: usize, spacer: f64) -> Result<(f64, f64)> {
    let amount = |aesthetic: Aesthetic| -> Result<f64> {
        let value = styles.numbers(aesthetic)?.get(row);
        // NaN and negative values pad nothing (`max` drops NaN).
        Ok(value.max(0.0))
    };
    let padding = |size: Aesthetic, stroke: Aesthetic| -> Result<f64> {
        Ok(amount(size)? * POINT_UNIT_SIZE / 2.0 + amount(stroke)? * POINT_UNIT_SIZE + spacer)
    };
    Ok((
        padding(Aesthetic::SizeStart, Aesthetic::StrokeStart)?,
        padding(Aesthetic::SizeEnd, Aesthetic::StrokeEnd)?,
    ))
}

/// Cut `start` pixels off the start of a polyline, then `end` pixels off the end
/// of the rest.
pub(super) fn pad_ends(points: &[Point], start: f64, end: f64) -> Vec<Point> {
    let trimmed = if start > 0.0 {
        trim_start(points, start)
    } else {
        points.to_vec()
    };
    if end > 0.0 {
        trim_end(&trimmed, end)
    } else {
        trimmed
    }
}

fn distance(a: Point, b: Point) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// Cut `distance_px` off the start of a polyline (lets-plot `PolylinePadding`):
/// the first vertex at least that far (straight-line) from the start is kept and
/// the start moves onto the piece before it. Lines shorter than that stay whole.
fn trim_start(points: &[Point], distance_px: f64) -> Vec<Point> {
    if points.len() < 2 {
        return points.to_vec();
    }
    let first = points[0];
    let Some(index) = points
        .iter()
        .position(|point| distance(first, *point) >= distance_px)
        .filter(|index| *index >= 1)
    else {
        return points.to_vec();
    };
    let (before, at) = (points[index - 1], points[index]);
    let remaining = distance(first, at) - distance_px;
    let piece = distance(at, before);
    let moved = if piece == 0.0 {
        at
    } else {
        let share = remaining / piece;
        (
            at.0 + (before.0 - at.0) * share,
            at.1 + (before.1 - at.1) * share,
        )
    };
    let mut trimmed = vec![moved];
    trimmed.extend_from_slice(&points[index..]);
    trimmed
}

/// [`trim_start`] at the end of the polyline.
fn trim_end(points: &[Point], distance_px: f64) -> Vec<Point> {
    let reversed: Vec<Point> = points.iter().rev().copied().collect();
    let mut trimmed = trim_start(&reversed, distance_px);
    trimmed.reverse();
    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trimming_follows_lets_plot_padding() {
        let line = [(0.0, 0.0), (10.0, 0.0), (20.0, 0.0)];
        assert_eq!(
            trim_start(&line, 5.0),
            vec![(5.0, 0.0), (10.0, 0.0), (20.0, 0.0)]
        );
        assert_eq!(trim_end(&line, 15.0), vec![(0.0, 0.0), (5.0, 0.0)]);
        // Longer than the line: left whole.
        assert_eq!(trim_start(&line, 25.0), line.to_vec());
    }

    #[test]
    fn start_is_cut_before_the_end() {
        // lets-plot: 275 + 275 px on a 445 px segment cuts only the start.
        let line = [(0.0, 0.0), (445.0, 0.0)];
        assert_eq!(
            pad_ends(&line, 275.0, 275.0),
            vec![(275.0, 0.0), (445.0, 0.0)]
        );
        assert_eq!(
            pad_ends(&line, 165.0, 165.0),
            vec![(165.0, 0.0), (280.0, 0.0)]
        );
        assert_eq!(pad_ends(&line, 0.0, 0.0), line.to_vec());
    }
}
