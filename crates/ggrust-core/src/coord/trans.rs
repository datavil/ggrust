//! `coord_trans` / `coord_transform` (ggplot2): cartesian coordinates whose axes
//! are transformed after the stats. The warp itself lives on the continuous
//! position scales ([`crate::scale::ContinuousPositionScale::coord_transform`]),
//! so every geom, grid line and tick maps through it; here the panel is plain
//! cartesian.
//!
//! What it does not do: bend straight segments between data points (ggplot2
//! `coord_munch`); lines are drawn straight between their warped points.

use super::{Cartesian, Coord};
use crate::scene::Rect;

/// Cartesian coordinates over transformed axes.
pub struct Trans;

impl Coord for Trans {
    fn name(&self) -> &'static str {
        "trans"
    }

    fn map(&self, x: f64, y: f64, panel: Rect) -> (f64, f64) {
        Cartesian.map(x, y, panel)
    }
}
