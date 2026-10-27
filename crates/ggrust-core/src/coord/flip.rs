//! `coord_flip`: x vertical, y horizontal (lets-plot `coord_flip()`, or
//! `coord_cartesian(flip=True)`). The swap itself happens in [`super::Panel`],
//! which knows the scales; this coord places points like [`super::Cartesian`].
//! Axis titles and theme elements follow the aesthetic, as lets-plot: the x title
//! and `axis_text_x` go to the left axis.

use super::{Cartesian, Coord};
use crate::scene::Rect;

/// Flipped cartesian coordinates.
pub struct Flip;

impl Coord for Flip {
    fn name(&self) -> &'static str {
        "flip"
    }

    fn map(&self, x: f64, y: f64, panel: Rect) -> (f64, f64) {
        Cartesian.map(x, y, panel)
    }
}
