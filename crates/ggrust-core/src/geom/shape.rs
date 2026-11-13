//! `geom_shape` (ggforce): `geom_polygon` with corners rounded by `radius` and the
//! outline grown by `expand`, both in millimetres of screen space.
//!
//! As ggforce's `shapeGrob`, the work happens after the coordinate system, so a
//! rounded corner stays round under any aspect ratio. Rounding replaces each convex
//! corner by a circular arc tangent to its two edges; concave corners stay sharp.
//! Unlike ggforce, a radius too large for a corner is reduced to fit the corner's
//! edges (a rectangle with a radius of at least half its height becomes a stadium)
//! instead of making the shape shrink or vanish.
//!
//! What it does not do: a negative `expand` (shrinking), rings or holes
//! (`subgroup`), single-row groups (drawn as nothing, as `geom_polygon`), tooltips.
//! Self-intersecting outlines are rounded by their net orientation.

use super::mark_label::PIXELS_PER_MM;
use super::mark_shape::{self, Point};
use super::polygon::draw_polygons;
use super::{DrawLayer, Geom, PolygonGeom};
use crate::coord::Panel;
use crate::scene::{Node, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};
use crate::{Error, Result};

/// Largest distance (pixels) between an arc and the chords drawn for it.
const ARC_TOLERANCE: f64 = 0.05;
/// Most chords per rounded corner.
const MAXIMUM_ARC_SEGMENTS: usize = 90;

/// Filled shapes with rounded corners.
pub struct ShapeGeom;

impl Geom for ShapeGeom {
    fn name(&self) -> &'static str {
        "shape"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        PolygonGeom.style_defaults()
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["expand", "radius"]
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let expand = millimetres(layer, "expand")?;
        let radius = millimetres(layer, "radius")?;
        let mut nodes = draw_polygons(layer, panel, false)?;
        if expand == 0.0 && radius == 0.0 {
            return Ok(nodes);
        }
        for node in &mut nodes {
            if let Node::Path(path) = node {
                path.points = shape_outline(&path.points, expand, radius, panel.rect);
            }
        }
        Ok(nodes)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        PolygonGeom.minimum_key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        PolygonGeom.draw_key(styles, cell)
    }
}

/// A non-negative length in millimetres (default 0), as pixels.
fn millimetres(layer: &DrawLayer, name: &str) -> Result<f64> {
    let value = layer.parameters.number(name)?.unwrap_or(0.0);
    if value < 0.0 {
        return Err(Error::spec(
            layer.parameters.path_of(name),
            format!(
                "{name} must be a non-negative length in millimetres, got {value}; \
                 shrinking shapes is not supported"
            ),
        ));
    }
    Ok(value * PIXELS_PER_MM)
}

/// The outline (panel pixels) grown by `expand` with corners rounded by `radius`.
///
/// ggforce offsets by `expand - radius` with mitred corners, then by `radius` with
/// round ones. Here the corners are rounded first by `radius - expand` (on the
/// original edges, where the radius is fitted), then the round offset by `expand`
/// grows those arcs to `radius`: the same outline where the radius fits. Before the
/// offset (a distance grid sized by the outline) the outline is clipped to `view`
/// grown by a margin, so a shape far larger than the panel keeps a fine grid; the
/// cut edges and their offsets stay outside `view`.
pub(super) fn shape_outline(outline: &[Point], expand: f64, radius: f64, view: Rect) -> Vec<Point> {
    let rounded = round_corners(outline, (radius - expand).max(0.0));
    if expand <= 0.0 {
        return rounded;
    }
    let margin = expand + 4.0;
    let clipped = clip_to_rect(
        &rounded,
        (view.x - margin, view.y - margin),
        (view.x + view.width + margin, view.y + view.height + margin),
    );
    if clipped.len() < 3 && rounded.len() >= 3 {
        // Entirely outside the view: nothing of it (or its offset) shows.
        return Vec::new();
    }
    let source = if rounded.len() >= 3 {
        &clipped
    } else {
        &rounded
    };
    mark_shape::expand_outline(source, expand, radius.min(expand))
}

/// Sutherland-Hodgman clipping of a closed polygon to the box `low`..`high`.
fn clip_to_rect(polygon: &[Point], low: Point, high: Point) -> Vec<Point> {
    let inside = |point: Point, side: usize| match side {
        0 => point.0 >= low.0,
        1 => point.0 <= high.0,
        2 => point.1 >= low.1,
        _ => point.1 <= high.1,
    };
    let crossing = |a: Point, b: Point, side: usize| {
        let t = match side {
            0 => (low.0 - a.0) / (b.0 - a.0),
            1 => (high.0 - a.0) / (b.0 - a.0),
            2 => (low.1 - a.1) / (b.1 - a.1),
            _ => (high.1 - a.1) / (b.1 - a.1),
        };
        (a.0 + t * (b.0 - a.0), a.1 + t * (b.1 - a.1))
    };
    let mut points = polygon.to_vec();
    for side in 0..4 {
        let Some(&last) = points.last() else { break };
        let mut previous = last;
        let mut kept = Vec::with_capacity(points.len() + 4);
        for &current in &points {
            match (inside(previous, side), inside(current, side)) {
                (true, true) => kept.push(current),
                (true, false) => kept.push(crossing(previous, current, side)),
                (false, true) => {
                    kept.push(crossing(previous, current, side));
                    kept.push(current);
                }
                (false, false) => {}
            }
            previous = current;
        }
        points = kept;
    }
    points
}

/// `polygon` without repeated vertices (the closing one included) and without
/// vertices on a straight line between their neighbours.
fn clean_polygon(polygon: &[Point]) -> Vec<Point> {
    let mut points: Vec<Point> = Vec::with_capacity(polygon.len());
    for &point in polygon {
        if points.last() != Some(&point) {
            points.push(point);
        }
    }
    while points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    loop {
        let n = points.len();
        if n < 3 {
            return points;
        }
        let straight = (0..n).find(|&index| {
            let (previous, vertex, next) = (
                points[(index + n - 1) % n],
                points[index],
                points[(index + 1) % n],
            );
            let incoming = (vertex.0 - previous.0, vertex.1 - previous.1);
            let outgoing = (next.0 - vertex.0, next.1 - vertex.1);
            let cross = incoming.0 * outgoing.1 - incoming.1 * outgoing.0;
            let dot = incoming.0 * outgoing.0 + incoming.1 * outgoing.1;
            let lengths = incoming.0.hypot(incoming.1) * outgoing.0.hypot(outgoing.1);
            // On the line and going on (a spike that turns back is a corner).
            cross.abs() <= 1e-9 * lengths && dot > 0.0
        });
        match straight {
            Some(index) => {
                points.remove(index);
            }
            None => return points,
        }
    }
}

/// A convex corner to round: its tangent length wish and the tangent of half its angle.
struct Corner {
    wanted: f64,
    half_tangent: f64,
}

/// `polygon` with every convex corner replaced by a circular arc of `radius`
/// tangent to both edges. A corner whose edges are too short for the radius gets
/// the largest radius that fits: each edge is shared by the two corners at its
/// ends (all of it goes to one end when the other is concave).
pub(super) fn round_corners(polygon: &[Point], radius: f64) -> Vec<Point> {
    let points = clean_polygon(polygon);
    let n = points.len();
    let area = mark_shape::signed_area(&points);
    if n < 3 || radius <= 0.0 || !radius.is_finite() || area == 0.0 || !area.is_finite() {
        return points;
    }
    let unit = |from: Point, to: Point| {
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        let length = dx.hypot(dy);
        ((dx / length, dy / length), length)
    };
    let corners: Vec<Option<Corner>> = (0..n)
        .map(|index| {
            let (previous, vertex, next) = (
                points[(index + n - 1) % n],
                points[index],
                points[(index + 1) % n],
            );
            let incoming = (vertex.0 - previous.0, vertex.1 - previous.1);
            let outgoing = (next.0 - vertex.0, next.1 - vertex.1);
            let turn = incoming.0 * outgoing.1 - incoming.1 * outgoing.0;
            if turn * area <= 0.0 {
                return None;
            }
            let ((back_x, back_y), _) = unit(vertex, previous);
            let ((ahead_x, ahead_y), _) = unit(vertex, next);
            let cosine = (back_x * ahead_x + back_y * ahead_y).clamp(-1.0, 1.0);
            let half_tangent = ((1.0 - cosine) / (1.0 + cosine)).sqrt();
            Some(Corner {
                wanted: radius / half_tangent,
                half_tangent,
            })
        })
        .collect();
    // The share of edge `index` -> `index + 1` the corner at `index` may use.
    let edge_share = |index: usize, other: usize| {
        let (_, length) = unit(points[index], points[other]);
        if corners[other].is_some() {
            0.5 * length
        } else {
            length
        }
    };
    let mut outline = Vec::with_capacity(n * 8);
    for index in 0..n {
        let vertex = points[index];
        let Some(corner) = &corners[index] else {
            outline.push(vertex);
            continue;
        };
        let (previous_index, next_index) = ((index + n - 1) % n, (index + 1) % n);
        let tangent = corner
            .wanted
            .min(edge_share(index, previous_index))
            .min(edge_share(index, next_index));
        let corner_radius = tangent * corner.half_tangent;
        if !(corner_radius > 1e-9 && tangent.is_finite()) {
            outline.push(vertex);
            continue;
        }
        let ((back_x, back_y), _) = unit(vertex, points[previous_index]);
        let ((ahead_x, ahead_y), _) = unit(vertex, points[next_index]);
        let start = (vertex.0 + back_x * tangent, vertex.1 + back_y * tangent);
        let end = (vertex.0 + ahead_x * tangent, vertex.1 + ahead_y * tangent);
        // The centre is on the bisector, `tangent` along an edge and `radius` off it.
        let ((bisector_x, bisector_y), _) = unit((0.0, 0.0), (back_x + ahead_x, back_y + ahead_y));
        let centre_distance = tangent.hypot(corner_radius);
        let centre = (
            vertex.0 + bisector_x * centre_distance,
            vertex.1 + bisector_y * centre_distance,
        );
        let start_angle = (start.1 - centre.1).atan2(start.0 - centre.0);
        let end_angle = (end.1 - centre.1).atan2(end.0 - centre.0);
        let mut sweep = end_angle - start_angle;
        // The arc is the short way round (less than half a turn).
        if sweep > std::f64::consts::PI {
            sweep -= std::f64::consts::TAU;
        } else if sweep < -std::f64::consts::PI {
            sweep += std::f64::consts::TAU;
        }
        let step = 2.0 * (1.0 - ARC_TOLERANCE / corner_radius).max(-1.0).acos();
        let segments = ((sweep.abs() / step).ceil() as usize).clamp(1, MAXIMUM_ARC_SEGMENTS);
        outline.push(start);
        for segment in 1..segments {
            let angle = start_angle + sweep * segment as f64 / segments as f64;
            outline.push((
                centre.0 + corner_radius * angle.cos(),
                centre.1 + corner_radius * angle.sin(),
            ));
        }
        outline.push(end);
    }
    // Corners that share a whole edge meet at one point.
    outline.dedup_by(|a, b| (a.0 - b.0).hypot(a.1 - b.1) < 1e-9);
    if outline.len() > 1
        && outline
            .first()
            .zip(outline.last())
            .is_some_and(|(a, b)| (a.0 - b.0).hypot(a.1 - b.1) < 1e-9)
    {
        outline.pop();
    }
    outline
}

#[cfg(test)]
mod tests {
    use super::*;

    fn distance(a: Point, b: Point) -> f64 {
        (a.0 - b.0).hypot(a.1 - b.1)
    }

    const VIEW: Rect = Rect {
        x: -50.0,
        y: -50.0,
        width: 200.0,
        height: 140.0,
    };

    const RECTANGLE: [Point; 4] = [(0.0, 0.0), (100.0, 0.0), (100.0, 40.0), (0.0, 40.0)];

    #[test]
    fn zero_radius_keeps_the_polygon() {
        assert_eq!(round_corners(&RECTANGLE, 0.0), RECTANGLE.to_vec());
    }

    #[test]
    fn rounded_rectangle_stays_inside_and_touches_every_edge() {
        let outline = round_corners(&RECTANGLE, 10.0);
        for &(x, y) in &outline {
            assert!((-1e-9..=100.0 + 1e-9).contains(&x) && (-1e-9..=40.0 + 1e-9).contains(&y));
        }
        // Arc points are 10 px from their corner's centre.
        let near_origin: Vec<Point> = outline
            .iter()
            .copied()
            .filter(|&(x, y)| x <= 10.0 && y <= 10.0)
            .collect();
        assert!(near_origin.len() > 3);
        for point in near_origin {
            assert!((distance(point, (10.0, 10.0)) - 10.0).abs() < 1e-9);
        }
        // The flat parts of the edges remain.
        assert!(outline.contains(&(10.0, 0.0)) && outline.contains(&(90.0, 0.0)));
    }

    #[test]
    fn radius_beyond_half_height_gives_a_stadium() {
        for radius in [20.0, 35.0, 1e6] {
            let outline = round_corners(&RECTANGLE, radius);
            // Every point is 20 px from the centre line between (20, 20) and (80, 20).
            for &(x, y) in &outline {
                let along = x.clamp(20.0, 80.0);
                assert!(
                    (distance((x, y), (along, 20.0)) - 20.0).abs() < 1e-9,
                    "{x} {y}"
                );
            }
            // The straight edges keep their middle part.
            assert!(outline.contains(&(20.0, 0.0)) && outline.contains(&(80.0, 0.0)));
        }
    }

    #[test]
    fn concave_corners_stay_sharp() {
        let l_shape = [
            (0.0, 0.0),
            (60.0, 0.0),
            (60.0, 20.0),
            (20.0, 20.0),
            (20.0, 60.0),
            (0.0, 60.0),
        ];
        let outline = round_corners(&l_shape, 5.0);
        assert!(outline.contains(&(20.0, 20.0)));
        assert!(!outline.contains(&(0.0, 0.0)));
        // Clockwise input rounds the same corners.
        let reversed: Vec<Point> = l_shape.iter().rev().copied().collect();
        let outline = round_corners(&reversed, 5.0);
        assert!(outline.contains(&(20.0, 20.0)) && !outline.contains(&(60.0, 0.0)));
    }

    #[test]
    fn repeated_and_collinear_vertices_are_ignored() {
        let messy = [
            (0.0, 0.0),
            (50.0, 0.0),
            (100.0, 0.0),
            (100.0, 0.0),
            (100.0, 40.0),
            (0.0, 40.0),
            (0.0, 0.0),
        ];
        assert_eq!(round_corners(&messy, 20.0), round_corners(&RECTANGLE, 20.0));
    }

    #[test]
    fn degenerate_input_comes_back() {
        assert_eq!(round_corners(&[(1.0, 1.0), (5.0, 5.0)], 3.0).len(), 2);
        let line = [(0.0, 0.0), (5.0, 5.0), (10.0, 10.0)];
        assert_eq!(round_corners(&line, 3.0).len(), 2);
    }

    #[test]
    fn expand_grows_the_rounded_shape() {
        let outline = shape_outline(&RECTANGLE, 5.0, 25.0, VIEW);
        let (mut low_y, mut high_y) = (f64::INFINITY, f64::NEG_INFINITY);
        for &(_, y) in &outline {
            low_y = low_y.min(y);
            high_y = high_y.max(y);
        }
        assert!((low_y + 5.0).abs() < 0.6 && (high_y - 45.0).abs() < 0.6);
        // A stadium of half height 25 around the centre line.
        for &(x, y) in &outline {
            let along = x.clamp(20.0, 80.0);
            assert!(
                (distance((x, y), (along, 20.0)) - 25.0).abs() < 0.6,
                "{x} {y}"
            );
        }
    }

    #[test]
    fn huge_shapes_are_clipped_before_expanding() {
        // A square far larger than the view: its left edge (x = 0) runs through it.
        let square = [(0.0, -1e6), (1e6, -1e6), (1e6, 1e6), (0.0, 1e6)];
        let outline = shape_outline(&square, 5.0, 0.0, VIEW);
        // Before the fix, a grid hundreds of pixels wide gave a diagonal here.
        let left = outline
            .iter()
            .map(|&(x, _)| x)
            .fold(f64::INFINITY, f64::min);
        assert!((left + 5.0).abs() < 0.6, "{left}");
        for &(x, y) in &outline {
            if x < 50.0 && (-55.0..=145.0).contains(&y) {
                assert!((x + 5.0).abs() < 0.6, "{x} {y}");
            }
        }
        // Wholly outside the view: nothing.
        let far = [(1e5, 1e5), (1e5 + 10.0, 1e5), (1e5 + 10.0, 1e5 + 10.0)];
        assert!(shape_outline(&far, 5.0, 2.0, VIEW).is_empty());
    }
}
