//! Coordinate system: `coord_cartesian`, `coord_fixed`, `coord_flip`, `coord_3d`.

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
    /// `false` drops the scales' expansion on both axes (ggplot2 `expand=FALSE`).
    #[serde(default)]
    pub expand: Option<bool>,
    /// `coord_3d`: view angle around the z axis, degrees.
    #[serde(default)]
    pub azimuth: Option<f64>,
    /// `coord_3d`: view angle above the x-y plane, degrees.
    #[serde(default)]
    pub elevation: Option<f64>,
    /// `coord_3d`: perspective projection instead of orthographic.
    #[serde(default)]
    pub perspective: Option<bool>,
    /// `coord_polar`: the aesthetic around the circle (`x` or `y`).
    #[serde(default)]
    pub theta: Option<String>,
    /// `coord_polar`: offset from 12 o'clock, radians.
    #[serde(default)]
    pub start: Option<f64>,
    /// `coord_polar`: 1 clockwise, -1 counter-clockwise.
    #[serde(default)]
    pub direction: Option<f64>,
    /// `coord_polar`: draw the background as a circle.
    #[serde(default)]
    pub transform_bkgr: Option<bool>,
    /// Keys no component consumes.
    #[serde(flatten)]
    pub unsupported: Options,
}
