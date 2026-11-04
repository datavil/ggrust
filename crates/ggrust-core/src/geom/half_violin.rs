//! `geom_half_violin` (R package gghalves): one half of a violin, on the `side`
//! `"l"` (default) or `"r"` of x, moved `nudge` (data units, default 0) away
//! from the centre. Drawing is the violin's (`show_half`), with spans `width`
//! (default 0.75, gghalves' box width) x the x resolution, dodged by group.
//!
//! What it does not do: gghalves' outline across the flat ends (lets-plot's
//! violin outline has none; see deviations.md).

use super::violin::{draw_violins, setup_spans};
use super::{DrawLayer, Geom, ViolinGeom};
use crate::coord::Panel;
use crate::data::LayerData;
use crate::parameters::Parameters;
use crate::position::DefaultPosition;
use crate::scene::{Node, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};
use crate::tooltip::{HoverTargets, TooltipBehavior};
use crate::{Error, Result};

/// gghalves' default box width (ggplot2's boxplot stat).
pub(super) const HALF_WIDTH: f64 = 0.75;

/// The side of x a half geom draws on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HalfSide {
    /// Towards smaller x (below, oriented along y).
    Left,
    /// Towards larger x.
    Right,
}

impl HalfSide {
    /// `side`: `"l"` or `"r"` (also `"left"`, `"right"`), else `default`.
    pub(super) fn from_parameters(parameters: &Parameters, default: HalfSide) -> Result<Self> {
        match parameters.string("side")? {
            None => Ok(default),
            Some("l" | "left") => Ok(HalfSide::Left),
            Some("r" | "right") => Ok(HalfSide::Right),
            Some(other) => Err(Error::spec(
                parameters.path_of("side"),
                format!("unknown side '{other}'; use \"l\" (left) or \"r\" (right)"),
            )),
        }
    }

    /// -1 for left, 1 for right.
    pub(super) fn sign(self) -> f64 {
        match self {
            HalfSide::Left => -1.0,
            HalfSide::Right => 1.0,
        }
    }
}

/// Check that `width` (when given) is positive.
pub(super) fn positive_width(parameters: &Parameters) -> Result<()> {
    match parameters.number("width")? {
        Some(width) if width <= 0.0 => Err(Error::spec(
            parameters.path_of("width"),
            format!(
                "width must be positive (a share of the x resolution, default 0.75), got {width}"
            ),
        )),
        _ => Ok(()),
    }
}

/// `nudge`: a finite distance in data units (default 0).
pub(super) fn nudge(parameters: &Parameters) -> Result<f64> {
    Ok(parameters.number("nudge")?.unwrap_or(0.0))
}

/// Half violins.
pub struct HalfViolinGeom;

impl Geom for HalfViolinGeom {
    fn name(&self) -> &'static str {
        "half_violin"
    }

    fn default_stat(&self) -> &'static str {
        "ydensity"
    }

    fn default_position(&self) -> DefaultPosition {
        DefaultPosition::named("dodge")
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        ViolinGeom.required_aesthetics()
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        ViolinGeom.style_defaults()
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["width", "side", "nudge", "quantile_lines", "quantiles"]
    }

    fn setup_aesthetics(&self) -> &'static [Aesthetic] {
        ViolinGeom.setup_aesthetics()
    }

    fn span_aesthetics(&self) -> &'static [Aesthetic] {
        ViolinGeom.span_aesthetics()
    }

    fn setup(&self, data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        HalfSide::from_parameters(parameters, HalfSide::Left)?;
        positive_width(parameters)?;
        setup_spans(data, parameters.number("width")?.unwrap_or(HALF_WIDTH))
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let side = HalfSide::from_parameters(&layer.parameters, HalfSide::Left)?;
        draw_violins(layer, panel, side.sign(), nudge(&layer.parameters)?)
    }

    fn tooltip_behavior(&self) -> TooltipBehavior {
        ViolinGeom.tooltip_behavior()
    }

    fn hover_targets(&self, layer: &DrawLayer, panel: &Panel) -> Result<Option<HoverTargets>> {
        ViolinGeom.hover_targets(layer, panel)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        ViolinGeom.minimum_key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        ViolinGeom.draw_key(styles, cell)
    }
}
