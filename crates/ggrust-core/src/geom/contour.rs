//! `geom_contour`, `geom_contourf`, `geom_density2d`, `geom_density2df`: the
//! contour stats drawn as paths (lines) or multi-ring polygons (bands), as
//! lets-plot's `ContourGeom` (a `PathGeom`) and `ContourfGeom` (a `PolygonGeom`).
//!
//! Defaults as lets-plot: lines 0.5 wide in grey, bands blue with no outline, and
//! no default `..level..` mapping (map `color`/`fill` to `..level..` to tell the
//! levels apart).
//!
//! What it does not do: anything `geom_path` / `geom_polygon` do not.

use super::path::{PathGeom, draw_paths};
use super::polygon::{PolygonGeom, draw_polygons};
use super::{DrawLayer, Geom};
use crate::Result;
use crate::coord::Panel;
use crate::scene::{Color, Node, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};

/// lets-plot contour lines: grey, size 0.5.
const LINE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (
        Aesthetic::Color,
        StyleDefault::Color(Color::rgb(0x47, 0x47, 0x47)),
    ),
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Size, StyleDefault::Number(0.5)),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
];

/// lets-plot contour bands: polygon blue, transparent outline.
const BAND_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (Aesthetic::Color, StyleDefault::Color(Color::TRANSPARENT)),
    (
        Aesthetic::Fill,
        StyleDefault::Color(Color::rgb(0x11, 0x8e, 0xd8)),
    ),
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Size, StyleDefault::Number(0.75)),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
];

/// Isolines of z (`stat="contour"`).
pub struct ContourGeom;

/// Isolines of a 2D density (`stat="density2d"`).
pub struct Density2dGeom;

/// Bands between isolines of z (`stat="contourf"`).
pub struct ContourfGeom;

/// Bands of a 2D density (`stat="density2df"`).
pub struct Density2dfGeom;

/// The contour line geoms share everything but their name and stat.
macro_rules! line_geom {
    ($geom:ty, $name:literal) => {
        impl Geom for $geom {
            fn name(&self) -> &'static str {
                $name
            }

            fn default_stat(&self) -> &'static str {
                $name
            }

            fn tooltip_behavior(&self) -> crate::tooltip::TooltipBehavior {
                PathGeom.tooltip_behavior()
            }

            fn hover_targets(
                &self,
                layer: &DrawLayer,
                panel: &Panel,
            ) -> Result<Option<crate::tooltip::HoverTargets>> {
                PathGeom.hover_targets(layer, panel)
            }

            fn required_aesthetics(&self) -> &'static [Aesthetic] {
                &[Aesthetic::X, Aesthetic::Y]
            }

            fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
                LINE_DEFAULTS
            }

            fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
                draw_paths(layer, panel, false, None)
            }

            fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
                PathGeom.minimum_key_size(styles)
            }

            fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
                PathGeom.draw_key(styles, cell)
            }
        }
    };
}

/// The band geoms share everything but their name and stat.
macro_rules! band_geom {
    ($geom:ty, $name:literal) => {
        impl Geom for $geom {
            fn name(&self) -> &'static str {
                $name
            }

            fn default_stat(&self) -> &'static str {
                $name
            }

            fn required_aesthetics(&self) -> &'static [Aesthetic] {
                &[Aesthetic::X, Aesthetic::Y]
            }

            fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
                BAND_DEFAULTS
            }

            fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
                draw_polygons(layer, panel, true)
            }

            fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
                PolygonGeom.minimum_key_size(styles)
            }

            fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
                PolygonGeom.draw_key(styles, cell)
            }
        }
    };
}

line_geom!(ContourGeom, "contour");
line_geom!(Density2dGeom, "density2d");
band_geom!(ContourfGeom, "contourf");
band_geom!(Density2dfGeom, "density2df");
