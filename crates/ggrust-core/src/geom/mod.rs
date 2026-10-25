//! Geoms: turn a layer's (stat- and position-adjusted) data into scene nodes.
//! One file per geom; register it in [`geom`].
//!
//! A geom declares what it needs (required aesthetics, style aesthetics with
//! defaults, extra parameters); the build stage validates the layer against that,
//! resolves styles (mapped -> constant -> default) and calls [`Geom::draw`].

mod point;

pub use point::PointGeom;

use std::collections::BTreeMap;

use crate::Result;
use crate::coord::Panel;
use crate::data::LayerData;
use crate::parameters::Parameters;
use crate::scene::{Color, Node, PerItem};
use crate::spec::Aesthetic;

/// Default value of a style aesthetic, which also fixes its value type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StyleDefault {
    /// A color (`color`, `fill`).
    Color(Color),
    /// A number (`size`, `alpha`, `shape`, `stroke`).
    Number(f64),
}

/// A geometric object.
pub trait Geom: Sync {
    /// lets-plot name, e.g. `point`.
    fn name(&self) -> &'static str;

    /// Stat used when the layer names none.
    fn default_stat(&self) -> &'static str {
        "identity"
    }

    /// Position used when the layer names none.
    fn default_position(&self) -> &'static str {
        "identity"
    }

    /// Position aesthetics that must be mapped.
    fn required_aesthetics(&self) -> &'static [Aesthetic];

    /// Style aesthetics the geom draws with, and their defaults.
    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)];

    /// Layer options besides style aesthetics that the geom reads.
    fn parameters(&self) -> &'static [&'static str] {
        &[]
    }

    /// Draw the layer into scene nodes (in panel pixel space).
    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>>;
}

/// Look up a geom by its lets-plot name.
pub fn geom(name: &str) -> Option<&'static dyn Geom> {
    match name {
        "point" => Some(&PointGeom),
        _ => None,
    }
}

/// Names [`geom`] accepts, for error messages.
pub const NAMES: &[&str] = &["point"];

/// Resolved style values of one style aesthetic.
#[derive(Debug, Clone, PartialEq)]
pub enum StyleValues {
    /// Colors.
    Colors(PerItem<Color>),
    /// Numbers.
    Numbers(PerItem<f64>),
}

/// Everything a geom needs to draw one layer.
pub struct DrawLayer<'a> {
    /// Position columns (continuous, in data units).
    pub data: &'a LayerData,
    /// Resolved style aesthetics; every entry of the geom's `style_defaults` is present.
    pub styles: &'a BTreeMap<Aesthetic, StyleValues>,
    /// Layer options.
    pub parameters: Parameters<'a>,
}

impl DrawLayer<'_> {
    /// Colors of a style aesthetic the geom declared as a color.
    pub fn colors(&self, aesthetic: Aesthetic) -> PerItem<Color> {
        match self.styles.get(&aesthetic) {
            Some(StyleValues::Colors(colors)) => colors.clone(),
            _ => PerItem::Uniform(Color::BLACK),
        }
    }

    /// Numbers of a style aesthetic the geom declared as a number.
    pub fn numbers(&self, aesthetic: Aesthetic) -> PerItem<f64> {
        match self.styles.get(&aesthetic) {
            Some(StyleValues::Numbers(numbers)) => numbers.clone(),
            _ => PerItem::Uniform(f64::NAN),
        }
    }
}
