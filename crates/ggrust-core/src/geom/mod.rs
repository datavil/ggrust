//! Geoms: turn a layer's (stat- and position-adjusted) data into scene nodes.
//! One file per geom; add it to [`GEOMS`].
//!
//! A geom declares what it needs (required aesthetics, style aesthetics with
//! defaults, extra parameters); the build stage validates the layer against that,
//! resolves styles (mapped -> constant -> default) and calls [`Geom::draw`].

mod point;

pub use point::PointGeom;

use crate::Result;
use crate::coord::Panel;
use crate::data::LayerData;
use crate::parameters::Parameters;
use crate::scene::Node;
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};

/// A geometric object.
pub trait Geom: Sync {
    /// lets-plot name, e.g. `point`.
    fn name(&self) -> &'static str;

    /// Stat used when the layer names none.
    fn default_stat(&self) -> &'static str {
        "identity"
    }

    /// Position used when the layer names none.
    fn default_position(&self) -> &'static str {
        "identity"
    }

    /// Position aesthetics that must be mapped.
    fn required_aesthetics(&self) -> &'static [Aesthetic];

    /// Style aesthetics the geom draws with, and their defaults.
    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)];

    /// Layer options besides style aesthetics that the geom reads.
    fn parameters(&self) -> &'static [&'static str] {
        &[]
    }

    /// Draw the layer into scene nodes (in panel pixel space).
    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>>;
}

/// Every geom, in one place: lookup and names derive from it.
pub static GEOMS: &[&dyn Geom] = &[&PointGeom];

/// Look up a geom by its lets-plot name.
pub fn geom(name: &str) -> Option<&'static dyn Geom> {
    GEOMS.iter().copied().find(|geom| geom.name() == name)
}

/// Names of all geoms, for error messages.
pub fn names() -> Vec<&'static str> {
    GEOMS.iter().map(|geom| geom.name()).collect()
}

/// Everything a geom needs to draw one layer.
pub struct DrawLayer<'a> {
    /// Position columns (continuous, in data units).
    pub data: &'a LayerData,
    /// Resolved style aesthetics; every entry of the geom's `style_defaults` is present.
    pub styles: &'a Styles,
    /// Layer options.
    pub parameters: Parameters<'a>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_names_round_trip() {
        for name in names() {
            assert_eq!(geom(name).map(|geom| geom.name()), Some(name));
        }
        let mut sorted = names();
        sorted.dedup();
        assert_eq!(sorted.len(), GEOMS.len(), "duplicate geom names");
    }

    #[test]
    fn style_defaults_match_aesthetic_kinds() {
        for geom in GEOMS {
            for (aesthetic, default) in geom.style_defaults() {
                assert_eq!(
                    aesthetic.kind().style(),
                    Some(default.kind()),
                    "geom_{}: default for '{aesthetic}'",
                    geom.name()
                );
            }
        }
    }
}
