//! `coord_fixed`: cartesian with a fixed aspect ratio (lets-plot `FixedRatioCoordProvider`):
//! one y unit is `ratio` x units long on screen. The layout shrinks the panel to
//! keep it (see [`crate::layout::layout_plot`]).

use super::{Cartesian, Coord};
use crate::scene::Rect;

/// Fixed-ratio cartesian coordinates.
pub struct Fixed;

impl Coord for Fixed {
    fn name(&self) -> &'static str {
        "fixed"
    }

    fn map(&self, x: f64, y: f64, panel: Rect) -> (f64, f64) {
        Cartesian.map(x, y, panel)
    }

    fn fixes_aspect(&self) -> bool {
        true
    }
}
