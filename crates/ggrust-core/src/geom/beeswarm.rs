//! `geom_beeswarm` and `geom_quasirandom` (R package ggbeeswarm): points whose
//! stat (`beeswarm`, `quasirandom`) moves them sideways. Drawing is
//! [`PointGeom`]'s.
//!
//! What it does not do: place points; see [`crate::stat`]'s `beeswarm` and
//! `quasirandom`. Dodging is the stat's `dodge_width`, so the default position is
//! identity.

use super::{DrawLayer, Geom, PointGeom};
use crate::Result;
use crate::coord::Panel;
use crate::scene::{Node, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};

/// Points drawn by [`PointGeom`] from a placing stat.
pub struct SwarmGeom {
    name: &'static str,
}

/// `geom_beeswarm`: non-overlapping points.
pub static BEESWARM_GEOM: SwarmGeom = SwarmGeom { name: "beeswarm" };

/// `geom_quasirandom`: points spread within their density.
pub static QUASIRANDOM_GEOM: SwarmGeom = SwarmGeom {
    name: "quasirandom",
};

impl Geom for SwarmGeom {
    fn tooltip_behavior(&self) -> crate::tooltip::TooltipBehavior {
        crate::tooltip::TooltipBehavior::NEAREST_XY
    }

    fn hover_targets(
        &self,
        layer: &DrawLayer,
        panel: &Panel,
    ) -> Result<Option<crate::tooltip::HoverTargets>> {
        PointGeom.hover_targets(layer, panel)
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn default_stat(&self) -> &'static str {
        self.name
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        PointGeom.required_aesthetics()
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        PointGeom.style_defaults()
    }

    fn parameters(&self) -> &'static [&'static str] {
        PointGeom.parameters()
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
