//! `geom_blank`: draws nothing but trains the position scales on whatever is
//! mapped (the way to stretch the axes, or to reserve a legend).

use super::{DrawLayer, Geom};
use crate::Result;
use crate::coord::Panel;
use crate::scene::{Color, Node, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};

/// Style aesthetics accepted so a mapping still trains its scale and legend
/// (as lets-plot); the values are never drawn.
const STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (
        Aesthetic::Color,
        StyleDefault::Color(Color::rgb(0x47, 0x47, 0x47)),
    ),
    (Aesthetic::Fill, StyleDefault::Color(Color::WHITE)),
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Size, StyleDefault::Number(0.5)),
    (Aesthetic::Shape, StyleDefault::Shape(16)),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
];

/// A layer with no marks.
pub struct BlankGeom;

impl Geom for BlankGeom {
    fn name(&self) -> &'static str {
        "blank"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[]
    }

    fn optional_aesthetics(&self) -> &'static [Aesthetic] {
        &[
            Aesthetic::X,
            Aesthetic::Y,
            Aesthetic::Xmin,
            Aesthetic::Xmax,
            Aesthetic::Ymin,
            Aesthetic::Ymax,
            Aesthetic::Xend,
            Aesthetic::Yend,
            Aesthetic::Xintercept,
            Aesthetic::Yintercept,
        ]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        STYLE_DEFAULTS
    }

    fn draw(&self, _layer: &DrawLayer, _panel: &Panel) -> Result<Vec<Node>> {
        Ok(Vec::new())
    }

    fn minimum_key_size(&self, _styles: &Styles) -> Result<(f64, f64)> {
        Ok((0.0, 0.0))
    }

    fn draw_key(&self, _styles: &Styles, _cell: Rect) -> Result<Vec<Node>> {
        Ok(Vec::new())
    }
}
