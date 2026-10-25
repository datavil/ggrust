//! `coord_cartesian`: x to the right, y up.

use super::Coord;
use crate::scene::Rect;

/// Cartesian coordinates.
pub struct Cartesian;

impl Coord for Cartesian {
    fn name(&self) -> &'static str {
        "cartesian"
    }

    fn map(&self, x: f64, y: f64, panel: Rect) -> (f64, f64) {
        (
            panel.x + x * panel.width,
            panel.y + (1.0 - y) * panel.height,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn y_points_up() {
        let panel = Rect {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        assert_eq!(Cartesian.map(0.0, 0.0, panel), (10.0, 70.0));
        assert_eq!(Cartesian.map(1.0, 1.0, panel), (110.0, 20.0));
    }
}
