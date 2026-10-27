//! Geoms: turn a layer's (stat- and position-adjusted) data into scene nodes.
//! One file per geom; add it to [`GEOMS`].
//!
//! A geom declares what it needs (required aesthetics, style aesthetics with
//! defaults, extra parameters); the build stage validates the layer against that,
//! resolves styles (mapped -> constant -> default) and calls [`Geom::draw`].

mod bar;
mod blank;
mod boxplot;
mod lines;
mod path;
mod point;
mod polygon;
mod reference_line;
mod segment;
mod text;

pub use bar::{BarGeom, HistogramGeom};
pub use blank::BlankGeom;
pub use boxplot::BoxplotGeom;
pub use path::{LineGeom, PathGeom};
pub use point::PointGeom;
pub use polygon::PolygonGeom;
pub use reference_line::{HorizontalLineGeom, VerticalLineGeom};
pub use segment::SegmentGeom;
pub use text::{LabelGeom, TextGeom};

use crate::Result;
use crate::coord::Panel;
use crate::data::LayerData;
use crate::parameters::Parameters;
use crate::position::DefaultPosition;
use crate::scene::{Node, Rect};
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
    fn default_position(&self) -> DefaultPosition {
        DefaultPosition::IDENTITY
    }

    /// Position aesthetics that must be mapped.
    fn required_aesthetics(&self) -> &'static [Aesthetic];

    /// Position aesthetics the geom accepts but does not need (`geom_blank`).
    fn optional_aesthetics(&self) -> &'static [Aesthetic] {
        &[]
    }

    /// Style aesthetics the geom draws with, and their defaults, in lets-plot's
    /// rendered order (legends follow it).
    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)];

    /// Layer options besides style aesthetics that the geom reads.
    fn parameters(&self) -> &'static [&'static str] {
        &[]
    }

    /// Whether the y axis must include 0 and not pad past it (lets-plot
    /// `rangeIncludesZero`, bars).
    fn includes_zero_y(&self) -> bool {
        false
    }

    /// Prepare the stat's output for the position adjustment, e.g. turn bars into
    /// `xmin`/`xmax`/`ymin`/`ymax` (axis units).
    fn setup(&self, data: LayerData, _parameters: &Parameters) -> Result<LayerData> {
        Ok(data)
    }

    /// Position columns made by [`Self::setup`] (e.g. bar `xmin`/`xmax`/`ymin`/
    /// `ymax`). As lets-plot's dry-run ranges, they do not train the axes after
    /// the position adjustment; those on the x axis train before it (the bar's
    /// width span), so dodged bars keep the undodged range.
    fn setup_aesthetics(&self) -> &'static [Aesthetic] {
        &[]
    }

    /// Draw the layer into scene nodes (in panel pixel space).
    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>>;

    /// Style defaults that differ in legend keys (lets-plot
    /// `AestheticsDefaults.defaultsInLegend`, e.g. bigger points).
    fn legend_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        &[]
    }

    /// Smallest key cell (width, height) that fits a key drawn with `styles`
    /// (one item per aesthetic), before lets-plot's rounding.
    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)>;

    /// Draw one legend key filling `cell`, with one-item `styles`.
    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>>;
}

/// Every geom, in one place: lookup and names derive from it.
pub static GEOMS: &[&dyn Geom] = &[
    &PointGeom,
    &PathGeom,
    &LineGeom,
    &SegmentGeom,
    &PolygonGeom,
    &VerticalLineGeom,
    &HorizontalLineGeom,
    &TextGeom,
    &LabelGeom,
    &BlankGeom,
    &BarGeom,
    &HistogramGeom,
    &BoxplotGeom,
];

/// lets-plot geoms the Python API builds but the engine does not draw yet (plan.md).
/// Remove a name when its geom joins [`GEOMS`].
pub static NOT_DRAWN_YET: &[&str] = &[
    "area_ridges",
    "bracket",
    "jitter",
    "label_repel",
    "raster",
    "rect",
    "sina",
    "text_repel",
    "tile",
    "violin",
];

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
            assert!(!NOT_DRAWN_YET.contains(&name), "{name} is drawn");
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
