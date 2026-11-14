//! `nudge_x`/`nudge_y` with `nudge_unit` on text-like geoms, as lets-plot's `NudgePos`.
//!
//! `nudge_unit="identity"` (default) adds the nudge in data units before the
//! coord maps the point. `"px"` adds it in pixels after the coord, `"size"` in
//! point diameters of `size=1` (2.2 px); a positive `nudge_y` moves up and a
//! positive `nudge_x` right, also under `coord_flip` (lets-plot adds the offset
//! in client space). What it does not do: the `position_nudge(unit=)` position,
//! which still works in data units only.

use super::point::POINT_UNIT_SIZE;
use crate::coord::Panel;
use crate::parameters::Parameters;
use crate::{Error, Result};

/// A layer's nudge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Nudge {
    /// Data units added before the coord (`identity`).
    data: (f64, f64),
    /// Pixels added after the coord (`px`, `size`), y already pointing down.
    pixels: (f64, f64),
}

impl Nudge {
    /// Reads `nudge_x`, `nudge_y` and `nudge_unit`.
    pub(crate) fn parse(parameters: &Parameters) -> Result<Nudge> {
        let x = parameters.number("nudge_x")?.unwrap_or(0.0);
        let y = parameters.number("nudge_y")?.unwrap_or(0.0);
        let pixels_per_unit = match parameters.string("nudge_unit")? {
            None | Some("identity") => {
                return Ok(Nudge {
                    data: (x, y),
                    pixels: (0.0, 0.0),
                });
            }
            Some("size") => POINT_UNIT_SIZE,
            Some("px") => 1.0,
            Some(other) => {
                return Err(Error::spec(
                    parameters.path_of("nudge_unit"),
                    format!("unknown nudge_unit '{other}'; expected 'identity', 'size' or 'px'"),
                ));
            }
        };
        Ok(Nudge {
            data: (0.0, 0.0),
            pixels: (x * pixels_per_unit, -y * pixels_per_unit),
        })
    }

    /// The data-unit part, added to x and y before mapping.
    pub(crate) fn data(&self) -> (f64, f64) {
        self.data
    }

    /// Moves an already mapped pixel point by the pixel part.
    pub(crate) fn shift(&self, (x, y): (f64, f64)) -> (f64, f64) {
        (x + self.pixels.0, y + self.pixels.1)
    }

    /// Maps the nudged data point `(x, y)` to pixels.
    pub(crate) fn map(&self, panel: &Panel, x: f64, y: f64) -> (f64, f64) {
        self.shift(self.anchor(panel, x, y))
    }

    /// The pixel point with only the data-unit part: where interactive zoom
    /// anchors the mark, so a pixel nudge stays the same number of pixels.
    pub(crate) fn anchor(&self, panel: &Panel, x: f64, y: f64) -> (f64, f64) {
        panel.map(x + self.data.0, y + self.data.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Options;
    use serde_json::json;

    fn parse(value: serde_json::Value) -> Result<Nudge> {
        let options: Options = serde_json::from_value(value).unwrap();
        Nudge::parse(&Parameters::new(&options, "layers[0]"))
    }

    #[test]
    fn identity_is_data_units() {
        let nudge = parse(json!({"nudge_x": 1.0, "nudge_y": -2.0})).unwrap();
        assert_eq!(nudge.data(), (1.0, -2.0));
        assert_eq!(nudge.shift((10.0, 10.0)), (10.0, 10.0));
    }

    #[test]
    fn pixels_point_up_and_right() {
        let nudge = parse(json!({"nudge_x": 3.0, "nudge_y": 4.0, "nudge_unit": "px"})).unwrap();
        assert_eq!(nudge.data(), (0.0, 0.0));
        assert_eq!(nudge.shift((10.0, 10.0)), (13.0, 6.0));
    }

    #[test]
    fn size_is_point_diameters() {
        let nudge = parse(json!({"nudge_y": 1.0, "nudge_unit": "size"})).unwrap();
        assert_eq!(nudge.shift((0.0, 0.0)), (0.0, -POINT_UNIT_SIZE));
    }

    #[test]
    fn unknown_unit_names_the_choices() {
        let error = parse(json!({"nudge_unit": "pt"})).unwrap_err().to_string();
        assert!(
            error.contains("nudge_unit") && error.contains("'px'"),
            "{error}"
        );
    }
}
