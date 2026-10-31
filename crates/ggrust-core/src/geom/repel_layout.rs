//! The label force layout behind `geom_text_repel` / `geom_label_repel`: a
//! port of lets-plot `LabelForceLayout` (with `TransformedRectangle`,
//! `DoubleCircle` and `DoubleSegment.intersection`), so a seeded layout puts the
//! labels where lets-plot does when the text boxes measure the same.
//!
//! Labels start at their anchor, get a random kick away from their point, then
//! repel overlapping labels, points and leader segments for up to `max_iter`
//! iterations with friction and an ease-out factor; a label that overlaps
//! nothing is pulled back toward its point. Labels with more than
//! `max_overlaps` overlaps on the first repulsion pass are hidden.
//!
//! What it does not do: drawing (see `repel.rs`), data or scene types; it works on
//! pixel-space boxes and circles only.

use std::f64::consts::PI;
use std::time::Instant;

use crate::position::random::KotlinRandom;

/// A point or offset in pixels (lets-plot `DoubleVector`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector {
    pub x: f64,
    pub y: f64,
}

impl Vector {
    pub const ZERO: Vector = Vector { x: 0.0, y: 0.0 };

    pub const fn new(x: f64, y: f64) -> Self {
        Vector { x, y }
    }

    fn add(self, other: Vector) -> Vector {
        Vector::new(self.x + other.x, self.y + other.y)
    }

    fn subtract(self, other: Vector) -> Vector {
        Vector::new(self.x - other.x, self.y - other.y)
    }

    fn mul(self, factor: f64) -> Vector {
        Vector::new(self.x * factor, self.y * factor)
    }

    fn dot(self, other: Vector) -> f64 {
        self.x * other.x + self.y * other.y
    }

    fn length(self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    fn orthogonal(self) -> Vector {
        Vector::new(-self.y, self.x)
    }

    fn negate(self) -> Vector {
        Vector::new(-self.x, -self.y)
    }

    /// Kotlin `normalize()`: NaN for the zero vector.
    fn normalize(self) -> Vector {
        self.mul(1.0 / self.length())
    }

    /// lets-plot `savedNormalize`: the zero vector stays zero.
    fn saved_normalize(self) -> Vector {
        let length = self.length();
        if length == 0.0 {
            Vector::ZERO
        } else {
            self.mul(1.0 / length)
        }
    }

    /// Kotlin `Double.sign` per component (0 stays 0, unlike `f64::signum`).
    fn sign(self) -> Vector {
        let sign = |value: f64| {
            if value > 0.0 {
                1.0
            } else if value < 0.0 {
                -1.0
            } else {
                value
            }
        };
        Vector::new(sign(self.x), sign(self.y))
    }

    fn hadamard(self, other: Vector) -> Vector {
        Vector::new(self.x * other.x, self.y * other.y)
    }

    /// Rotate by `phi` radians around `center` (lets-plot `rotateAround`).
    pub fn rotate_around(self, center: Vector, phi: f64) -> Vector {
        let offset = self.subtract(center);
        let (sin, cos) = phi.sin_cos();
        Vector::new(
            offset.x * cos - offset.y * sin,
            offset.x * sin + offset.y * cos,
        )
        .add(center)
    }
}

/// A line segment (lets-plot `DoubleSegment`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub start: Vector,
    pub end: Vector,
}

impl Segment {
    pub const fn new(start: Vector, end: Vector) -> Self {
        Segment { start, end }
    }

    fn length(&self) -> f64 {
        self.start.subtract(self.end).length()
    }

    /// The crossing point with `other`, if any (lets-plot `intersection`; NaN
    /// parameters pass its range checks, as in Kotlin).
    #[allow(clippy::manual_range_contains)]
    fn intersection(&self, other: &Segment) -> Option<Vector> {
        let (o1, o2) = (self.start, other.start);
        let d1 = self.end.subtract(self.start);
        let d2 = other.end.subtract(other.start);
        let td = d1.dot(d2.orthogonal());
        if td == 0.0 {
            return None;
        }
        let t = o2.subtract(o1).dot(d2.orthogonal()) / td;
        if t < 0.0 || t > 1.0 {
            return None;
        }
        let sd = d2.dot(d1.orthogonal());
        let s = o1.subtract(o2).dot(d1.orthogonal()) / sd;
        if s < 0.0 || s > 1.0 {
            None
        } else {
            Some(o1.add(d1.mul(t)))
        }
    }

    fn intersects(&self, other: &Segment) -> bool {
        self.intersection(other).is_some()
    }
}

/// A circle (lets-plot `DoubleCircle`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circle {
    pub center: Vector,
    pub radius: f64,
}

impl Circle {
    fn intersects(&self, line: &Segment) -> bool {
        if line.start.subtract(self.center).length() <= self.radius
            || line.end.subtract(self.center).length() <= self.radius
        {
            return true;
        }
        let d = line.end.subtract(line.start);
        let f = line.start.subtract(self.center);
        let a = d.dot(d);
        let b = 2.0 * f.dot(d);
        let c = f.dot(f) - self.radius * self.radius;
        let root = (b * b - 4.0 * a * c).sqrt();
        let t1 = (-b - root) / (2.0 * a);
        let t2 = (-b + root) / (2.0 * a);
        (0.0..=1.0).contains(&t1) || (0.0..=1.0).contains(&t2)
    }
}

/// A possibly rotated rectangle by its corners (lets-plot `TransformedRectangle`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quad {
    pub left_top: Vector,
    pub left_bottom: Vector,
    pub right_bottom: Vector,
    pub right_top: Vector,
}

/// Axis-aligned bounds: `(left, top, right, bottom)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl Bounds {
    /// Inclusive on every side (lets-plot `DoubleRectangle.contains`).
    pub fn contains(&self, point: Vector) -> bool {
        self.left <= point.x
            && point.x <= self.right
            && self.top <= point.y
            && point.y <= self.bottom
    }
}

impl Quad {
    /// The rectangle `(left, top, width, height)` turned by `phi` radians around
    /// `around` (lets-plot `DoubleRectangle.rotate`).
    pub fn rotated(left: f64, top: f64, width: f64, height: f64, phi: f64, around: Vector) -> Self {
        let (right, bottom) = (left + width, top + height);
        let turn = |x: f64, y: f64| Vector::new(x, y).rotate_around(around, phi);
        Quad {
            left_top: turn(left, top),
            left_bottom: turn(left, bottom),
            right_bottom: turn(right, bottom),
            right_top: turn(right, top),
        }
    }

    fn width(&self) -> f64 {
        self.right_top.subtract(self.left_top).length()
    }

    fn height(&self) -> f64 {
        self.left_top.subtract(self.left_bottom).length()
    }

    fn hypot(&self) -> f64 {
        let (width, height) = (self.width(), self.height());
        (width * width + height * height).sqrt()
    }

    fn corners(&self) -> [Vector; 4] {
        [
            self.left_top,
            self.left_bottom,
            self.right_bottom,
            self.right_top,
        ]
    }

    pub fn bbox(&self) -> Bounds {
        let corners = self.corners();
        let fold = |pick: fn(&Vector) -> f64, min: bool| {
            corners.iter().map(pick).fold(
                if min {
                    f64::INFINITY
                } else {
                    f64::NEG_INFINITY
                },
                |acc, value| if min { acc.min(value) } else { acc.max(value) },
            )
        };
        Bounds {
            left: fold(|corner| corner.x, true),
            top: fold(|corner| corner.y, true),
            right: fold(|corner| corner.x, false),
            bottom: fold(|corner| corner.y, false),
        }
    }

    fn edges(&self) -> [Segment; 4] {
        [
            Segment::new(self.left_top, self.left_bottom),
            Segment::new(self.left_bottom, self.right_bottom),
            Segment::new(self.right_bottom, self.right_top),
            Segment::new(self.right_top, self.left_top),
        ]
    }

    pub fn translated(&self, offset: Vector) -> Quad {
        Quad {
            left_top: self.left_top.add(offset),
            left_bottom: self.left_bottom.add(offset),
            right_bottom: self.right_bottom.add(offset),
            right_top: self.right_top.add(offset),
        }
    }

    fn expand(&self, padding: f64) -> Quad {
        let up = self
            .left_top
            .subtract(self.left_bottom)
            .normalize()
            .mul(padding);
        let right = self
            .right_top
            .subtract(self.left_top)
            .normalize()
            .mul(padding);
        Quad {
            left_top: self.left_top.add(up).add(right.negate()),
            left_bottom: self.left_bottom.add(up.negate()).add(right.negate()),
            right_bottom: self.right_bottom.add(up.negate()).add(right),
            right_top: self.right_top.add(up).add(right),
        }
    }

    /// The point at (`hjust`, `vjust`) of the box, from its bottom-left corner.
    fn anchor(&self, hjust: f64, vjust: f64) -> Vector {
        let horizontal = self.right_bottom.subtract(self.left_bottom);
        let vertical = self.left_top.subtract(self.left_bottom);
        self.left_bottom
            .add(horizontal.mul(hjust))
            .add(vertical.mul(vjust))
    }

    fn intersects_circle(&self, circle: &Circle) -> bool {
        self.contains(circle.center) || self.edges().iter().any(|edge| circle.intersects(edge))
    }

    /// Separating-axis test.
    fn intersects_quad(&self, other: &Quad) -> bool {
        let axes = self
            .separating_axes()
            .into_iter()
            .chain(other.separating_axes());
        for axis in axes {
            let (a, b) = (self.project(axis), other.project(axis));
            if !(a.1 >= b.0 && b.1 >= a.0) {
                return false;
            }
        }
        true
    }

    fn intersects_segment(&self, segment: &Segment) -> bool {
        self.edges().iter().any(|edge| segment.intersects(edge))
            || self.contains(segment.start)
            || self.contains(segment.end)
    }

    /// The shortest segment from `point` to the centre of an edge it can reach
    /// without crossing another edge; `None` when the point is inside.
    pub fn segment_to_edge_center(&self, point: Vector) -> Option<Segment> {
        if self.contains(point) {
            return None;
        }
        let edges = self.edges();
        let mut best: Option<Segment> = None;
        for edge in &edges {
            let center = Vector::new(
                (edge.start.x + edge.end.x) / 2.0,
                (edge.start.y + edge.end.y) / 2.0,
            );
            let candidate = Segment::new(point, center);
            let crosses = edges
                .iter()
                .any(|other| other != edge && candidate.intersects(other));
            if !crosses && best.is_none_or(|best| candidate.length() < best.length()) {
                best = Some(candidate);
            }
        }
        best
    }

    /// lets-plot `pointInRectangle`.
    fn contains(&self, point: Vector) -> bool {
        let ab = self.left_bottom.subtract(self.left_top);
        let ad = self.right_top.subtract(self.left_top);
        let ap = point.subtract(self.left_top);
        let (ab_ap, ad_ap) = (ab.dot(ap), ad.dot(ap));
        0.0 <= ab_ap && ab_ap <= ab.dot(ab) && 0.0 <= ad_ap && ad_ap <= ad.dot(ad)
    }

    fn separating_axes(&self) -> [Vector; 4] {
        let normal = |vector: Vector| vector.orthogonal().saved_normalize();
        [
            normal(self.right_top.subtract(self.left_top)),
            normal(self.right_bottom.subtract(self.right_top)),
            normal(self.left_bottom.subtract(self.right_bottom)),
            normal(self.left_top.subtract(self.left_bottom)),
        ]
    }

    fn project(&self, axis: Vector) -> (f64, f64) {
        self.corners()
            .iter()
            .map(|corner| corner.dot(axis))
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), value| {
                (low.min(value), high.max(value))
            })
    }
}

/// Which way labels may move (`direction`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Both,
    X,
    Y,
}

impl Direction {
    /// lets-plot names: `both`, `x`, `y` (any case).
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "both" => Some(Direction::Both),
            "x" => Some(Direction::X),
            "y" => Some(Direction::Y),
            _ => None,
        }
    }
}

/// Layout options, lets-plot's defaults in [`Options::default`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Options {
    /// Labels with more overlaps after the first repulsion are hidden; negative
    /// keeps all.
    pub max_overlaps: i64,
    pub seed: i64,
    pub max_iter: i64,
    /// Milliseconds; 0 or less means no limit.
    pub max_time: f64,
    pub direction: Direction,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            max_overlaps: 10,
            seed: crate::position::random::DEFAULT_SEED,
            max_iter: 2000,
            max_time: 5000.0,
            direction: Direction::Both,
        }
    }
}

/// A label to place: its row, text box at the anchor, and its point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LabelInput {
    pub row: usize,
    pub quad: Quad,
    /// Extra space kept between labels (`box_padding` in pixels).
    pub padding: f64,
    pub hjust: f64,
    pub vjust: f64,
    /// The point, its radius grown by `point_padding`.
    pub point: Circle,
}

/// Where a label ended up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub row: usize,
    /// The label's anchor point (where its text is drawn).
    pub position: Vector,
    pub quad: Quad,
    pub hidden: bool,
}

struct Label {
    row: usize,
    quad: Quad,
    padding: f64,
    point: Vector,
    point_radius: f64,
    position: Vector,
    hidden: bool,
    clamped: bool,
    velocity: Vector,
}

const FRICTION: f64 = 0.7;

impl Label {
    fn d_length(&self) -> f64 {
        self.quad.hypot() / 2.0
    }

    fn expanded(&self) -> Quad {
        self.quad.expand(self.padding / 2.0)
    }

    fn segment(&self) -> Option<Segment> {
        self.quad.segment_to_edge_center(self.point)
    }

    fn set_force(&mut self, force: Vector) {
        self.velocity = self.velocity.mul(FRICTION).add(force);
        self.update_position(self.velocity);
    }

    fn update_position(&mut self, delta: Vector) {
        self.position = self.position.add(delta);
        self.quad = self.quad.translated(delta);
    }
}

/// The other item a label is checked against: a label (by index) or a point.
#[derive(Clone, Copy)]
enum Item {
    Label(usize),
    Point { row: usize, circle: Circle },
}

/// Place `labels` inside `bounds` away from each other and from `points` (every
/// point in the panel, including those without a label). Placements follow
/// `labels` order.
pub fn layout(
    labels: &[LabelInput],
    points: &[(usize, Circle)],
    bounds: Bounds,
    options: &Options,
) -> Vec<Placement> {
    let mut solver = Solver {
        labels: labels
            .iter()
            .map(|input| Label {
                row: input.row,
                quad: input.quad,
                padding: input.padding,
                point: input.point.center,
                point_radius: input.point.radius,
                position: input.quad.anchor(input.hjust, input.vjust),
                hidden: false,
                clamped: false,
                velocity: Vector::ZERO,
            })
            .collect(),
        items: (0..labels.len())
            .map(Item::Label)
            .chain(
                points
                    .iter()
                    .filter(|(_, circle)| circle.radius > 0.0)
                    .map(|&(row, circle)| Item::Point { row, circle }),
            )
            .collect(),
        bounds,
        direction: options.direction,
        random: KotlinRandom::new(options.seed),
    };
    solver.run(options);
    solver
        .labels
        .into_iter()
        .map(|label| Placement {
            row: label.row,
            position: label.position,
            quad: label.quad,
            hidden: label.hidden,
        })
        .collect()
}

struct Solver {
    labels: Vec<Label>,
    items: Vec<Item>,
    bounds: Bounds,
    direction: Direction,
    random: KotlinRandom,
}

const FIRST_REPULSION_ITERATION: i64 = 1;
const HIDE_ITERATION: i64 = 2;

impl Solver {
    fn run(&mut self, options: &Options) {
        let start = Instant::now();
        for iteration in 0..options.max_iter {
            if iteration == 0 {
                continue;
            }
            if iteration == FIRST_REPULSION_ITERATION {
                for index in 0..self.labels.len() {
                    let force = self.self_repulsion(index);
                    self.labels[index].set_force(force);
                }
                continue;
            }
            let ease = ease_out_quint(1.0 - iteration as f64 / options.max_iter as f64);
            let mut overlap_count = 0;
            for index in 0..self.labels.len() {
                if self.labels[index].hidden {
                    continue;
                }
                if iteration % 10 == 0 {
                    self.resolve_segment_intersection(index);
                }
                let (mut force, overlaps) = self.aggregate_forces(index);
                if iteration == HIDE_ITERATION
                    && options.max_overlaps >= 0
                    && overlaps > options.max_overlaps
                {
                    self.labels[index].hidden = true;
                    continue;
                }
                if overlaps == 0 {
                    force = self.self_attraction(index);
                }
                self.labels[index].set_force(force.mul(ease));
                self.clamp_to_bounds(index);
                overlap_count += overlaps;
            }
            if overlap_count == 0 {
                break;
            }
            if options.max_time > 0.0 && start.elapsed().as_secs_f64() * 1000.0 > options.max_time {
                break;
            }
        }
    }

    fn position_of(&self, item: Item) -> Vector {
        match item {
            Item::Label(index) => self.labels[index].position,
            Item::Point { circle, .. } => circle.center,
        }
    }

    fn aggregate_forces(&mut self, index: usize) -> (Vector, i64) {
        let mut force = Vector::ZERO;
        let mut overlaps = 0;
        for item_index in 0..self.items.len() {
            let item = self.items[item_index];
            match item {
                Item::Label(other) => {
                    if other == index || self.labels[other].hidden {
                        continue;
                    }
                    let (label, other_label) = (&self.labels[index], &self.labels[other]);
                    if label.expanded().intersects_quad(&other_label.expanded()) {
                        overlaps += 1;
                        force = force.add(self.repulsion(index, item));
                        continue;
                    }
                    if let Some(other_segment) = other_label.segment()
                        && label.quad.intersects_segment(&other_segment)
                    {
                        let delta = perpendicular_from_segment(label.position, &other_segment)
                            .saved_normalize();
                        force = force.add(self.apply_direction(delta));
                        overlaps += 1;
                    }
                    let (label, other_label) = (&self.labels[index], &self.labels[other]);
                    if let Some(segment) = label.segment()
                        && other_label.quad.intersects_segment(&segment)
                    {
                        let delta = perpendicular_from_segment(other_label.position, &segment)
                            .saved_normalize();
                        force = force.add(self.apply_direction(delta.negate()));
                        overlaps += 1;
                    }
                }
                Item::Point { circle, .. } => {
                    if self.labels[index].quad.intersects_circle(&circle) {
                        overlaps += 1;
                        force = force.add(self.repulsion(index, item));
                    }
                }
            }
        }
        (force, overlaps)
    }

    fn repulsion(&mut self, index: usize, other: Item) -> Vector {
        let position = self.labels[index].position;
        let other_position = self.position_of(other);
        let direction = self.non_zero_direction(position, other_position);
        let (other_length, other_row) = match other {
            Item::Label(other) => (self.labels[other].d_length(), self.labels[other].row),
            Item::Point { row, circle } => (circle.radius, row),
        };
        let reach = self.labels[index].d_length() + other_length;
        let distance = position.subtract(other_position).length();
        let mut value = reach / (reach + distance);
        if self.labels[index].row == other_row {
            value *= 2.0;
        }
        self.apply_direction(direction.negate().mul(value))
    }

    fn self_attraction(&mut self, index: usize) -> Vector {
        let (position, point) = (self.labels[index].position, self.labels[index].point);
        let direction = self.non_zero_direction(position, point);
        let distance = point.subtract(position).length();
        let reach = self.labels[index].d_length() + self.labels[index].point_radius;
        self.apply_direction(direction.mul(distance / reach))
    }

    fn self_repulsion(&mut self, index: usize) -> Vector {
        if self.labels[index].point_radius == 0.0 {
            return Vector::ZERO;
        }
        let kick = self.random_vector().mul(8.0);
        self.apply_direction(kick)
    }

    fn apply_direction(&self, force: Vector) -> Vector {
        match self.direction {
            Direction::X => Vector::new(force.x, 0.0),
            Direction::Y => Vector::new(0.0, force.y),
            Direction::Both => force,
        }
    }

    fn clamp_to_bounds(&mut self, index: usize) {
        let bbox = self.labels[index].quad.bbox();
        let bounds = self.bounds;
        let dx = if bbox.left < bounds.left {
            bounds.left - bbox.left
        } else if bbox.right > bounds.right {
            bounds.right - bbox.right
        } else {
            0.0
        };
        let dy = if bbox.top < bounds.top {
            bounds.top - bbox.top
        } else if bbox.bottom > bounds.bottom {
            bounds.bottom - bbox.bottom
        } else {
            0.0
        };
        let shift = Vector::new(dx, dy);
        if shift == Vector::ZERO {
            self.labels[index].clamped = false;
            return;
        }
        if self.labels[index].clamped {
            let size = Vector::new(bbox.right - bbox.left, bbox.bottom - bbox.top);
            let extra = shift.sign().hadamard(size);
            let delta = self.apply_direction(shift.add(extra));
            self.labels[index].update_position(delta);
        }
        let delta = self.apply_direction(shift);
        let label = &mut self.labels[index];
        label.update_position(delta);
        label.clamped = true;
    }

    /// Swap two labels whose leader lines cross.
    fn resolve_segment_intersection(&mut self, index: usize) {
        for other in 0..self.labels.len() {
            if other == index || self.labels[other].hidden {
                continue;
            }
            let (label, other_label) = (&self.labels[index], &self.labels[other]);
            if label.point == other_label.point {
                continue;
            }
            let segment = Segment::new(label.point, label.position);
            let other_segment = Segment::new(other_label.point, other_label.position);
            if segment.intersects(&other_segment) {
                let delta = other_label.position.subtract(label.position);
                self.labels[index].update_position(delta);
                self.labels[other].update_position(delta.mul(-1.0));
            }
        }
    }

    fn non_zero_direction(&mut self, from: Vector, to: Vector) -> Vector {
        let direction = to.subtract(from).saved_normalize();
        if direction == Vector::ZERO {
            self.random_vector()
        } else {
            direction
        }
    }

    fn random_vector(&mut self) -> Vector {
        let angle = self.random.next_double() * 2.0 * PI;
        Vector::new(angle.cos(), angle.sin())
    }
}

/// The offset from the closest point of `segment` to `point`.
fn perpendicular_from_segment(point: Vector, segment: &Segment) -> Vector {
    let ab = segment.end.subtract(segment.start);
    let ap = point.subtract(segment.start);
    let length_squared = ab.dot(ab);
    if length_squared == 0.0 {
        return segment.start.subtract(point);
    }
    let t = (ap.dot(ab) / length_squared).clamp(0.0, 1.0);
    point.subtract(segment.start.add(ab.mul(t)))
}

fn ease_out_quint(x: f64) -> f64 {
    1.0 - (1.0 - x).powf(5.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad_at(x: f64, y: f64, width: f64, height: f64) -> Quad {
        Quad::rotated(
            x - width / 2.0,
            y - height / 2.0,
            width,
            height,
            0.0,
            Vector::new(x, y),
        )
    }

    fn input(row: usize, x: f64, y: f64) -> LabelInput {
        LabelInput {
            row,
            quad: quad_at(x, y, 40.0, 14.0),
            padding: 0.0,
            hjust: 0.5,
            vjust: 0.5,
            point: Circle {
                center: Vector::new(x, y),
                radius: 3.3,
            },
        }
    }

    const BOUNDS: Bounds = Bounds {
        left: 0.0,
        top: 0.0,
        right: 400.0,
        bottom: 300.0,
    };

    #[test]
    fn overlapping_labels_end_apart_and_inside() {
        let labels: Vec<LabelInput> = (0..5)
            .map(|row| input(row, 200.0 + row as f64, 150.0))
            .collect();
        let points: Vec<(usize, Circle)> = labels
            .iter()
            .map(|label| (label.row, label.point))
            .collect();
        let placed = layout(&labels, &points, BOUNDS, &Options::default());
        for (index, first) in placed.iter().enumerate() {
            assert!(!first.hidden);
            assert!(BOUNDS.contains(first.position));
            for second in &placed[index + 1..] {
                assert!(
                    !first.quad.intersects_quad(&second.quad),
                    "{first:?} {second:?}"
                );
            }
        }
    }

    #[test]
    fn seeded_layout_repeats() {
        let labels: Vec<LabelInput> = (0..4).map(|row| input(row, 100.0, 100.0)).collect();
        let options = Options {
            seed: 7,
            ..Options::default()
        };
        let first = layout(&labels, &[], BOUNDS, &options);
        assert_eq!(first, layout(&labels, &[], BOUNDS, &options));
    }

    #[test]
    fn crowded_labels_are_hidden_past_max_overlaps() {
        let labels: Vec<LabelInput> = (0..6).map(|row| input(row, 100.0, 100.0)).collect();
        let options = Options {
            max_overlaps: 2,
            ..Options::default()
        };
        let placed = layout(&labels, &[], BOUNDS, &options);
        assert!(placed.iter().any(|placement| placement.hidden));
        let all = layout(
            &labels,
            &[],
            BOUNDS,
            &Options {
                max_overlaps: -1,
                ..Options::default()
            },
        );
        assert!(all.iter().all(|placement| !placement.hidden));
    }

    #[test]
    fn direction_x_keeps_y() {
        let labels: Vec<LabelInput> = (0..3).map(|row| input(row, 200.0, 150.0)).collect();
        let options = Options {
            direction: Direction::X,
            ..Options::default()
        };
        for placement in layout(&labels, &[], BOUNDS, &options) {
            assert_eq!(placement.position.y, 150.0);
        }
    }

    #[test]
    fn edge_segment_skips_crossing_edges() {
        let quad = quad_at(100.0, 100.0, 40.0, 14.0);
        let segment = quad
            .segment_to_edge_center(Vector::new(100.0, 130.0))
            .unwrap();
        assert_eq!(segment.end, Vector::new(100.0, 107.0));
        assert!(
            quad.segment_to_edge_center(Vector::new(100.0, 100.0))
                .is_none()
        );
    }

    /// lets-plot 4.11 `to_svg()` of `geom_point() + geom_text_repel(seed=1)` over
    /// six labels: its points, text widths (from `geom_label`) and panel give
    /// exactly its label positions.
    #[test]
    fn matches_lets_plot_with_its_text_widths() {
        let points = [
            (24.9984226718054, 330.13570487483526),
            (49.38712771746924, 322.3623188405797),
            (73.775832763133, 337.9090909090909),
            (268.8854731284434, 174.6679841897233),
            (512.7725235850813, 19.20026350461137),
            (524.9668761079132, 16.090909090909122),
        ];
        let widths = [
            39.84175152592688,
            31.247253343114977,
            50.262988807104335,
            36.71101213779417,
            53.90000850341801,
            31.247253343115005,
        ];
        let expected = [
            (39.306897067033375, 347.0),
            (64.8549807083088, 307.3252413468336),
            (91.16936024175003, 321.1920946330329),
            (297.2104822655157, 167.6153953420245),
            (474.17896508644174, 32.67142038191407),
            (519.0432598247779, 38.89739218085996),
        ];
        let labels: Vec<LabelInput> = points
            .iter()
            .zip(widths)
            .enumerate()
            .map(|(row, (&(x, y), width))| LabelInput {
                quad: quad_at(x, y, width, 14.0),
                ..input(row, x, y)
            })
            .collect();
        let circles: Vec<(usize, Circle)> = labels
            .iter()
            .map(|label| (label.row, label.point))
            .collect();
        let bounds = Bounds {
            left: 0.0,
            top: 0.0,
            right: 549.9652987797186,
            bottom: 354.0,
        };
        let options = Options {
            seed: 1,
            ..Options::default()
        };
        let placed = layout(&labels, &circles, bounds, &options);
        for (placement, (x, y)) in placed.iter().zip(expected) {
            assert!(!placement.hidden);
            let (dx, dy) = (placement.position.x - x, placement.position.y - y);
            assert!(
                dx.abs() < 1e-6 && dy.abs() < 1e-6,
                "{placement:?} vs ({x}, {y})"
            );
        }
    }

    /// The same points with `geom_label_repel(seed=42, box_padding=0.4,
    /// point_padding=0.3, max_overlaps=50, nudge_y=0.05)` (cellestial's volcano
    /// options): label boxes pad 3.5 px, starts sit 0.05 y units above the points.
    #[test]
    fn matches_lets_plot_labels_with_paddings() {
        let points = [
            (24.9984226718054, 330.13570487483526),
            (49.38712771746924, 322.3623188405797),
            (73.775832763133, 337.9090909090909),
            (268.8854731284434, 174.6679841897233),
            (512.7725235850813, 19.20026350461137),
            (524.9668761079132, 16.090909090909122),
        ];
        let widths = [
            39.84175152592688,
            31.247253343114977,
            50.262988807104335,
            36.71101213779417,
            53.90000850341801,
            31.247253343115005,
        ];
        let expected = [
            (27.898117270889152, 297.94223232381364),
            (71.1193108276086, 318.26628809335574),
            (120.48688481056935, 340.49663759757664),
            (292.63319643928907, 161.12827244323265),
            (478.96583637025805, 11.33314656659368),
            (521.5734086503392, 34.29004247388145),
        ];
        // y pixels per data unit, from the points at y = 1 and y = 2.
        let nudge = 0.05 * (points[0].1 - points[3].1);
        let labels: Vec<LabelInput> = points
            .iter()
            .zip(widths)
            .enumerate()
            .map(|(row, (&(x, y), width))| LabelInput {
                row,
                quad: quad_at(x, y - nudge, width + 7.0, 21.0),
                padding: 0.4 * 2.2,
                hjust: 0.5,
                vjust: 0.5,
                point: Circle {
                    center: Vector::new(x, y),
                    radius: 3.3 + 0.3 * 2.2,
                },
            })
            .collect();
        let circles: Vec<(usize, Circle)> = labels
            .iter()
            .map(|label| (label.row, label.point))
            .collect();
        let bounds = Bounds {
            left: 0.0,
            top: 0.0,
            right: 549.9652987797186,
            bottom: 354.0,
        };
        let options = Options {
            seed: 42,
            max_overlaps: 50,
            ..Options::default()
        };
        let placed = layout(&labels, &circles, bounds, &options);
        for (placement, (x, y)) in placed.iter().zip(expected) {
            assert!(!placement.hidden);
            let (dx, dy) = (placement.position.x - x, placement.position.y - y);
            assert!(
                dx.abs() < 1e-6 && dy.abs() < 1e-6,
                "{placement:?} vs ({x}, {y})"
            );
        }
    }

    #[test]
    fn kotlin_sign_keeps_zero() {
        assert_eq!(Vector::new(0.0, -2.0).sign(), Vector::new(0.0, -1.0));
    }
}
