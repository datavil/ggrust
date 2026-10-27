//! `geom_jitter`: points with `position="jitter"` by default, the jitter taking
//! the layer's `width`, `height` and `seed` (lets-plot `GeomProto`). Drawing is
//! [`PointGeom`]'s.

use super::{DrawLayer, Geom, PointGeom};
use crate::Result;
use crate::coord::Panel;
use crate::position::DefaultPosition;
use crate::scene::{Node, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};

/// Jittered points.
pub struct JitterGeom;

impl Geom for JitterGeom {
    fn name(&self) -> &'static str {
        "jitter"
    }

    fn default_position(&self) -> DefaultPosition {
        DefaultPosition {
            from_layer: &["width", "height", "seed"],
            ..DefaultPosition::named("jitter")
        }
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        PointGeom.required_aesthetics()
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        PointGeom.style_defaults()
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["width", "height", "seed"]
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        PointGeom.draw(layer, panel)
    }

    fn legend_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        PointGeom.legend_defaults()
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        PointGeom.minimum_key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        PointGeom.draw_key(styles, cell)
    }
}
