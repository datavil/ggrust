//! Coordinate system: `coord_cartesian`, `coord_fixed`, `coord_flip`.

use serde::Deserialize;

use super::Options;

/// One `coord_*()` call.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CoordSpec {
    /// `cartesian`, `fixed`, `flip`, ...
    pub name: String,
    /// x limits `[min, max]`; either may be null.
    #[serde(default)]
    pub xlim: Option<[Option<f64>; 2]>,
    /// y limits `[min, max]`; either may be null.
    #[serde(default)]
    pub ylim: Option<[Option<f64>; 2]>,
    /// Aspect ratio y/x for `coord_fixed`.
    #[serde(default)]
    pub ratio: Option<f64>,
    /// Swap x and y.
    #[serde(default)]
    pub flip: bool,
    /// Keys no component consumes.
    #[serde(flatten)]
    pub unsupported: Options,
}
