//! `geom_curve`: a curved line from (`x`, `y`) to (`xend`, `yend`) per row, as
//! lets-plot `CurveGeom`: `curvature`, `angle`, `ncp`, `spacer` and `arrow=arrow(...)`.
//!
//! The control points are R grid's `calcControlPoints` (ported by lets-plot), computed
//! in pixels; the drawn line is the uniform cubic B-spline over the start point, the
//! control points and the end point (lets-plot `Interpolation.BSPLINE`), flattened
//! into a polyline. Arrow heads aim along the control polygon, as lets-plot.
//!
//! What it does not do (yet): the `size_start`/`size_end`/`stroke_start`/`stroke_end`
//! offsets (ggrust has no such aesthetics; segments lack them too).

use super::lines::{self, Arrow, LINE_STYLE_DEFAULTS};
use super::{DrawLayer, Geom};
use crate::Error;
use crate::Result;
use crate::coord::Panel;
use crate::parameters::Parameters;
use crate::scene::{Node, Path, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};

/// Curved line segments.
pub struct CurveGeom;

/// lets-plot `CurveGeom` defaults.
const DEFAULT_CURVATURE: f64 = 0.5;
const DEFAULT_ANGLE: f64 = 90.0;
const DEFAULT_NCP: usize = 5;
/// Upper bound on `ncp`, so a typo cannot allocate millions of vertices per row.
const MAX_NCP: usize = 10_000;

type Point = (f64, f64);

/// The curve options of one layer.
#[derive(Debug, Clone, Copy, PartialEq)]
struct CurveOptions {
    curvature: f64,
    /// Degrees, reduced to `[0, 180)` as lets-plot.
    angle: f64,
    ncp: usize,
    spacer: f64,
}

impl CurveOptions {
    fn parse(parameters: &Parameters) -> Result<CurveOptions> {
        let curvature = parameters.number("curvature")?.unwrap_or(DEFAULT_CURVATURE);
        // lets-plot `CurveGeom.angle` setter: `angle % 180`, negative values + 180.
        let mut angle = parameters.number("angle")?.unwrap_or(DEFAULT_ANGLE) % 180.0;
        if angle < 0.0 {
            angle += 180.0;
        }
        let ncp = match parameters.number("ncp")? {
            None => DEFAULT_NCP,
            Some(value) if value >= 0.0 && value.fract() == 0.0 && value <= MAX_NCP as f64 => {
                value as usize
            }
            Some(value) => {
                return Err(Error::spec(
                    parameters.path_of("ncp"),
                    format!(
                        "expected a whole number of control points from 0 to {MAX_NCP}, got \
                         {value}; try the default, ncp=5"
                    ),
                ));
            }
        };
        let spacer = match parameters.number("spacer")? {
            None => 0.0,
            Some(value) if value >= 0.0 => value,
            Some(value) => {
                return Err(Error::spec(
                    parameters.path_of("spacer"),
                    format!("expected a non-negative number of pixels, got {value}"),
                ));
            }
        };
        Ok(CurveOptions {
            curvature,
            angle,
            ncp,
            spacer,
        })
    }
}

impl Geom for CurveGeom {
    fn name(&self) -> &'static str {
        "curve"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y, Aesthetic::Xend, Aesthetic::Yend]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        LINE_STYLE_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["arrow", "curvature", "angle", "ncp", "spacer"]
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
        let options = CurveOptions::parse(&layer.parameters)?;
        let arrow = Arrow::parse(&layer.parameters)?;
        let mut nodes = Vec::new();
        for row in 0..layer.data.row_count() {
            let value =
                |column: &crate::data::ContinuousColumn| column.get(row).unwrap_or(f64::NAN);
            let start = panel.map(value(x), value(y));
            let end = panel.map(value(x_end), value(y_end));
            let finite = [start.0, start.1, end.0, end.1]
                .iter()
                .all(|coordinate| coordinate.is_finite());
            // lets-plot draws nothing when the ends coincide.
            if !finite || start == end {
                continue;
            }
            let Some(stroke) = lines::stroke_at(layer.styles, row)? else {
                continue;
            };
            // Screen y points down, so lets-plot passes the angle negated.
            let mut guide =
                control_polygon(start, end, options.curvature, -options.angle, options.ncp);
            if guide
                .iter()
                .any(|point| !point.0.is_finite() || !point.1.is_finite())
            {
                guide = vec![start, end];
            }
            if options.spacer > 0.0 {
                guide = trim_end(&trim_start(&guide, options.spacer), options.spacer);
            }
            let mut points = basis_spline(&guide);
            let heads = arrow
                .map(|arrow| arrow.apply_along(&mut points, &guide, &stroke))
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
        lines::key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        lines::key_line(styles, cell, false)
    }
}

fn add(a: Point, b: Point) -> Point {
    (a.0 + b.0, a.1 + b.1)
}

fn subtract(a: Point, b: Point) -> Point {
    (a.0 - b.0, a.1 - b.1)
}

fn rotate(point: Point, angle: f64) -> Point {
    let (sin, cos) = angle.sin_cos();
    (point.0 * cos - point.1 * sin, point.0 * sin + point.1 * cos)
}

fn slope(a: Point, b: Point) -> f64 {
    (b.1 - a.1) / (b.0 - a.0)
}

fn distance(a: Point, b: Point) -> f64 {
    (b.0 - a.0).hypot(b.1 - a.1)
}

/// Start, control points and end of one curve (lets-plot `curve()` in
/// `commons.geometry`, a port of R grid's `calcControlPoints`/`calcOrigin`).
/// `angle` is in degrees; a straight line (curvature 0, or an angle outside
/// 1..=179 in magnitude) has no control points.
fn control_polygon(start: Point, end: Point, curvature: f64, angle: f64, ncp: usize) -> Vec<Point> {
    let mut polygon = vec![start];
    if curvature != 0.0 && (1.0..=179.0).contains(&angle.abs()) {
        polygon.extend(control_points(start, end, curvature, angle, ncp));
    }
    polygon.push(end);
    polygon
}

fn control_points(start: Point, end: Point, curvature: f64, angle: f64, ncp: usize) -> Vec<Point> {
    let middle = ((start.0 + end.0) * 0.5, (start.1 + end.1) * 0.5);
    let chord = subtract(end, start);
    // The corner of the region the control points live in.
    let corner = add(middle, rotate(subtract(start, middle), angle.to_radians()));
    // Rotate the region to align it with the axes, then scale it to a square.
    let beta = -slope(start, corner).atan();
    let rotated_end = add(start, rotate(chord, beta));
    let scale_x = slope(start, rotated_end);
    let ratio_angle = curvature.atan().sin();
    let origin_offset = curvature - curvature / (2.0 * ratio_angle.powi(2));
    let square_start = (start.0 * scale_x, start.1);
    let square_end = (rotated_end.0 * scale_x, rotated_end.1);
    let origin = rotation_origin(square_start, square_end, origin_offset);
    let direction = curvature.signum();
    let max_theta = std::f64::consts::PI
        + 2.0 * (origin_offset * direction).signum() * origin_offset.abs().atan();
    let from_origin = subtract(square_start, origin);
    (1..=ncp)
        .map(|index| {
            let theta = index as f64 * direction * max_theta / (ncp + 1) as f64;
            let square = add(origin, rotate(from_origin, theta));
            let unscaled = (square.0 / scale_x, square.1);
            add(start, rotate(subtract(unscaled, start), -beta))
        })
        .collect()
}

/// R grid `calcOrigin`: the point the start is rotated about, at `origin_offset`
/// half-chords from the middle, perpendicular to the chord.
fn rotation_origin(start: Point, end: Point, origin_offset: f64) -> Point {
    let middle = ((start.0 + end.0) * 0.5, (start.1 + end.1) * 0.5);
    let chord = subtract(end, start);
    let chord_slope = slope(start, end);
    let normal_slope = -1.0 / chord_slope;
    let along_x = if chord_slope.is_finite() {
        origin_offset * chord.0 / 2.0
    } else {
        0.0
    };
    let along_y = if chord_slope.is_finite() && !normal_slope.is_finite() {
        0.0
    } else {
        origin_offset * chord.1 / 2.0
    };
    (middle.0 + along_y, middle.1 - along_x)
}

/// The uniform cubic B-spline over `polygon` with tripled ends (lets-plot
/// `SvgPathDataBuilder` BSPLINE: a line to the first knot, cubic pieces, a line to
/// the last vertex), flattened into a polyline. Two vertices stay a straight line.
fn basis_spline(polygon: &[Point]) -> Vec<Point> {
    if polygon.len() < 3 {
        return polygon.to_vec();
    }
    let blend = |weights: [f64; 4], window: &[Point; 4]| -> Point {
        window
            .iter()
            .zip(weights)
            .fold((0.0, 0.0), |sum, (point, weight)| {
                (sum.0 + weight * point.0, sum.1 + weight * point.1)
            })
    };
    const FIRST: [f64; 4] = [0.0, 2.0 / 3.0, 1.0 / 3.0, 0.0];
    const SECOND: [f64; 4] = [0.0, 1.0 / 3.0, 2.0 / 3.0, 0.0];
    const KNOT: [f64; 4] = [0.0, 1.0 / 6.0, 2.0 / 3.0, 1.0 / 6.0];
    let first = polygon[0];
    let last = polygon[polygon.len() - 1];
    let mut window = [first, first, first, polygon[1]];
    let mut points = vec![first];
    let mut current = blend(KNOT, &window);
    points.push(current);
    for index in 2..=polygon.len() {
        let next = polygon.get(index).copied().unwrap_or(last);
        window.rotate_left(1);
        window[3] = next;
        let control_1 = blend(FIRST, &window);
        let control_2 = blend(SECOND, &window);
        let knot = blend(KNOT, &window);
        flatten_cubic(current, control_1, control_2, knot, &mut points);
        current = knot;
    }
    points.push(last);
    points
}

/// Append the cubic Bezier from `p0` (excluded) to `p3` as straight pieces about
/// 2 px long (at least 2, at most 64 pieces).
fn flatten_cubic(p0: Point, p1: Point, p2: Point, p3: Point, points: &mut Vec<Point>) {
    let hull = distance(p0, p1) + distance(p1, p2) + distance(p2, p3);
    let pieces = ((hull / 2.0).ceil() as usize).clamp(2, 64);
    for step in 1..=pieces {
        let t = step as f64 / pieces as f64;
        let u = 1.0 - t;
        let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
        points.push((
            a * p0.0 + b * p1.0 + c * p2.0 + d * p3.0,
            a * p0.1 + b * p1.1 + c * p2.1 + d * p3.1,
        ));
    }
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

    fn close(a: Point, b: Point) -> bool {
        (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
    }

    #[test]
    fn straight_without_curvature_or_with_flat_angles() {
        let (start, end) = ((0.0, 0.0), (100.0, 0.0));
        assert_eq!(control_polygon(start, end, 0.0, -90.0, 5), vec![start, end]);
        assert_eq!(control_polygon(start, end, 0.5, -0.5, 5), vec![start, end]);
        assert_eq!(
            control_polygon(start, end, 0.5, -179.5, 5),
            vec![start, end]
        );
        assert_eq!(basis_spline(&[start, end]), vec![start, end]);
    }

    #[test]
    fn symmetric_arc_at_right_angle() {
        // curvature 1, angle 90 is a half circle: one control point at the apex.
        let polygon = control_polygon((0.0, 0.0), (100.0, 0.0), 1.0, -90.0, 1);
        assert_eq!(polygon.len(), 3);
        assert!(close(polygon[1], (50.0, 50.0)), "{:?}", polygon[1]);
        // Negative curvature bends to the other side.
        let mirrored = control_polygon((0.0, 0.0), (100.0, 0.0), -1.0, -90.0, 1);
        assert!(close(mirrored[1], (50.0, -50.0)), "{:?}", mirrored[1]);
    }

    #[test]
    fn control_points_match_lets_plot() {
        // lets-plot 4.11 `geom_curve(curvature=0.5)` from (0, 0) to (100, 0) px
        // (control points recovered from its BSPLINE path).
        let polygon = control_polygon((0.0, 0.0), (100.0, 0.0), 0.5, -90.0, 5);
        assert_eq!(polygon.len(), 7);
        // Symmetric about the middle, all on one side, apex 25 px out.
        for index in 1..=5 {
            let (a, b) = (polygon[index], polygon[6 - index]);
            assert!((a.0 + b.0 - 100.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9);
            assert!(a.1 > 0.0);
        }
        assert!(close(polygon[3], (50.0, 25.0)), "{:?}", polygon[3]);
    }

    #[test]
    fn spline_starts_and_ends_at_the_ends() {
        let polygon = control_polygon((10.0, 20.0), (200.0, 80.0), 0.4, -90.0, 1);
        let points = basis_spline(&polygon);
        assert_eq!(points[0], (10.0, 20.0));
        assert_eq!(points[points.len() - 1], (200.0, 80.0));
        // The first knot is 1/6 of the way to the control point.
        let knot = (
            10.0 + (polygon[1].0 - 10.0) / 6.0,
            20.0 + (polygon[1].1 - 20.0) / 6.0,
        );
        assert!(close(points[1], knot));
    }

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
}
