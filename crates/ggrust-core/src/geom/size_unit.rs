//! `size_unit`: sizes relative to one unit of an axis instead of pixels.
//!
//! Follows lets-plot `AesScaling.sizeUnitRatio`: with `size_unit="x"` (or `"y"`),
//! a size is multiplied by (pixels of one data unit along that axis) / (the geom's
//! pixels per unit of size), so `size=1` spans one axis unit: a point's diameter
//! is one unit, a text's font is a sixth of a unit per unit of size times 2 px.
//! On a discrete axis one unit is the distance between two categories.
//!
//! What it does not do: units of a non-linear (log) axis at each row; the unit is
//! measured between data 0 and 1 of the axis scale.

use crate::coord::Panel;
use crate::parameters::Parameters;
use crate::{Error, Result};

/// Multiplier of the layer's sizes: 1 without `size_unit`.
///
/// `unit_size` is the geom's pixels per unit of size (lets-plot passes 2.2 for
/// points and 6 for texts). Errors on a `size_unit` other than `"x"` or `"y"`.
pub(super) fn size_unit_ratio(
    parameters: &Parameters,
    panel: &Panel,
    unit_size: f64,
) -> Result<f64> {
    let Some(text) = parameters.string("size_unit")? else {
        return Ok(1.0);
    };
    let (start, end) = match text.to_ascii_lowercase().as_str() {
        "x" => (panel.map(0.0, 0.0), panel.map(1.0, 0.0)),
        "y" => (panel.map(0.0, 0.0), panel.map(0.0, 1.0)),
        _ => {
            return Err(Error::spec(
                parameters.path_of("size_unit"),
                format!(
                    "unknown size_unit '{text}'; expected 'x' or 'y'{}",
                    crate::suggest::did_you_mean(text, ["x", "y"])
                ),
            ));
        }
    };
    let pixels = (end.0 - start.0).hypot(end.1 - start.1);
    Ok(if pixels.is_finite() {
        pixels / unit_size
    } else {
        1.0
    })
}
