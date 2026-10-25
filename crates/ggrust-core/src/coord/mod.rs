//! Coordinate systems: place scale-mapped positions (0..1) inside the panel.
//!
//! One file per coordinate system; register it in [`coord`].

mod cartesian;

pub use cartesian::Cartesian;

use crate::scale::ContinuousPositionScale;
use crate::scene::Rect;

/// A coordinate system.
pub trait Coord: Sync {
    /// lets-plot name, e.g. `cartesian`.
    fn name(&self) -> &'static str;

    /// Pixel position of a point whose scaled coordinates are `x`, `y` in 0..1.
    fn map(&self, x: f64, y: f64, panel: Rect) -> (f64, f64);
}

/// Look up a coordinate system by its lets-plot name.
pub fn coord(name: &str) -> Option<&'static dyn Coord> {
    match name {
        "cartesian" => Some(&Cartesian),
        _ => None,
    }
}

/// Names [`coord`] accepts, for error messages.
pub const NAMES: &[&str] = &["cartesian"];

/// The drawing area of one panel and how data maps into it.
pub struct Panel<'a> {
    /// Panel rectangle in scene pixels.
    pub rect: Rect,
    /// Horizontal position scale.
    pub x_scale: &'a ContinuousPositionScale,
    /// Vertical position scale.
    pub y_scale: &'a ContinuousPositionScale,
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
