//! `geom_quantile()`: quantile regression lines (`stat="quantile"`), drawn as
//! paths with ggplot2's defaults (blue, thin).
//!
//! What it does not do: anything a path does not (the lines come from the stat).

use super::path::{PathGeom, draw_paths};
use super::{DrawLayer, Geom};
use crate::Result;
use crate::coord::Panel;
use crate::scene::{Color, Node, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};

/// ggplot2 `GeomQuantile$default_aes`: colour "#3366FF", linewidth 0.5.
const DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (
        Aesthetic::Color,
        StyleDefault::Color(Color::rgb(0x33, 0x66, 0xFF)),
    ),
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Size, StyleDefault::Number(0.5)),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
];

/// Quantile regression lines.
pub struct QuantileGeom;

impl Geom for QuantileGeom {
    fn name(&self) -> &'static str {
        "quantile"
    }

    fn default_stat(&self) -> &'static str {
        "quantile"
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
        DEFAULTS
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
