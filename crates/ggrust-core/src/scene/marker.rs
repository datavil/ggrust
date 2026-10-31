//! Marker geometry: lets-plot's point glyphs (`render/point/symbol/*Glyph.kt`) as
//! circles, rectangles, polygons and lines in pixel space. Both renderers draw these
//! parts, so SVG and PNG markers have one geometry.
//!
//! A marker's `size` is its full extent in pixels (lets-plot `NamedShape.size`,
//! stroke included); `stroke` is its outline width, which some glyphs use to fit
//! their parts. What it does not decide: fill and stroke colors (see
//! [`MarkerShape::style`]).

use std::f64::consts::PI;

use super::Rect;

/// The geometry of a marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarkerShape {
    /// A circle.
    Circle,
    /// An axis-aligned square.
    Square,
    /// A square rotated 45 degrees.
    Diamond,
    /// An equilateral triangle, pointing up or down. `centroid`: lets-plot's
    /// "pinned to centroid" placement (data points); otherwise centred in its box
    /// (legend keys).
    Triangle {
        /// Points up.
        up: bool,
        /// Pinned to the centroid.
        centroid: bool,
    },
    /// `+`.
    Plus,
    /// `x`, inscribed in the circle of the marker's size.
    Cross,
    /// A square with an `x` corner to corner.
    SquareCross,
    /// `+` over `x`.
    Star,
    /// A diamond with a `+`.
    DiamondPlus,
    /// A circle with a `+`.
    CirclePlus,
    /// Triangles up and down over each other.
    TriangleUpDown,
    /// A square with a `+`.
    SquarePlus,
    /// A circle with an `x`.
    CircleCross,
    /// A square with a triangle inscribed.
    SquareTriangle,
    /// lets-plot `TinyPointShape`: a 1 px square (missing shape values).
    Dot,
}

/// How a lets-plot shape code is painted (lets-plot `NamedShape`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerStyle {
    /// Outline only, in `color` (codes 0..14).
    Hollow,
    /// Filled with `color`, no outline (codes 15..20).
    Solid,
    /// Filled with `fill`, outlined in `color` (codes 21..25).
    Filled,
}

/// A drawable part of a marker.
#[derive(Debug, Clone, PartialEq)]
pub enum MarkerPart {
    /// A circle.
    Circle {
        /// Centre x.
        x: f64,
        /// Centre y.
        y: f64,
        /// Radius.
        radius: f64,
    },
    /// An axis-aligned rectangle.
    Rect(Rect),
    /// A closed polygon.
    Polygon(Vec<(f64, f64)>),
    /// A line segment (stroked only).
    Line((f64, f64), (f64, f64)),
}

impl MarkerShape {
    /// lets-plot shape code 0..25 (`None` for other codes). Triangles are pinned to
    /// their centroid, as on data points.
    pub fn from_code(code: u8) -> Option<(MarkerShape, MarkerStyle)> {
        use MarkerShape::*;
        use MarkerStyle::*;
        let up = Triangle {
            up: true,
            centroid: true,
        };
        let down = Triangle {
            up: false,
            centroid: true,
        };
        Some(match code {
            0 => (Square, Hollow),
            1 => (Circle, Hollow),
            2 => (up, Hollow),
            3 => (Plus, Hollow),
            4 => (Cross, Hollow),
            5 => (Diamond, Hollow),
            6 => (down, Hollow),
            7 => (SquareCross, Hollow),
            8 => (Star, Hollow),
            9 => (DiamondPlus, Hollow),
            10 => (CirclePlus, Hollow),
            11 => (TriangleUpDown, Hollow),
            12 => (SquarePlus, Hollow),
            13 => (CircleCross, Hollow),
            14 => (SquareTriangle, Hollow),
            15 => (Square, Solid),
            16 | 19 | 20 => (Circle, Solid),
            17 => (up, Solid),
            18 => (Diamond, Solid),
            21 => (Circle, Filled),
            22 => (Square, Filled),
            23 => (Diamond, Filled),
            24 => (up, Filled),
            25 => (down, Filled),
            _ => return None,
        })
    }

    /// The same shape centred in its box (lets-plot legend keys: `verticallyAligned`).
    pub fn box_aligned(self) -> MarkerShape {
        match self {
            MarkerShape::Triangle { up, .. } => MarkerShape::Triangle {
                up,
                centroid: false,
            },
            other => other,
        }
    }

    /// Visit the parts of a marker (see [`Self::parts`]) without allocating for the
    /// common single-part shapes: renderers call this once per data point.
    pub fn for_each_part(
        self,
        x: f64,
        y: f64,
        size: f64,
        stroke: f64,
        mut visit: impl FnMut(MarkerPart),
    ) {
        match self {
            MarkerShape::Circle => visit(circle(x, y, size)),
            MarkerShape::Square => visit(square(x, y, size)),
            _ => self.parts(x, y, size, stroke).into_iter().for_each(visit),
        }
    }

    /// The parts of a marker of `size` px with a `stroke` px outline at (`x`, `y`).
    pub fn parts(self, x: f64, y: f64, size: f64, stroke: f64) -> Vec<MarkerPart> {
        use MarkerShape::*;
        match self {
            Circle => vec![circle(x, y, size)],
            Square => vec![square(x, y, size)],
            Diamond => vec![diamond(x, y, size)],
            Triangle { up, centroid } => vec![triangle(x, y, size, stroke, up, centroid, false)],
            Plus => plus(x, y, size).to_vec(),
            Cross => cross(x, y, size, true).to_vec(),
            SquareCross => [vec![square(x, y, size)], cross(x, y, size, false).to_vec()].concat(),
            Star => [plus(x, y, size), cross(x, y, size, true)].concat(),
            DiamondPlus => [vec![diamond(x, y, size)], plus(x, y, size).to_vec()].concat(),
            CirclePlus => [vec![circle(x, y, size)], plus(x, y, size + stroke).to_vec()].concat(),
            TriangleUpDown => vec![
                triangle(x, y, size, stroke, true, true, false),
                triangle(x, y, size, stroke, false, true, false),
            ],
            SquarePlus => [vec![square(x, y, size)], plus(x, y, size + stroke).to_vec()].concat(),
            CircleCross => [
                vec![circle(x, y, size)],
                cross(x, y, size + stroke, true).to_vec(),
            ]
            .concat(),
            SquareTriangle => vec![
                square(x, y, size),
                triangle(x, y, size, stroke, true, false, true),
            ],
            Dot => vec![MarkerPart::Rect(Rect {
                x: x - 0.5,
                y: y - 0.5,
                width: 1.0,
                height: 1.0,
            })],
        }
    }
}

fn circle(x: f64, y: f64, size: f64) -> MarkerPart {
    MarkerPart::Circle {
        x,
        y,
        radius: size / 2.0,
    }
}

fn square(x: f64, y: f64, size: f64) -> MarkerPart {
    MarkerPart::Rect(Rect {
        x: x - size / 2.0,
        y: y - size / 2.0,
        width: size,
        height: size,
    })
}

fn diamond(x: f64, y: f64, size: f64) -> MarkerPart {
    let half = size / 2.0;
    MarkerPart::Polygon(vec![
        (x, y - half),
        (x + half, y),
        (x, y + half),
        (x - half, y),
    ])
}

fn plus(x: f64, y: f64, size: f64) -> [MarkerPart; 2] {
    let half = size / 2.0;
    [
        MarkerPart::Line((x - half, y), (x + half, y)),
        MarkerPart::Line((x, y - half), (x, y + half)),
    ]
}

/// lets-plot `CrossGlyph`: inscribed in the circle of `size` (`cos 45` of it) or
/// spanning the square.
fn cross(x: f64, y: f64, size: f64, inscribed_in_circle: bool) -> [MarkerPart; 2] {
    let width = if inscribed_in_circle {
        size * (PI / 4.0).cos()
    } else {
        size
    };
    let half = width / 2.0;
    [
        MarkerPart::Line((x - half, y - half), (x + half, y + half)),
        MarkerPart::Line((x - half, y + half), (x + half, y - half)),
    ]
}

/// lets-plot `TriangleGlyph.createTriangleShape`.
fn triangle(
    x: f64,
    y: f64,
    size: f64,
    stroke: f64,
    up: bool,
    pinned_to_centroid: bool,
    inscribed_in_square: bool,
) -> MarkerPart {
    let height = if inscribed_in_square {
        let outer_height = size + stroke;
        let stroke_section = 5f64.sqrt() * stroke;
        outer_height - stroke / 2.0 - stroke_section / 2.0
    } else {
        (PI / 3.0).sin() * size
    };
    let base = if inscribed_in_square { height } else { size };
    let half_height_offset = (height - size) / 2.0;
    let vertical_offset = if pinned_to_centroid {
        height / 12.0 - half_height_offset
    } else if inscribed_in_square {
        half_height_offset
    } else {
        -stroke / 4.0
    };
    let mut dy = (size - height) / 2.0;
    let points = if up {
        dy -= vertical_offset;
        [(base / 2.0, dy), (base, height + dy), (0.0, height + dy)]
    } else {
        dy += vertical_offset;
        [(0.0, dy), (base, dy), (base / 2.0, height + dy)]
    };
    let (origin_x, origin_y) = (x - base / 2.0, y - size / 2.0);
    MarkerPart::Polygon(
        points
            .iter()
            .map(|(px, py)| (px + origin_x, py + origin_y))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-9, "{actual} vs {expected}");
    }

    #[test]
    fn triangles_match_lets_plot_svg() {
        // lets-plot 4.11 `geom_point(shape=2)`: size 3 (6.6 px) + stroke 1.65 px.
        let MarkerPart::Polygon(points) =
            &MarkerShape::from_code(2)
                .unwrap()
                .0
                .parts(274.9826493898593, 177.0, 8.25, 1.65)[0]
        else {
            panic!("expected a polygon");
        };
        close(points[0].1, 172.2796075348982);
        close(points[1].0, 279.1076493898593);
        close(points[1].1, 179.42431711611982);
        close(points[2].0, 270.8576493898593);
    }

    #[test]
    fn square_triangles_and_crosses_match_lets_plot_svg() {
        // shape=14: square + inscribed triangle.
        let parts =
            MarkerShape::from_code(14)
                .unwrap()
                .0
                .parts(274.9826493898593, 177.0, 8.25, 1.65);
        let MarkerPart::Polygon(points) = &parts[1] else {
            panic!("expected a polygon");
        };
        close(points[0].1, 173.89475608143732);
        close(points[1].0, 278.59777134914066);
        close(points[1].1, 181.125);
        // shape=4: cross inscribed in the circle.
        let parts = MarkerShape::from_code(4)
            .unwrap()
            .0
            .parts(274.9826493898593, 177.0, 9.9, 1.65);
        let MarkerPart::Line(start, end) = parts[0] else {
            panic!("expected a line");
        };
        close(start.0, 271.48247082298593);
        close(end.1, 180.50017856687342);
    }

    #[test]
    fn every_lets_plot_code_has_a_shape() {
        for code in 0..=25 {
            assert!(MarkerShape::from_code(code).is_some(), "{code}");
        }
        assert!(MarkerShape::from_code(26).is_none());
    }
}
