//! `geom_bin2d`: the tiles of `stat="bin2d"` (lets-plot `Bin2dGeom`, a `TileGeom`).
//!
//! As `geom_tile`, but the `"res"` unit of `width`/`height` is the stat's bin
//! size (`..binwidth..`, `..binheight..`) when the data has it, so one bin along
//! an axis still spans it (lets-plot #1290 uses the resolution; deviation).
//!
//! What it does not do: anything `geom_tile` does not (see `tile.rs`).

use super::tile::{TileGeom, setup_tiles};
use super::{DrawLayer, Geom};
use crate::Result;
use crate::coord::Panel;
use crate::data::{Computed, LayerData};
use crate::parameters::Parameters;
use crate::scene::{Node, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};

/// Tiles of 2D bins.
pub struct Bin2dGeom;

impl Geom for Bin2dGeom {
    fn name(&self) -> &'static str {
        "bin2d"
    }

    fn default_stat(&self) -> &'static str {
        "bin2d"
    }

    fn tooltip_behavior(&self) -> crate::tooltip::TooltipBehavior {
        TileGeom.tooltip_behavior()
    }

    fn hover_targets(
        &self,
        layer: &DrawLayer,
        panel: &Panel,
    ) -> Result<Option<crate::tooltip::HoverTargets>> {
        TileGeom.hover_targets(layer, panel)
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        TileGeom.required_aesthetics()
    }

    fn optional_aesthetics(&self) -> &'static [Aesthetic] {
        TileGeom.optional_aesthetics()
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        TileGeom.style_defaults()
    }

    fn parameters(&self) -> &'static [&'static str] {
        TileGeom.parameters()
    }

    fn setup_aesthetics(&self) -> &'static [Aesthetic] {
        TileGeom.setup_aesthetics()
    }

    fn span_aesthetics(&self) -> &'static [Aesthetic] {
        TileGeom.span_aesthetics()
    }

    fn setup(&self, data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        setup_tiles(
            data,
            parameters,
            Some((Computed::BinWidth, Computed::BinHeight)),
        )
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        TileGeom.draw(layer, panel)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        TileGeom.minimum_key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        TileGeom.draw_key(styles, cell)
    }
}
