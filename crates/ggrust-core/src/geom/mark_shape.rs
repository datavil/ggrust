//! Outlines of the ggforce marks (`geom_mark_hull`, `geom_mark_ellipse`,
//! `geom_mark_rect`), in panel pixels.
//!
//! - [`concave_hull`]: the concaveman algorithm (Park & Oh 2012, as in the ISC
//!   `concaveman` JavaScript library that R's ggforce calls): start from the convex
//!   hull and dig each edge towards the nearest inner point while the edge is longer
//!   than `concavity` times that point's distance to its ends.
//! - [`enclosing_ellipse`]: Khachiyan's minimum volume enclosing ellipse, with the
//!   2D degenerate cases of ggforce (one point, collinear points).
//! - [`expand_outline`]: ggforce's `shapeGrob` expansion: offset by
//!   `expand - radius` with mitred corners, then by `radius` with round corners.
//!   Computed as a level set of the exact distance to the mitred shape on a fine
//!   grid (marching squares), so concave outlines never self-intersect.
//!
//! What it does not do: drawing, labels (see `mark_label.rs`), data units.

use std::collections::{HashMap, VecDeque};

/// A point in panel pixels.
pub(super) type Point = (f64, f64);

/// Grid cells used for the distance field of one expanded outline (at most).
const MAXIMUM_GRID_CELLS: f64 = 1.5e6;
/// Finest grid spacing of the distance field, in pixels.
const FINEST_SPACING: f64 = 0.5;

fn subtract(a: Point, b: Point) -> Point {
    (a.0 - b.0, a.1 - b.1)
}

fn cross(o: Point, a: Point, b: Point) -> f64 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

fn squared_distance(a: Point, b: Point) -> f64 {
    let (dx, dy) = subtract(a, b);
    dx * dx + dy * dy
}

/// Squared distance from `point` to the segment `start`-`end`, and the closest
/// point of the segment.
pub(super) fn segment_projection(point: Point, start: Point, end: Point) -> (f64, Point) {
    let (dx, dy) = subtract(end, start);
    let length = dx * dx + dy * dy;
    let t = if length > 0.0 {
        (((point.0 - start.0) * dx + (point.1 - start.1) * dy) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let closest = (start.0 + t * dx, start.1 + t * dy);
    (squared_distance(point, closest), closest)
}

fn squared_segment_distance(point: Point, start: Point, end: Point) -> f64 {
    segment_projection(point, start, end).0
}

/// Signed area (shoelace); positive for counter-clockwise in a y-up frame.
pub(super) fn signed_area(polygon: &[Point]) -> f64 {
    let n = polygon.len();
    (0..n)
        .map(|index| {
            let (a, b) = (polygon[index], polygon[(index + 1) % n]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum::<f64>()
        / 2.0
}

/// Even-odd point in polygon.
pub(super) fn contains(polygon: &[Point], point: Point) -> bool {
    let n = polygon.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut previous = polygon[n - 1];
    for &current in polygon {
        if (current.1 > point.1) != (previous.1 > point.1) {
            let x = previous.0
                + (point.1 - previous.1) / (current.1 - previous.1) * (current.0 - previous.0);
            if point.0 < x {
                inside = !inside;
            }
        }
        previous = current;
    }
    inside
}

/// Unique points in first-seen order (exact comparison, as R's `unique`).
pub(super) fn unique_points(points: &[Point]) -> Vec<Point> {
    let mut seen = std::collections::HashSet::with_capacity(points.len());
    points
        .iter()
        .copied()
        .filter(|point| seen.insert((point.0.to_bits(), point.1.to_bits())))
        .collect()
}

/// The ggforce shortcuts before a hull: at most two points as they are, points on
/// one line as their two ends. `None` when a polygon hull is needed.
fn degenerate_outline(points: &[Point]) -> Option<Vec<Point>> {
    if points.len() <= 2 {
        return Some(points.to_vec());
    }
    let first = points[0];
    // The point farthest from the first one gives the line direction.
    let far = points
        .iter()
        .copied()
        .max_by(|a, b| squared_distance(first, *a).total_cmp(&squared_distance(first, *b)))?;
    let length = squared_distance(first, far).sqrt();
    let tolerance = 1e-9 * length.max(1e-300);
    let collinear = points
        .iter()
        .all(|point| cross(first, far, *point).abs() / length.max(1e-300) <= tolerance);
    if !collinear {
        return None;
    }
    let mut direction = subtract(far, first);
    // Ends from low to high x (low to high y when vertical), as ggforce.
    if direction.0 < 0.0 || (direction.0 == 0.0 && direction.1 < 0.0) {
        direction = (-direction.0, -direction.1);
    }
    let position =
        |point: &Point| (point.0 - first.0) * direction.0 + (point.1 - first.1) * direction.1;
    let low = points
        .iter()
        .min_by(|a, b| position(a).total_cmp(&position(b)))?;
    let high = points
        .iter()
        .max_by(|a, b| position(a).total_cmp(&position(b)))?;
    Some(vec![*low, *high])
}

/// Convex hull (Andrew's monotone chain) without collinear points, in
/// counter-clockwise order of a y-up frame. Needs at least three non-collinear points
/// for a polygon.
pub(super) fn convex_hull(points: &[Point]) -> Vec<Point> {
    let mut sorted = points.to_vec();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    sorted.dedup();
    if sorted.len() < 3 {
        return sorted;
    }
    let mut hull: Vec<Point> = Vec::with_capacity(2 * sorted.len());
    for pass in 0..2 {
        let start = hull.len();
        let iterator: Box<dyn Iterator<Item = &Point>> = if pass == 0 {
            Box::new(sorted.iter())
        } else {
            Box::new(sorted.iter().rev())
        };
        for &point in iterator {
            while hull.len() >= start + 2
                && cross(hull[hull.len() - 2], hull[hull.len() - 1], point) <= 0.0
            {
                hull.pop();
            }
            hull.push(point);
        }
        hull.pop();
    }
    hull
}

/// A uniform grid over points, for the nearest-candidate searches of the concave hull.
struct PointGrid {
    origin: Point,
    cell: f64,
    columns: usize,
    rows: usize,
    cells: Vec<Vec<usize>>,
}

impl PointGrid {
    fn new(points: &[Point], members: impl Iterator<Item = usize>) -> Self {
        let (mut low, mut high) = (
            (f64::INFINITY, f64::INFINITY),
            (f64::NEG_INFINITY, f64::NEG_INFINITY),
        );
        for point in points {
            low = (low.0.min(point.0), low.1.min(point.1));
            high = (high.0.max(point.0), high.1.max(point.1));
        }
        let (width, height) = ((high.0 - low.0).max(1e-12), (high.1 - low.1).max(1e-12));
        // About four points per cell.
        let cell = ((width * height * 4.0 / points.len().max(1) as f64).sqrt()).max(1e-12);
        let columns = ((width / cell) as usize + 1).min(4096);
        let rows = ((height / cell) as usize + 1).min(4096);
        let cell = (width / columns as f64)
            .max(height / rows as f64)
            .max(1e-12)
            * 1.000001;
        let mut grid = PointGrid {
            origin: low,
            cell,
            columns,
            rows,
            cells: vec![Vec::new(); columns * rows],
        };
        for index in members {
            let (column, row) = grid.cell_of(points[index]);
            grid.cells[row * columns + column].push(index);
        }
        grid
    }

    fn cell_of(&self, point: Point) -> (usize, usize) {
        let column = ((point.0 - self.origin.0) / self.cell).floor().max(0.0) as usize;
        let row = ((point.1 - self.origin.1) / self.cell).floor().max(0.0) as usize;
        (column.min(self.columns - 1), row.min(self.rows - 1))
    }

    /// Indices of the points in cells overlapping the box.
    fn near(&self, low: Point, high: Point, mut visit: impl FnMut(usize)) {
        let (column_low, row_low) = self.cell_of(low);
        let (column_high, row_high) = self.cell_of(high);
        for row in row_low..=row_high {
            for column in column_low..=column_high {
                for &index in &self.cells[row * self.columns + column] {
                    visit(index);
                }
            }
        }
    }
}

/// Concave hull of `points` (concaveman, `length_threshold` 0). One or two points,
/// or points on one line, come back as they are or as the line's two ends.
pub(super) fn concave_hull(points: &[Point], concavity: f64) -> Vec<Point> {
    let points = unique_points(points);
    if let Some(outline) = degenerate_outline(&points) {
        return outline;
    }
    let hull = convex_hull(&points);
    // Hull vertices as indices into `points` (exact matches of hull points).
    let index_of: HashMap<(u64, u64), usize> = points
        .iter()
        .enumerate()
        .map(|(index, point)| ((point.0.to_bits(), point.1.to_bits()), index))
        .collect();
    let mut used = vec![false; points.len()];
    // A circular linked list of hull nodes: point index, previous, next.
    let mut node_point: Vec<usize> = Vec::with_capacity(hull.len() * 2);
    let mut previous: Vec<usize> = Vec::with_capacity(hull.len() * 2);
    let mut next: Vec<usize> = Vec::with_capacity(hull.len() * 2);
    let count = hull.len();
    for (position, point) in hull.iter().enumerate() {
        let index = index_of[&(point.0.to_bits(), point.1.to_bits())];
        used[index] = true;
        node_point.push(index);
        previous.push((position + count - 1) % count);
        next.push((position + 1) % count);
    }
    let grid = PointGrid::new(&points, (0..points.len()).filter(|index| !used[*index]));
    let squared_concavity = concavity * concavity;
    let mut queue: VecDeque<usize> = (0..count).collect();
    let mut candidates: Vec<(f64, usize)> = Vec::new();
    while let Some(node) = queue.pop_front() {
        let a = points[node_point[node]];
        let b = points[node_point[next[node]]];
        let squared_length = squared_distance(a, b);
        let maximum = squared_length / squared_concavity;
        let before = points[node_point[previous[node]]];
        let after = points[node_point[next[next[node]]]];
        // Candidates within `maximum` of the edge, nearest first.
        let reach = maximum.sqrt();
        candidates.clear();
        grid.near(
            (a.0.min(b.0) - reach, a.1.min(b.1) - reach),
            (a.0.max(b.0) + reach, a.1.max(b.1) + reach),
            |index| {
                if used[index] {
                    return;
                }
                let distance = squared_segment_distance(points[index], a, b);
                if distance <= maximum {
                    candidates.push((distance, index));
                }
            },
        );
        candidates.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)));
        let edges_cross = |from: Point, to: Point, skip: usize| {
            // Any hull edge (not touching `skip`, the hull end of the new edge)
            // that properly crosses from-to.
            let start = node;
            let mut current = start;
            loop {
                let (p, q) = (node_point[current], node_point[next[current]]);
                if p != skip && q != skip {
                    let (p, q) = (points[p], points[q]);
                    if (cross(p, q, from) > 0.0) != (cross(p, q, to) > 0.0)
                        && (cross(from, to, p) > 0.0) != (cross(from, to, q) > 0.0)
                    {
                        return true;
                    }
                }
                current = next[current];
                if current == start {
                    return false;
                }
            }
        };
        let mut chosen = None;
        for &(distance, index) in &candidates {
            let point = points[index];
            if distance < squared_segment_distance(point, before, a)
                && distance < squared_segment_distance(point, b, after)
                && !edges_cross(a, point, node_point[node])
                && !edges_cross(b, point, node_point[next[node]])
            {
                chosen = Some(index);
                break;
            }
        }
        let Some(index) = chosen else { continue };
        let point = points[index];
        if squared_distance(point, a).min(squared_distance(point, b)) <= maximum {
            used[index] = true;
            let new = node_point.len();
            node_point.push(index);
            previous.push(node);
            next.push(next[node]);
            let after_node = next[node];
            previous[after_node] = new;
            next[node] = new;
            queue.push_back(node);
            queue.push_back(new);
        }
    }
    let mut outline = Vec::with_capacity(node_point.len());
    let mut current = 0usize;
    loop {
        outline.push(points[node_point[current]]);
        current = next[current];
        if current == 0 {
            break;
        }
    }
    outline
}

/// An ellipse: centre, semi-axes and the angle of the first axis (radians).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Ellipse {
    pub(super) center: Point,
    pub(super) a: f64,
    pub(super) b: f64,
    pub(super) angle: f64,
}

/// Inverse of a symmetric 3x3 matrix (row-major); `None` when singular.
fn invert3(m: [f64; 9]) -> Option<[f64; 9]> {
    let c00 = m[4] * m[8] - m[5] * m[7];
    let c01 = m[5] * m[6] - m[3] * m[8];
    let c02 = m[3] * m[7] - m[4] * m[6];
    let determinant = m[0] * c00 + m[1] * c01 + m[2] * c02;
    if !determinant.is_finite() || determinant.abs() < 1e-300 {
        return None;
    }
    let inverse = 1.0 / determinant;
    Some([
        c00 * inverse,
        (m[2] * m[7] - m[1] * m[8]) * inverse,
        (m[1] * m[5] - m[2] * m[4]) * inverse,
        c01 * inverse,
        (m[0] * m[8] - m[2] * m[6]) * inverse,
        (m[2] * m[3] - m[0] * m[5]) * inverse,
        c02 * inverse,
        (m[1] * m[6] - m[0] * m[7]) * inverse,
        (m[0] * m[4] - m[1] * m[3]) * inverse,
    ])
}

/// Minimum enclosing ellipse of `points` (Khachiyan, stopping when the weights
/// move less than `tolerance`), scaled up afterwards if needed so that every point is
/// inside. One point gives a zero ellipse; points on one line an ellipse along
/// the line with semi-axes half its length and a tenth of that (ggforce).
pub(super) fn enclosing_ellipse(points: &[Point], tolerance: f64) -> Ellipse {
    let points = unique_points(points);
    let Some(&first) = points.first() else {
        return Ellipse {
            center: (f64::NAN, f64::NAN),
            a: 0.0,
            b: 0.0,
            angle: 0.0,
        };
    };
    if let Some(line) = degenerate_outline(&points) {
        let (start, end) = (line[0], *line.last().unwrap_or(&first));
        let half = squared_distance(start, end).sqrt() / 2.0;
        let a = half;
        return Ellipse {
            center: ((start.0 + end.0) / 2.0, (start.1 + end.1) / 2.0),
            a,
            b: a / 10.0,
            angle: if half > 0.0 {
                (end.1 - start.1).atan2(end.0 - start.0)
            } else {
                0.0
            },
        };
    }
    // The ellipse depends on the hull vertices only; centre them for precision.
    let hull = convex_hull(&points);
    let n = hull.len();
    let mean = (
        hull.iter().map(|p| p.0).sum::<f64>() / n as f64,
        hull.iter().map(|p| p.1).sum::<f64>() / n as f64,
    );
    let centered: Vec<Point> = hull.iter().map(|p| subtract(*p, mean)).collect();
    let mut weights = vec![1.0 / n as f64; n];
    let dimension = 2.0;
    for _ in 0..100_000 {
        let mut x = [0.0; 9];
        for (point, weight) in centered.iter().zip(&weights) {
            let q = [point.0, point.1, 1.0];
            for row in 0..3 {
                for column in 0..3 {
                    x[row * 3 + column] += weight * q[row] * q[column];
                }
            }
        }
        let Some(inverse) = invert3(x) else { break };
        let (mut best, mut best_value) = (0, f64::NEG_INFINITY);
        for (index, point) in centered.iter().enumerate() {
            let q = [point.0, point.1, 1.0];
            let mut value = 0.0;
            for row in 0..3 {
                for column in 0..3 {
                    value += q[row] * inverse[row * 3 + column] * q[column];
                }
            }
            if value > best_value {
                (best, best_value) = (index, value);
            }
        }
        let step = (best_value - dimension - 1.0) / ((dimension + 1.0) * (best_value - 1.0));
        let mut change = 0.0;
        for (index, weight) in weights.iter_mut().enumerate() {
            let updated = (1.0 - step) * *weight + if index == best { step } else { 0.0 };
            change += (updated - *weight) * (updated - *weight);
            *weight = updated;
        }
        if change.sqrt() < tolerance {
            break;
        }
    }
    let center = centered
        .iter()
        .zip(&weights)
        .fold((0.0, 0.0), |sum, (point, weight)| {
            (sum.0 + weight * point.0, sum.1 + weight * point.1)
        });
    let (mut sxx, mut sxy, mut syy) = (0.0, 0.0, 0.0);
    for (point, weight) in centered.iter().zip(&weights) {
        let (dx, dy) = subtract(*point, center);
        sxx += weight * dx * dx;
        sxy += weight * dx * dy;
        syy += weight * dy * dy;
    }
    // A = inverse(S) / d; the semi-axes are 1 / sqrt(eigenvalues of A), i.e.
    // sqrt(d x eigenvalues of S).
    let trace = sxx + syy;
    let difference = ((sxx - syy) * (sxx - syy) / 4.0 + sxy * sxy).sqrt();
    let (large, small) = (
        trace / 2.0 + difference,
        (trace / 2.0 - difference).max(0.0),
    );
    let angle = if sxy == 0.0 && sxx >= syy {
        0.0
    } else if sxy == 0.0 {
        std::f64::consts::FRAC_PI_2
    } else {
        (large - sxx).atan2(sxy)
    };
    let mut ellipse = Ellipse {
        center: (center.0 + mean.0, center.1 + mean.1),
        a: (dimension * large).sqrt(),
        b: (dimension * small).sqrt(),
        angle,
    };
    // Khachiyan stops early: grow the ellipse until it holds every point.
    let worst = hull
        .iter()
        .map(|point| ellipse_measure(&ellipse, *point))
        .fold(0.0, f64::max);
    if worst > 1.0 && worst.is_finite() {
        let scale = worst.sqrt() * (1.0 + 1e-12);
        ellipse.a *= scale;
        ellipse.b *= scale;
    }
    ellipse
}

/// `(x - c)' A (x - c)`: at most 1 inside the ellipse.
pub(super) fn ellipse_measure(ellipse: &Ellipse, point: Point) -> f64 {
    let (dx, dy) = subtract(point, ellipse.center);
    let (sin, cos) = ellipse.angle.sin_cos();
    let along = dx * cos + dy * sin;
    let across = -dx * sin + dy * cos;
    let term = |value: f64, axis: f64| {
        if axis > 0.0 {
            (value / axis).powi(2)
        } else if value.abs() <= 1e-9 {
            0.0
        } else {
            f64::INFINITY
        }
    };
    term(along, ellipse.a) + term(across, ellipse.b)
}

/// `n` points around the ellipse, as ggforce (`2 pi (i - 1) / n`); a zero ellipse
/// is its centre.
pub(super) fn ellipse_polygon(ellipse: &Ellipse, n: usize) -> Vec<Point> {
    if ellipse.a <= 0.0 && ellipse.b <= 0.0 {
        return vec![ellipse.center];
    }
    let (sin, cos) = ellipse.angle.sin_cos();
    let outline: Vec<Point> = (0..n)
        .map(|index| {
            let theta = 2.0 * std::f64::consts::PI * index as f64 / n as f64;
            let (x, y) = (theta.cos() * ellipse.a, theta.sin() * ellipse.b);
            (
                ellipse.center.0 + x * cos - y * sin,
                ellipse.center.1 + x * sin + y * cos,
            )
        })
        .collect();
    unique_points(&outline)
}

/// A convex piece of the mitred shape, or a segment (a point when both ends match).
enum Piece {
    Convex(Vec<Point>),
    Segment(Point, Point),
}

impl Piece {
    fn bounds(&self) -> (Point, Point) {
        let points: &[Point] = match self {
            Piece::Convex(points) => points,
            Piece::Segment(start, end) => &[*start, *end],
        };
        points.iter().fold(
            (
                (f64::INFINITY, f64::INFINITY),
                (f64::NEG_INFINITY, f64::NEG_INFINITY),
            ),
            |(low, high), point| {
                (
                    (low.0.min(point.0), low.1.min(point.1)),
                    (high.0.max(point.0), high.1.max(point.1)),
                )
            },
        )
    }

    /// Euclidean distance from `point` (0 inside).
    fn distance(&self, point: Point) -> f64 {
        match self {
            Piece::Segment(start, end) => squared_segment_distance(point, *start, *end).sqrt(),
            Piece::Convex(polygon) => {
                let n = polygon.len();
                let (mut positive, mut negative) = (false, false);
                let mut nearest = f64::INFINITY;
                for index in 0..n {
                    let (a, b) = (polygon[index], polygon[(index + 1) % n]);
                    let side = cross(a, b, point);
                    positive |= side > 0.0;
                    negative |= side < 0.0;
                    nearest = nearest.min(squared_segment_distance(point, a, b));
                }
                if positive && negative {
                    nearest.sqrt()
                } else {
                    0.0
                }
            }
        }
    }
}

fn normalized(vector: Point) -> Point {
    let length = (vector.0 * vector.0 + vector.1 * vector.1).sqrt();
    if length > 0.0 {
        (vector.0 / length, vector.1 / length)
    } else {
        (0.0, 0.0)
    }
}

/// The mitred offset of `outline` by `offset` as convex pieces (with the
/// outline's own edges): a point becomes a square, a segment a rectangle with
/// square ends, a polygon gets a strip per edge and a mitre per convex corner.
fn mitred_pieces(outline: &[Point], offset: f64) -> Vec<Piece> {
    let mut pieces = Vec::new();
    match outline {
        [] => {}
        [point] => {
            if offset > 0.0 {
                let (x, y) = *point;
                pieces.push(Piece::Convex(vec![
                    (x - offset, y - offset),
                    (x + offset, y - offset),
                    (x + offset, y + offset),
                    (x - offset, y + offset),
                ]));
            } else {
                pieces.push(Piece::Segment(*point, *point));
            }
        }
        [start, end] => {
            let direction = normalized(subtract(*end, *start));
            if offset > 0.0 {
                let (dx, dy) = (direction.0 * offset, direction.1 * offset);
                let (nx, ny) = (-dy, dx);
                pieces.push(Piece::Convex(vec![
                    (start.0 - dx - nx, start.1 - dy - ny),
                    (end.0 + dx - nx, end.1 + dy - ny),
                    (end.0 + dx + nx, end.1 + dy + ny),
                    (start.0 - dx + nx, start.1 - dy + ny),
                ]));
            } else {
                pieces.push(Piece::Segment(*start, *end));
            }
        }
        polygon => {
            let n = polygon.len();
            let orientation = signed_area(polygon).signum();
            let outward = |a: Point, b: Point| {
                let direction = normalized(subtract(b, a));
                (direction.1 * orientation, -direction.0 * orientation)
            };
            for index in 0..n {
                let (a, b) = (polygon[index], polygon[(index + 1) % n]);
                pieces.push(Piece::Segment(a, b));
                if offset <= 0.0 {
                    continue;
                }
                let normal = outward(a, b);
                let shift = (normal.0 * offset, normal.1 * offset);
                pieces.push(Piece::Convex(vec![
                    a,
                    b,
                    (b.0 + shift.0, b.1 + shift.1),
                    (a.0 + shift.0, a.1 + shift.1),
                ]));
                // The corner at b, between edge a-b and edge b-c.
                let c = polygon[(index + 2) % n];
                if cross(a, b, c) * orientation <= 0.0 {
                    continue; // reflex or straight: the strips already overlap
                }
                let other = outward(b, c);
                let first = (b.0 + normal.0 * offset, b.1 + normal.1 * offset);
                let second = (b.0 + other.0 * offset, b.1 + other.1 * offset);
                let denominator = 1.0 + normal.0 * other.0 + normal.1 * other.1;
                // ggforce's mitre limit is 1000 x the offset: square beyond it.
                if denominator > 2e-6 {
                    let tip = (
                        b.0 + offset * (normal.0 + other.0) / denominator,
                        b.1 + offset * (normal.1 + other.1) / denominator,
                    );
                    pieces.push(Piece::Convex(vec![b, first, tip, second]));
                } else {
                    let incoming = normalized(subtract(b, a));
                    let outgoing = normalized(subtract(c, b));
                    pieces.push(Piece::Convex(vec![
                        b,
                        first,
                        (first.0 + incoming.0 * offset, first.1 + incoming.1 * offset),
                        (
                            second.0 - outgoing.0 * offset,
                            second.1 - outgoing.1 * offset,
                        ),
                        second,
                    ]));
                }
            }
        }
    }
    pieces
}

/// ggforce's mark expansion: the outline offset by `expand - radius` with mitred
/// corners, then by `radius` with round corners (`radius` is capped at `expand`).
/// Returns the outer boundary; both 0 returns the outline as it is.
pub(super) fn expand_outline(outline: &[Point], expand: f64, radius: f64) -> Vec<Point> {
    let radius = radius.clamp(0.0, expand.max(0.0));
    let expand = expand.max(0.0);
    if outline.is_empty() || (expand == 0.0 && radius == 0.0) {
        return outline.to_vec();
    }
    let pieces = mitred_pieces(outline, expand - radius);
    let (mut low, mut high) = (
        (f64::INFINITY, f64::INFINITY),
        (f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for piece in &pieces {
        let (piece_low, piece_high) = piece.bounds();
        low = (low.0.min(piece_low.0), low.1.min(piece_low.1));
        high = (high.0.max(piece_high.0), high.1.max(piece_high.1));
    }
    let (width, height) = (high.0 - low.0 + 2.0 * radius, high.1 - low.1 + 2.0 * radius);
    let spacing = FINEST_SPACING.max((width * height / MAXIMUM_GRID_CELLS).sqrt());
    // Half a grid cell more than the radius: marching squares cut sharp inner
    // corners by up to that much, and every point must keep `expand` of room.
    let level = radius + 0.5 * spacing;
    let margin = level + 3.0 * spacing;
    let origin = (low.0 - margin, low.1 - margin);
    let columns = ((high.0 - low.0 + 2.0 * margin) / spacing).ceil() as usize + 1;
    let rows = ((high.1 - low.1 + 2.0 * margin) / spacing).ceil() as usize + 1;
    let far = level + 2.0 * spacing;
    let mut field = vec![f64::INFINITY; columns * rows];
    let at = |column: usize, row: usize| {
        (
            origin.0 + column as f64 * spacing,
            origin.1 + row as f64 * spacing,
        )
    };
    for piece in &pieces {
        let (piece_low, piece_high) = piece.bounds();
        let column_low = (((piece_low.0 - far - origin.0) / spacing).floor().max(0.0)) as usize;
        let column_high =
            ((((piece_high.0 + far - origin.0) / spacing).ceil()) as usize).min(columns - 1);
        let row_low = (((piece_low.1 - far - origin.1) / spacing).floor().max(0.0)) as usize;
        let row_high =
            ((((piece_high.1 + far - origin.1) / spacing).ceil()) as usize).min(rows - 1);
        for row in row_low..=row_high {
            for column in column_low..=column_high {
                let cell = &mut field[row * columns + column];
                if *cell > 0.0 {
                    *cell = cell.min(piece.distance(at(column, row)));
                }
            }
        }
    }
    // The outline's interior is inside (scanlines at grid rows).
    if outline.len() >= 3 {
        let n = outline.len();
        let mut crossings = Vec::new();
        for row in 0..rows {
            let y = origin.1 + row as f64 * spacing;
            crossings.clear();
            for index in 0..n {
                let (a, b) = (outline[index], outline[(index + 1) % n]);
                if (a.1 > y) != (b.1 > y) {
                    crossings.push(a.0 + (y - a.1) / (b.1 - a.1) * (b.0 - a.0));
                }
            }
            crossings.sort_by(f64::total_cmp);
            for pair in crossings.chunks_exact(2) {
                let start = ((pair[0] - origin.0) / spacing).ceil().max(0.0) as usize;
                let end = ((pair[1] - origin.0) / spacing).floor();
                if end < 0.0 {
                    continue;
                }
                let end = (end as usize).min(columns - 1);
                for column in start..=end {
                    field[row * columns + column] = 0.0;
                }
            }
        }
    }
    let loops = contours(&field, columns, rows, level);
    let Some(outer) = loops
        .into_iter()
        .max_by(|a, b| signed_area(a).abs().total_cmp(&signed_area(b).abs()))
    else {
        return outline.to_vec();
    };
    let outer: Vec<Point> = outer
        .into_iter()
        .map(|(x, y)| (origin.0 + x * spacing, origin.1 + y * spacing))
        .collect();
    simplify_closed(&outer, 0.02 * spacing.max(1.0) + 0.03)
}

/// Closed marching-squares contours of `field < level` (grid coordinates).
fn contours(field: &[f64], columns: usize, rows: usize, level: f64) -> Vec<Vec<Point>> {
    let value = |column: usize, row: usize| field[row * columns + column];
    let inside = |column: usize, row: usize| value(column, row) < level;
    // Edge keys: horizontal edge (column, row) to (column + 1, row) is 2 x index,
    // vertical edge (column, row) to (column, row + 1) is 2 x index + 1.
    let horizontal = |column: usize, row: usize| 2 * (row * columns + column);
    let vertical = |column: usize, row: usize| 2 * (row * columns + column) + 1;
    let crossing = |a: f64, b: f64| {
        let (a, b) = (a.min(1e300), b.min(1e300));
        if (b - a).abs() < 1e-300 {
            0.5
        } else {
            ((level - a) / (b - a)).clamp(0.0, 1.0)
        }
    };
    let position = |key: usize| -> Point {
        let index = key / 2;
        let (column, row) = (index % columns, index / columns);
        if key % 2 == 0 {
            let t = crossing(value(column, row), value(column + 1, row));
            (column as f64 + t, row as f64)
        } else {
            let t = crossing(value(column, row), value(column, row + 1));
            (column as f64, row as f64 + t)
        }
    };
    // Directed segments keep the inside on one side, so loops chain one way.
    let mut successor: HashMap<usize, usize> = HashMap::new();
    for row in 0..rows - 1 {
        for column in 0..columns - 1 {
            let corners = [
                inside(column, row),
                inside(column + 1, row),
                inside(column + 1, row + 1),
                inside(column, row + 1),
            ];
            let case = corners
                .iter()
                .enumerate()
                .fold(0, |code, (bit, flag)| code | (usize::from(*flag) << bit));
            if case == 0 || case == 15 {
                continue;
            }
            let top = horizontal(column, row);
            let right = vertical(column + 1, row);
            let bottom = horizontal(column, row + 1);
            let left = vertical(column, row);
            let center = (value(column, row)
                + value(column + 1, row)
                + value(column + 1, row + 1)
                + value(column, row + 1))
                / 4.0;
            // Segments (from, to) with the inside on the left of the direction
            // in grid coordinates (y down).
            let segments: &[(usize, usize)] = match case {
                1 => &[(left, top)],
                2 => &[(top, right)],
                3 => &[(left, right)],
                4 => &[(right, bottom)],
                5 if center < level => &[(left, bottom), (right, top)],
                5 => &[(left, top), (right, bottom)],
                6 => &[(top, bottom)],
                7 => &[(left, bottom)],
                8 => &[(bottom, left)],
                9 => &[(bottom, top)],
                10 if center < level => &[(top, left), (bottom, right)],
                10 => &[(top, right), (bottom, left)],
                11 => &[(bottom, right)],
                12 => &[(right, left)],
                13 => &[(right, top)],
                14 => &[(top, left)],
                _ => &[],
            };
            for &(from, to) in segments {
                successor.insert(from, to);
            }
        }
    }
    let mut loops = Vec::new();
    let mut keys: Vec<usize> = successor.keys().copied().collect();
    keys.sort_unstable();
    for start in keys {
        if !successor.contains_key(&start) {
            continue;
        }
        let mut ring = Vec::new();
        let mut current = start;
        while let Some(following) = successor.remove(&current) {
            ring.push(position(current));
            current = following;
            if current == start {
                break;
            }
        }
        if ring.len() >= 3 {
            loops.push(ring);
        }
    }
    loops
}

/// Douglas-Peucker on a closed ring.
fn simplify_closed(ring: &[Point], tolerance: f64) -> Vec<Point> {
    let n = ring.len();
    if n <= 4 {
        return ring.to_vec();
    }
    // Split at the vertex farthest from the first one.
    let far = (1..n)
        .max_by(|a, b| {
            squared_distance(ring[0], ring[*a]).total_cmp(&squared_distance(ring[0], ring[*b]))
        })
        .unwrap_or(n / 2);
    let mut keep = vec![false; n];
    keep[0] = true;
    keep[far] = true;
    let mut stack = vec![(0, far), (far, n)];
    let squared_tolerance = tolerance * tolerance;
    while let Some((start, end)) = stack.pop() {
        if end - start < 2 {
            continue;
        }
        let (a, b) = (ring[start], ring[end % n]);
        let (mut worst, mut worst_index) = (0.0, start);
        for (index, point) in ring.iter().enumerate().take(end).skip(start + 1) {
            let distance = squared_segment_distance(*point, a, b);
            if distance > worst {
                (worst, worst_index) = (distance, index);
            }
        }
        if worst > squared_tolerance {
            keep[worst_index] = true;
            stack.push((start, worst_index));
            stack.push((worst_index, end));
        }
    }
    ring.iter()
        .zip(keep)
        .filter_map(|(point, kept)| kept.then_some(*point))
        .collect()
}

/// Distance from `point` to the outline (its edges; a point or a segment as such),
/// and the closest outline point.
pub(super) fn closest_on_outline(outline: &[Point], point: Point) -> (f64, Point) {
    let n = outline.len();
    let mut best = (f64::INFINITY, point);
    for index in 0..n {
        let next = if n == 1 { 0 } else { (index + 1) % n };
        let (distance, closest) = segment_projection(point, outline[index], outline[next]);
        if distance < best.0 {
            best = (distance, closest);
        }
    }
    (best.0.sqrt(), best.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A seeded cloud: two blobs and a crescent, like UMAP clusters.
    fn cloud(seed: u64, n: usize) -> Vec<Point> {
        let mut state = seed;
        let mut next = || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 11) as f64 / (1u64 << 53) as f64
        };
        (0..n)
            .map(|index| {
                let (u, v) = (next(), next());
                match index % 3 {
                    0 => (100.0 + 40.0 * u, 100.0 + 30.0 * v),
                    1 => {
                        let angle = std::f64::consts::PI * u;
                        (
                            200.0 + 60.0 * angle.cos() * (1.0 + 0.2 * v),
                            150.0 + 60.0 * angle.sin(),
                        )
                    }
                    _ => (60.0 + 20.0 * u * u, 220.0 + 50.0 * v),
                }
            })
            .collect()
    }

    fn polygon_distance(polygon: &[Point], point: Point) -> f64 {
        if contains(polygon, point) {
            0.0
        } else {
            closest_on_outline(polygon, point).0
        }
    }

    #[test]
    fn convex_hull_of_a_square_with_inner_points() {
        let hull = convex_hull(&[
            (0.0, 0.0),
            (1.0, 0.0),
            (0.5, 0.5),
            (1.0, 1.0),
            (0.0, 1.0),
            (0.5, 0.0),
        ]);
        assert_eq!(hull.len(), 4);
        assert!(signed_area(&hull) > 0.0);
    }

    #[test]
    fn concave_hull_encloses_every_point_and_digs_in() {
        for concavity in [1.0, 2.0, 5.0] {
            let points = cloud(7, 900);
            let hull = concave_hull(&points, concavity);
            assert!(hull.len() >= 3);
            for point in &points {
                // On or inside the hull (vertices lie on it).
                assert!(polygon_distance(&hull, *point) < 1e-9, "{point:?} outside");
            }
        }
        // Concave hulls are smaller than the convex hull.
        let points = cloud(7, 900);
        let convex = signed_area(&convex_hull(&points)).abs();
        let concave = signed_area(&concave_hull(&points, 1.0)).abs();
        assert!(concave < 0.9 * convex, "{concave} vs {convex}");
    }

    #[test]
    fn concave_hull_is_simple() {
        let hull = concave_hull(&cloud(3, 600), 1.0);
        let n = hull.len();
        for i in 0..n {
            for j in i + 2..n {
                if i == 0 && j == n - 1 {
                    continue;
                }
                let (p, q) = (hull[i], hull[(i + 1) % n]);
                let (r, s) = (hull[j], hull[(j + 1) % n]);
                let proper = (cross(p, q, r) > 0.0) != (cross(p, q, s) > 0.0)
                    && (cross(r, s, p) > 0.0) != (cross(r, s, q) > 0.0);
                assert!(!proper, "edges {i} and {j} cross");
            }
        }
    }

    #[test]
    fn degenerate_hulls_follow_ggforce() {
        assert_eq!(concave_hull(&[(1.0, 2.0)], 2.0), vec![(1.0, 2.0)]);
        assert_eq!(
            concave_hull(&[(1.0, 2.0), (1.0, 2.0)], 2.0),
            vec![(1.0, 2.0)]
        );
        let line = concave_hull(&[(0.0, 0.0), (2.0, 2.0), (1.0, 1.0), (3.0, 3.0)], 2.0);
        assert_eq!(line, vec![(0.0, 0.0), (3.0, 3.0)]);
        let vertical = concave_hull(&[(1.0, 5.0), (1.0, 0.0), (1.0, 3.0)], 2.0);
        assert_eq!(vertical, vec![(1.0, 0.0), (1.0, 5.0)]);
    }

    #[test]
    fn expansion_keeps_points_at_least_expand_inside() {
        let points = cloud(11, 600);
        let hull = concave_hull(&points, 2.0);
        for (expand, radius) in [(18.9, 9.45), (10.0, 0.0), (10.0, 10.0), (5.0, 20.0)] {
            let expanded = expand_outline(&hull, expand, radius);
            for point in &points {
                assert!(contains(&expanded, *point));
                let margin = closest_on_outline(&expanded, *point).0;
                assert!(margin >= expand - 0.1, "{expand}/{radius}: margin {margin}");
            }
            // Not much larger either: the outline stays within expand of the hull.
            for vertex in &expanded {
                let distance = polygon_distance(&hull, *vertex);
                let limit = if radius >= expand {
                    expand
                } else {
                    expand * 3.0
                };
                assert!(distance <= limit + 0.5, "{distance} > {limit}");
            }
        }
    }

    #[test]
    fn rectangle_expansion_has_round_corners_of_radius() {
        let rectangle = vec![(0.0, 0.0), (100.0, 0.0), (100.0, 50.0), (0.0, 50.0)];
        let expanded = expand_outline(&rectangle, 10.0, 4.0);
        let (mut low, mut high) = (
            (f64::INFINITY, f64::INFINITY),
            (f64::NEG_INFINITY, f64::NEG_INFINITY),
        );
        for point in &expanded {
            low = (low.0.min(point.0), low.1.min(point.1));
            high = (high.0.max(point.0), high.1.max(point.1));
        }
        // Half a grid cell (0.25 px) of slack outside, see `expand_outline`.
        assert!((low.0 + 10.0).abs() < 0.3 && (high.0 - 110.0).abs() < 0.3);
        assert!((low.1 + 10.0).abs() < 0.3 && (high.1 - 60.0).abs() < 0.3);
        // The corner is a quarter circle of radius 4 around (104, 54).
        let corner = expanded
            .iter()
            .map(|point| squared_distance(*point, (110.0, 60.0)))
            .fold(f64::INFINITY, f64::min)
            .sqrt();
        let expected = 4.0 * 2f64.sqrt() - 4.0;
        assert!((corner - expected).abs() < 0.4, "{corner} vs {expected}");
        // Radius 0 keeps square corners.
        let square = expand_outline(&rectangle, 10.0, 0.0);
        let corner = square
            .iter()
            .map(|point| squared_distance(*point, (110.0, 60.0)))
            .fold(f64::INFINITY, f64::min)
            .sqrt();
        assert!(corner < 0.6, "{corner}");
    }

    #[test]
    fn points_and_segments_expand_to_rounded_boxes() {
        let point = expand_outline(&[(10.0, 10.0)], 8.0, 8.0);
        for vertex in &point {
            let radius = squared_distance(*vertex, (10.0, 10.0)).sqrt();
            assert!((radius - 8.0).abs() < 0.35, "{radius}");
        }
        let segment = expand_outline(&[(0.0, 0.0), (40.0, 0.0)], 5.0, 2.0);
        assert!(contains(&segment, (-4.0, 0.0)) && contains(&segment, (44.0, 4.0)));
        assert!(!contains(&segment, (0.0, 5.5)));
        assert_eq!(expand_outline(&[(1.0, 1.0)], 0.0, 0.0), vec![(1.0, 1.0)]);
    }

    #[test]
    fn enclosing_ellipse_holds_every_point_and_is_tight() {
        let rectangle = [(0.0, 0.0), (10.0, 0.0), (0.0, 4.0), (10.0, 4.0), (5.0, 2.0)];
        let ellipse = enclosing_ellipse(&rectangle, 0.001);
        for point in rectangle {
            assert!(ellipse_measure(&ellipse, point) <= 1.0 + 1e-9);
        }
        // The exact minimum ellipse of a 10 x 4 rectangle: semi-axes 5 sqrt 2, 2 sqrt 2.
        assert!((ellipse.a - 50f64.sqrt()).abs() < 0.1, "{ellipse:?}");
        assert!((ellipse.b - 8f64.sqrt()).abs() < 0.1, "{ellipse:?}");
        let points = cloud(5, 400);
        let ellipse = enclosing_ellipse(&points, 0.01);
        for point in &points {
            assert!(ellipse_measure(&ellipse, *point) <= 1.0 + 1e-9);
        }
    }

    #[test]
    fn degenerate_ellipses_follow_ggforce() {
        // ggforce: enclose_ellip_points(c(0, 10), c(0, 5)) gives a = 5.59017,
        // b = 0.559017, angle 0.4636476; one point a = b = 0.
        let ellipse = enclosing_ellipse(&[(0.0, 0.0), (10.0, 5.0)], 0.01);
        assert!((ellipse.a - 5.59017).abs() < 1e-5 && (ellipse.b - 0.559017).abs() < 1e-6);
        assert!((ellipse.angle - 0.4636476).abs() < 1e-6);
        assert_eq!(ellipse.center, (5.0, 2.5));
        let line = enclosing_ellipse(&[(0.0, 0.0), (10.0, 5.0), (20.0, 10.0)], 0.01);
        assert!((line.a - 11.18034).abs() < 1e-5);
        let point = enclosing_ellipse(&[(3.0, 4.0)], 0.01);
        assert_eq!((point.center, point.a, point.b), ((3.0, 4.0), 0.0, 0.0));
        assert_eq!(ellipse_polygon(&point, 100), vec![(3.0, 4.0)]);
    }
}
