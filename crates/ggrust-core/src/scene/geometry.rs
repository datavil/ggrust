//! Geometry rules shared by every renderer, so SVG and PNG agree on edge cases.
//!
//! - what counts as drawable (finite, positive sizes, SVG semantics)
//! - dash patterns normalised like SVG (odd lengths repeat, invalid means solid)
//! - huge coordinates clipped to a box around the scene, so no renderer sees values
//!   it cannot represent (tiny-skia drops paths beyond ~1e9 px; f32 overflows)

use super::{Rect, Stroke};

/// Margin around the scene inside which geometry is kept as is (pixels).
/// Far larger than any stroke, so clipping never changes what is visible.
pub const CLIP_MARGIN: f64 = 10_000.0;

/// The box geometry is clipped to: the scene grown by [`CLIP_MARGIN`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipBox {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

impl ClipBox {
    /// The clip box of a `width` x `height` scene.
    pub fn for_scene(width: f64, height: f64) -> Self {
        ClipBox {
            left: -CLIP_MARGIN,
            top: -CLIP_MARGIN,
            right: width + CLIP_MARGIN,
            bottom: height + CLIP_MARGIN,
        }
    }

    /// Whether a point lies inside the box.
    pub fn contains(&self, x: f64, y: f64) -> bool {
        (self.left..=self.right).contains(&x) && (self.top..=self.bottom).contains(&y)
    }

    /// Visible subpaths of a path: split at non-finite vertices, then clipped.
    /// Closed rings are clipped as polygons (keeps fills correct); open lines are
    /// clipped per segment and split where they leave the box.
    pub fn subpaths(&self, points: &[(f64, f64)], closed: bool) -> Vec<Vec<(f64, f64)>> {
        points
            .split(|(x, y)| !(x.is_finite() && y.is_finite()))
            .filter(|run| !run.is_empty())
            .flat_map(|run| {
                if closed {
                    let ring = self.clip_polygon(run);
                    if ring.len() >= 2 { vec![ring] } else { vec![] }
                } else {
                    self.clip_polyline(run)
                }
            })
            .collect()
    }

    /// Intersect a rectangle with the box; `None` when nothing remains.
    pub fn clip_rect(&self, rect: &Rect) -> Option<Rect> {
        if !is_drawable_rect(rect) {
            return None;
        }
        let left = rect.x.max(self.left);
        let top = rect.y.max(self.top);
        let right = (rect.x + rect.width).min(self.right);
        let bottom = (rect.y + rect.height).min(self.bottom);
        (right > left && bottom > top).then_some(Rect {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        })
    }

    fn clip_polyline(&self, run: &[(f64, f64)]) -> Vec<Vec<(f64, f64)>> {
        if run.len() == 1 {
            return if self.contains(run[0].0, run[0].1) {
                vec![run.to_vec()]
            } else {
                vec![]
            };
        }
        let mut lines: Vec<Vec<(f64, f64)>> = Vec::new();
        let mut current: Vec<(f64, f64)> = Vec::new();
        for pair in run.windows(2) {
            match self.clip_segment(pair[0], pair[1]) {
                Some((start, end)) => {
                    if current.last() != Some(&start) {
                        if current.len() >= 2 {
                            lines.push(std::mem::take(&mut current));
                        }
                        current.clear();
                        current.push(start);
                    }
                    current.push(end);
                }
                None => {
                    if current.len() >= 2 {
                        lines.push(std::mem::take(&mut current));
                    }
                    current.clear();
                }
            }
        }
        if current.len() >= 2 {
            lines.push(current);
        }
        lines
    }

    /// Liang-Barsky segment clipping.
    fn clip_segment(&self, start: (f64, f64), end: (f64, f64)) -> Option<((f64, f64), (f64, f64))> {
        let (dx, dy) = (end.0 - start.0, end.1 - start.1);
        let mut low = 0.0_f64;
        let mut high = 1.0_f64;
        for (p, q) in [
            (-dx, start.0 - self.left),
            (dx, self.right - start.0),
            (-dy, start.1 - self.top),
            (dy, self.bottom - start.1),
        ] {
            if p == 0.0 {
                if q < 0.0 {
                    return None;
                }
            } else {
                let ratio = q / p;
                if p < 0.0 {
                    low = low.max(ratio);
                } else {
                    high = high.min(ratio);
                }
            }
        }
        if low > high {
            return None;
        }
        let at = |t: f64| {
            if t == 0.0 {
                start
            } else if t == 1.0 {
                end
            } else {
                (start.0 + t * dx, start.1 + t * dy)
            }
        };
        Some((at(low), at(high)))
    }

    /// Sutherland-Hodgman polygon clipping against the four box edges.
    fn clip_polygon(&self, ring: &[(f64, f64)]) -> Vec<(f64, f64)> {
        if ring.iter().all(|&(x, y)| self.contains(x, y)) {
            return ring.to_vec();
        }
        let edges: [(InsideTest, Edge); 4] = [
            (|clip, (x, _)| x >= clip.left, Edge::Vertical(self.left)),
            (|clip, (x, _)| x <= clip.right, Edge::Vertical(self.right)),
            (|clip, (_, y)| y >= clip.top, Edge::Horizontal(self.top)),
            (
                |clip, (_, y)| y <= clip.bottom,
                Edge::Horizontal(self.bottom),
            ),
        ];
        let mut output = ring.to_vec();
        for (inside, edge) in edges {
            let input = std::mem::take(&mut output);
            for (index, &current) in input.iter().enumerate() {
                let previous = input[(index + input.len() - 1) % input.len()];
                match (inside(self, previous), inside(self, current)) {
                    (true, true) => output.push(current),
                    (true, false) => output.push(edge.intersect(previous, current)),
                    (false, true) => {
                        output.push(edge.intersect(previous, current));
                        output.push(current);
                    }
                    (false, false) => {}
                }
            }
            if output.is_empty() {
                break;
            }
        }
        output
    }
}

/// Whether a point is on the kept side of one box edge.
type InsideTest = fn(&ClipBox, (f64, f64)) -> bool;

#[derive(Clone, Copy)]
enum Edge {
    Vertical(f64),
    Horizontal(f64),
}

impl Edge {
    fn intersect(self, from: (f64, f64), to: (f64, f64)) -> (f64, f64) {
        match self {
            Edge::Vertical(x) => {
                let t = (x - from.0) / (to.0 - from.0);
                (x, from.1 + t * (to.1 - from.1))
            }
            Edge::Horizontal(y) => {
                let t = (y - from.1) / (to.1 - from.1);
                (from.0 + t * (to.0 - from.0), y)
            }
        }
    }
}

/// A rectangle is drawn only with finite values and positive width and height
/// (SVG semantics: zero or negative size disables rendering).
pub fn is_drawable_rect(rect: &Rect) -> bool {
    [rect.x, rect.y, rect.width, rect.height]
        .iter()
        .all(|value| value.is_finite())
        && rect.width > 0.0
        && rect.height > 0.0
}

/// A marker is drawn only at a finite position with a finite positive size.
pub fn is_drawable_marker(x: f64, y: f64, size: f64) -> bool {
    x.is_finite() && y.is_finite() && size.is_finite() && size > 0.0
}

/// A stroke is drawn only with a visible color and a finite positive width.
pub fn is_drawable_stroke(stroke: &Stroke) -> bool {
    !stroke.color.is_transparent() && is_drawable_width(stroke.width)
}

/// Whether a line width draws anything.
pub fn is_drawable_width(width: f64) -> bool {
    width.is_finite() && width > 0.0
}

impl Stroke {
    /// The dash pattern as renderers must apply it, or `None` for a solid line.
    ///
    /// SVG rules: any negative or non-finite length, or an all-zero pattern, means
    /// solid; an odd-length pattern is repeated to make it even.
    pub fn dash_pattern(&self) -> Option<Vec<f64>> {
        let valid = !self.dash.is_empty()
            && self
                .dash
                .iter()
                .all(|length| length.is_finite() && *length >= 0.0)
            && self.dash.iter().sum::<f64>() > 0.0;
        if !valid {
            return None;
        }
        let mut pattern = self.dash.clone();
        if pattern.len() % 2 == 1 {
            pattern.extend_from_within(..);
        }
        Some(pattern)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Color;

    const BOX: ClipBox = ClipBox {
        left: 0.0,
        top: 0.0,
        right: 100.0,
        bottom: 100.0,
    };

    #[test]
    fn polylines_split_at_gaps_and_clip_at_the_box() {
        let lines = BOX.subpaths(
            &[
                (10.0, 50.0),
                (50.0, 50.0),
                (50.0, 1e12),
                (f64::NAN, 0.0),
                (20.0, 20.0),
            ],
            false,
        );
        assert_eq!(
            lines,
            vec![
                vec![(10.0, 50.0), (50.0, 50.0), (50.0, 100.0)],
                vec![(20.0, 20.0)]
            ]
        );
    }

    #[test]
    fn slanted_segment_keeps_its_direction() {
        let lines = BOX.subpaths(&[(0.0, 0.0), (1e9, 1e9)], false);
        assert_eq!(lines, vec![vec![(0.0, 0.0), (100.0, 100.0)]]);
    }

    #[test]
    fn segment_leaving_and_reentering_splits() {
        let lines = BOX.subpaths(&[(10.0, 10.0), (10.0, 200.0), (20.0, 10.0)], false);
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn polygons_clip_to_the_box_and_rings_stay_separate() {
        let rings = BOX.subpaths(
            &[
                (50.0, 50.0),
                (1e9, 50.0),
                (50.0, 90.0),
                (f64::NAN, 0.0),
                (1.0, 1.0),
                (2.0, 1.0),
                (1.0, 2.0),
            ],
            true,
        );
        assert_eq!(rings.len(), 2);
        assert!(rings[0].iter().all(|&(x, y)| BOX.contains(x, y)));
        assert_eq!(rings[1], vec![(1.0, 1.0), (2.0, 1.0), (1.0, 2.0)]);
    }

    #[test]
    fn rects_clip_and_degenerate_rects_vanish() {
        let huge = Rect {
            x: 10.0,
            y: 10.0,
            width: 1e40,
            height: 50.0,
        };
        assert_eq!(
            BOX.clip_rect(&huge),
            Some(Rect {
                x: 10.0,
                y: 10.0,
                width: 90.0,
                height: 50.0
            })
        );
        let flat = Rect {
            x: 10.0,
            y: 10.0,
            width: 0.0,
            height: 50.0,
        };
        assert_eq!(BOX.clip_rect(&flat), None);
        let negative = Rect {
            x: 10.0,
            y: 10.0,
            width: -5.0,
            height: 50.0,
        };
        assert_eq!(BOX.clip_rect(&negative), None);
    }

    #[test]
    fn dash_patterns_follow_svg_rules() {
        let dash = |dash: Vec<f64>| {
            Stroke {
                color: Color::BLACK,
                width: 1.0,
                dash,
            }
            .dash_pattern()
        };
        assert_eq!(dash(vec![]), None);
        assert_eq!(dash(vec![10.0]), Some(vec![10.0, 10.0]));
        assert_eq!(
            dash(vec![10.0, 5.0, 2.0]),
            Some(vec![10.0, 5.0, 2.0, 10.0, 5.0, 2.0])
        );
        assert_eq!(dash(vec![0.0, 0.0]), None);
        assert_eq!(dash(vec![5.0, -1.0]), None);
        assert_eq!(dash(vec![5.0, f64::NAN]), None);
    }
}
