//! `coord_polar`: theta around the circle, radius outward (lets-plot
//! `PolarCoordProvider`, with ggplot2's closed circle where they differ).
//!
//! Geoms do not know about it. Layers draw on the flat panel, whose horizontal
//! pixels stand for theta and vertical pixels for the radius, and [`Warp`] bends the
//! finished nodes once: paths are resampled along each edge (ggplot2's `coord_munch`),
//! rectangles become closed paths (bars become wedges), markers and text keep their
//! shape and move with their anchor.
//!
//! What it does not do: bitmaps (`geom_raster`, `geom_imshow` are rejected when the
//! plot is prepared), interactive HTML, clipping to the circle (the panel rectangle
//! clips, as in ggplot2).

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use super::{Cartesian, Coord};
use crate::scale::PositionScale;
use crate::scene::{Group, Node, Path, Points, Raster, Rect, Rects, Text};
use crate::spec::CoordSpec;
use crate::{Error, Result};

/// lets-plot `R_EXPAND`: radius kept for labels and the axis ring.
pub const RADIUS_EXPAND: f64 = 0.15;
/// lets-plot `R_PADDING`: radius between the data and the ring.
pub const RADIUS_PADDING: f64 = 0.06;
/// Largest angle of one resampled step; at a radius of 300 px a chord leaves the
/// arc by 0.05 px.
const STEP: f64 = PI / 90.0;
/// Most steps of one edge: five turns, however far a zoomed edge leaves the panel.
const MAX_STEPS: f64 = 900.0;

/// Polar coordinates. Points are placed flat ([`Cartesian`]); see the module docs.
pub struct Polar;

impl Coord for Polar {
    fn name(&self) -> &'static str {
        "polar"
    }

    fn map(&self, x: f64, y: f64, panel: Rect) -> (f64, f64) {
        Cartesian.map(x, y, panel)
    }
}

/// The options of a `coord_polar()`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PolarView {
    /// Offset from 12 o'clock, radians.
    pub start: f64,
    /// Whether theta runs clockwise.
    pub clockwise: bool,
    /// Draw the panel background and border as a circle, not a rectangle.
    pub transform_background: bool,
}

impl PolarView {
    /// The view of `spec`, or `None` for another coord.
    pub fn from_spec(spec: Option<&CoordSpec>) -> Result<Option<PolarView>> {
        let Some(spec) = spec.filter(|spec| spec.name == "polar") else {
            return Ok(None);
        };
        let direction = spec.direction.unwrap_or(1.0);
        if direction != 1.0 && direction != -1.0 {
            return Err(Error::spec(
                "coord.direction",
                format!("expected 1 (clockwise) or -1 (counter-clockwise), got {direction}"),
            ));
        }
        Ok(Some(PolarView {
            start: spec.start.unwrap_or(0.0),
            clockwise: direction == 1.0,
            transform_background: spec.transform_bkgr.unwrap_or(true),
        }))
    }

    /// Whether `theta="y"` asks for the data y around the circle.
    pub fn theta_is_y(spec: &CoordSpec) -> Result<bool> {
        match spec.theta.as_deref() {
            None | Some("x") => Ok(false),
            Some("y") => Ok(true),
            Some(other) => Err(Error::spec(
                "coord.theta",
                format!("expected 'x' or 'y', got '{other}'"),
            )),
        }
    }

    /// Fit screen scales to the circle (lets-plot `adjustXYDomains`): no expansion;
    /// a discrete theta gets one more slot, so the last category does not land on
    /// the first; the radius keeps room for the ring and labels.
    pub fn adjust_scales(theta: &mut PositionScale, radius: &mut PositionScale) {
        let levels = theta.levels().len();
        let (low, high) = match levels > 0 && theta.limits() == [None, None] {
            true => (0.0, (levels - 1) as f64),
            false => theta.unexpanded_domain(),
        };
        theta.set_view_domain(low, high + f64::from(u8::from(theta.is_discrete())));
        let (low, high) = radius.unexpanded_domain();
        radius.set_view_domain(
            low,
            low + (high - low) * (1.0 + RADIUS_EXPAND + RADIUS_PADDING),
        );
    }

    /// The warp of a panel.
    pub fn warp(&self, panel: Rect) -> Warp {
        let sign = if self.clockwise { -1.0 } else { 1.0 };
        Warp {
            panel,
            center: (panel.x + panel.width / 2.0, panel.y + panel.height / 2.0),
            radius: panel.width.min(panel.height) / 2.0,
            sign,
            base: FRAC_PI_2 + sign * self.start,
        }
    }
}

/// Bends the flat panel into a circle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Warp {
    panel: Rect,
    /// Centre of the circle in pixels.
    pub center: (f64, f64),
    /// Radius at the top of the radial domain, in pixels.
    pub radius: f64,
    sign: f64,
    base: f64,
}

impl Warp {
    /// Radius of the ring, as a share of the radial domain.
    pub const RING: f64 = 1.0 / (1.0 + RADIUS_EXPAND);

    /// The pixel of `theta` and `radius`, both shares (0..1) of their domains.
    pub fn at(&self, theta: f64, radius: f64) -> (f64, f64) {
        let angle = self.sign * theta * TAU + self.base;
        let length = radius * self.radius;
        (
            self.center.0 + length * angle.cos(),
            self.center.1 - length * angle.sin(),
        )
    }

    /// The shares of a flat panel pixel.
    fn shares(&self, (x, y): (f64, f64)) -> (f64, f64) {
        (
            (x - self.panel.x) / self.panel.width,
            1.0 - (y - self.panel.y) / self.panel.height,
        )
    }

    /// Where a flat panel pixel goes.
    pub fn point(&self, pixel: (f64, f64)) -> (f64, f64) {
        let (theta, radius) = self.shares(pixel);
        self.at(theta, radius)
    }

    /// The shares of a path vertex: one at or past the top of the flat panel (a
    /// `geom_vline`, a rect to `Inf`) ends on the ring, not in the room for labels.
    fn path_shares(&self, pixel: (f64, f64)) -> (f64, f64) {
        let (theta, radius) = self.shares(pixel);
        (
            theta,
            if radius >= 1.0 - 1e-9 {
                Self::RING
            } else {
                radius
            },
        )
    }

    /// `points` bent: every edge resampled so arcs stay smooth.
    pub fn line(&self, points: &[(f64, f64)], closed: bool) -> Vec<(f64, f64)> {
        let mut bent = Vec::with_capacity(points.len());
        let edges = points.len().saturating_sub(usize::from(!closed));
        for index in 0..edges {
            let (from, to) = (points[index], points[(index + 1) % points.len()]);
            let (start, end) = (self.path_shares(from), self.path_shares(to));
            let steps = ((end.0 - start.0).abs() * TAU / STEP).ceil();
            let steps = if steps.is_finite() {
                steps.clamp(1.0, MAX_STEPS)
            } else {
                1.0
            };
            for step in 0..steps as usize {
                let share = step as f64 / steps;
                bent.push(self.at(
                    start.0 + (end.0 - start.0) * share,
                    start.1 + (end.1 - start.1) * share,
                ));
            }
        }
        if !closed && let Some(&last) = points.last() {
            let (theta, radius) = self.path_shares(last);
            bent.push(self.at(theta, radius));
        }
        bent
    }

    /// `nodes` bent.
    pub fn nodes(&self, nodes: Vec<Node>) -> Vec<Node> {
        let mut bent = Vec::with_capacity(nodes.len());
        for node in nodes {
            match node {
                Node::Group(group) => bent.push(Node::Group(Group {
                    clip: None,
                    anchor: None,
                    nodes: self.nodes(group.nodes),
                    ..group
                })),
                Node::Raster(raster) => bent.push(Node::Raster(Raster {
                    nodes: self.nodes(raster.nodes),
                    ..raster
                })),
                Node::Path(path) => bent.push(Node::Path(Path {
                    points: self.line(&path.points, path.closed),
                    ..path
                })),
                Node::Points(points) => bent.push(Node::Points(self.markers(points))),
                Node::Text(text) => {
                    let (x, y) = self.point((text.x, text.y));
                    bent.push(Node::Text(Text { x, y, ..text }));
                }
                Node::Rects(rects) => self.wedges(rects, &mut bent),
                image @ Node::Image(_) => bent.push(image),
            }
        }
        bent
    }

    fn markers(&self, mut points: Points) -> Points {
        for (x, y) in points.x.iter_mut().zip(points.y.iter_mut()) {
            (*x, *y) = self.point((*x, *y));
        }
        points
    }

    fn wedges(&self, rects: Rects, bent: &mut Vec<Node>) {
        for (index, rect) in rects.rects.iter().enumerate() {
            let corners = [
                (rect.x, rect.y),
                (rect.x + rect.width, rect.y),
                (rect.x + rect.width, rect.y + rect.height),
                (rect.x, rect.y + rect.height),
            ];
            bent.push(Node::Path(Path {
                points: self.line(&corners, true),
                closed: true,
                fill: rects.fill.get(index),
                stroke: rects.stroke.clone(),
            }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::PerItem;

    const PANEL: Rect = Rect {
        x: 10.0,
        y: 20.0,
        width: 200.0,
        height: 200.0,
    };

    fn view(clockwise: bool) -> PolarView {
        PolarView {
            start: 0.0,
            clockwise,
            transform_background: true,
        }
    }

    fn close(left: (f64, f64), right: (f64, f64)) -> bool {
        (left.0 - right.0).abs() < 1e-9 && (left.1 - right.1).abs() < 1e-9
    }

    #[test]
    fn theta_starts_at_twelve_and_runs_clockwise() {
        let warp = view(true).warp(PANEL);
        assert!(close(warp.at(0.0, 1.0), (110.0, 20.0)));
        assert!(close(warp.at(0.25, 1.0), (210.0, 120.0)));
        assert!(close(warp.at(0.0, 0.0), (110.0, 120.0)));
    }

    #[test]
    fn direction_and_start_turn_the_circle() {
        assert!(close(view(false).warp(PANEL).at(0.25, 1.0), (10.0, 120.0)));
        let turned = PolarView {
            start: FRAC_PI_2,
            ..view(true)
        };
        assert!(close(turned.warp(PANEL).at(0.0, 1.0), (210.0, 120.0)));
    }

    #[test]
    fn flat_top_right_corner_is_a_full_turn() {
        let warp = view(true).warp(PANEL);
        assert!(close(warp.point((210.0, 20.0)), (110.0, 20.0)));
    }

    #[test]
    fn horizontal_edges_become_arcs_and_vertical_edges_stay_straight() {
        let warp = view(true).warp(PANEL);
        let arc = warp.line(&[(10.0, 20.0), (110.0, 20.0)], false);
        assert_eq!(arc.len(), 91);
        assert!(
            arc.iter().all(|&(x, y)| {
                ((x - 110.0).powi(2) + (y - 120.0).powi(2)).sqrt() - 100.0 < 1e-9
            })
        );
        assert_eq!(warp.line(&[(10.0, 20.0), (10.0, 220.0)], false).len(), 2);
    }

    #[test]
    fn a_line_to_the_top_of_the_panel_ends_on_the_ring() {
        let warp = view(true).warp(PANEL);
        let spoke = warp.line(&[(10.0, 220.0), (10.0, 20.0)], false);
        assert!(close(spoke[1], (110.0, 120.0 - 100.0 * Warp::RING)));
    }

    #[test]
    fn rectangles_become_closed_wedges() {
        let warp = view(true).warp(PANEL);
        let mut nodes = Vec::new();
        warp.wedges(
            Rects {
                rects: vec![Rect {
                    x: 10.0,
                    y: 20.0,
                    width: 50.0,
                    height: 100.0,
                }],
                fill: PerItem::Uniform(None),
                stroke: None,
            },
            &mut nodes,
        );
        let Some(Node::Path(path)) = nodes.first() else {
            panic!("a rectangle becomes one path");
        };
        assert!(path.closed);
        assert!(path.points.len() > 4);
    }

    #[test]
    fn a_non_square_panel_keeps_a_round_circle() {
        let warp = view(true).warp(Rect {
            width: 400.0,
            ..PANEL
        });
        assert_eq!(warp.radius, 100.0);
        assert!(close(warp.at(0.25, 1.0), (310.0, 120.0)));
    }
}
