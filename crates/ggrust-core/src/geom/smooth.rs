//! `geom_smooth`: a fitted curve over its confidence band, as lets-plot's
//! `SmoothGeom` (default `stat="smooth"`).
//!
//! Rows sort by x per group. All bands (`ymin` to `ymax`, filled with `fill`
//! and `alpha`, no outline) are drawn first, then all curves (`y`, stroked
//! without `alpha`). Without a band (`se=False`) only the curves are drawn.
//! Defaults as lets-plot: magenta line of size 0.75, grey band at alpha 0.15.
//!
//! What it does not do: horizontal smooths (`orientation="y"`).

use super::area::{band_node, edge, finite, pieces, sorted_groups};
use super::lines;
use super::{DrawLayer, Geom};
use crate::Result;
use crate::coord::Panel;
use crate::scene::{Color, Node, PerItem, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, StyleValues, Styles};
use crate::tooltip::{HoverTargets, TooltipBehavior};

/// Fitted curves with confidence bands.
pub struct SmoothGeom;

const STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (
        Aesthetic::Color,
        StyleDefault::Color(Color::rgb(0xFF, 0x00, 0xFF)),
    ),
    (Aesthetic::Fill, StyleDefault::PEN),
    (Aesthetic::Alpha, StyleDefault::Number(0.15)),
    (Aesthetic::Size, StyleDefault::Number(0.75)),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
];

impl Geom for SmoothGeom {
    fn name(&self) -> &'static str {
        "smooth"
    }

    fn default_stat(&self) -> &'static str {
        "smooth"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn optional_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Ymin, Aesthetic::Ymax]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        STYLE_DEFAULTS
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let (Some(x), Some(y)) = (
            layer.data.continuous(Aesthetic::X),
            layer.data.continuous(Aesthetic::Y),
        ) else {
            return Ok(Vec::new());
        };
        let groups = sorted_groups(layer, x);
        let mut nodes = Vec::new();
        if let (Some(low), Some(high)) = (
            layer.data.continuous(Aesthetic::Ymin),
            layer.data.continuous(Aesthetic::Ymax),
        ) {
            for rows in &groups {
                for piece in pieces(rows, |row| finite(low, row) && finite(high, row)) {
                    nodes.push(band_node(layer, panel, &piece, x, high, low)?);
                }
            }
        }
        for rows in &groups {
            for piece in pieces(rows, |row| finite(y, row)) {
                nodes.extend(edge(layer, panel, &piece, x, y)?);
            }
        }
        Ok(nodes)
    }

    fn tooltip_behavior(&self) -> TooltipBehavior {
        TooltipBehavior::ALONG_X
    }

    fn hover_targets(&self, layer: &DrawLayer, panel: &Panel) -> Result<Option<HoverTargets>> {
        Ok(super::hover::vertices(layer, panel))
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        lines::key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        // lets-plot uses the hline key, stroked without alpha as the curve.
        let mut styles = styles.clone();
        styles.insert(
            Aesthetic::Alpha,
            StyleValues::Numbers(PerItem::Uniform(f64::NAN)),
        )?;
        lines::key_line(&styles, cell, false)
    }
}
