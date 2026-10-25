//! Coordinate systems: place scale-mapped positions (0..1) inside the panel.
//!
//! One file per coordinate system; add it to [`COORDS`].

mod cartesian;

pub use cartesian::Cartesian;

use crate::scale::PositionScale;
use crate::scene::Rect;

/// A coordinate system.
pub trait Coord: Sync {
    /// lets-plot name, e.g. `cartesian`.
    fn name(&self) -> &'static str;

    /// Pixel position of a point whose scaled coordinates are `x`, `y` in 0..1.
    fn map(&self, x: f64, y: f64, panel: Rect) -> (f64, f64);
}

/// Every coordinate system, in one place: lookup and names derive from it.
pub static COORDS: &[&dyn Coord] = &[&Cartesian];

/// Look up a coordinate system by its lets-plot name.
pub fn coord(name: &str) -> Option<&'static dyn Coord> {
    COORDS.iter().copied().find(|item| item.name() == name)
}

/// Names of all coordinate systems, for error messages.
pub fn names() -> Vec<&'static str> {
    COORDS.iter().map(|item| item.name()).collect()
}

/// The drawing area of one panel and how data maps into it.
pub struct Panel<'a> {
    /// Panel rectangle in scene pixels.
    pub rect: Rect,
    /// Horizontal position scale.
    pub x_scale: &'a PositionScale,
    /// Vertical position scale.
    pub y_scale: &'a PositionScale,
    /// Coordinate system.
    pub coord: &'a dyn Coord,
}

impl Panel<'_> {
    /// Pixel position of a data point. Missing values give NaN.
    pub fn map(&self, x: f64, y: f64) -> (f64, f64) {
        self.coord
            .map(self.x_scale.map(x), self.y_scale.map(y), self.rect)
    }
}
