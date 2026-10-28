//! `geom_rect`: one rectangle per row from `xmin`/`xmax`/`ymin`/`ymax`.
//!
//! Drawing follows lets-plot `RectGeom`: rows in order, pen outline, pen fill at
//! 10% opacity, `alpha` on the fill only.
//!
//! What it does not do (yet): maps (`map`, `map_join`, `use_crs`).

use super::bar::{draw_key, draw_rectangles, key_size};
use super::{DrawLayer, Geom};
use crate::Result;
use crate::coord::Panel;
use crate::scene::{Color, Node, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};

/// lets-plot rect style: pen outline, pen fill at 10% opacity, size 0.75.
const STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (
        Aesthetic::Color,
        StyleDefault::Color(Color::rgb(0x47, 0x47, 0x47)),
    ),
    (
        Aesthetic::Fill,
        StyleDefault::Color(Color {
            red: 0x47,
            green: 0x47,
            blue: 0x47,
            alpha: 26,
        }),
    ),
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Size, StyleDefault::Number(0.75)),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
];

/// Rectangles given by their corners.
pub struct RectGeom;

impl Geom for RectGeom {
    fn tooltip_behavior(&self) -> crate::tooltip::TooltipBehavior {
        crate::tooltip::TooltipBehavior {
            hidden: &[
                Aesthetic::Xmin,
                Aesthetic::Ymin,
                Aesthetic::Xmax,
                Aesthetic::Ymax,
            ],
            ..crate::tooltip::TooltipBehavior::AREA
        }
    }

    fn hover_targets(
        &self,
        layer: &DrawLayer,
        panel: &Panel,
    ) -> Result<Option<crate::tooltip::HoverTargets>> {
        Ok(super::hover::rectangles(
            layer,
            panel,
            (Aesthetic::Ymin, Aesthetic::Ymax),
        ))
    }

    fn name(&self) -> &'static str {
        "rect"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[
            Aesthetic::Xmin,
            Aesthetic::Xmax,
            Aesthetic::Ymin,
            Aesthetic::Ymax,
        ]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        STYLE_DEFAULTS
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        draw_rectangles(layer, panel, false)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        draw_key(styles, cell)
    }
}
