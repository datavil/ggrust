//! `geom_image`: one bitmap (`href`, a PNG data URL) stretched over the box
//! `xmin`..`xmax` x `ymin`..`ymax`. `geom_imshow` builds this layer in Python.
//!
//! Follows lets-plot `ImageGeom`: the image's top row is drawn at `ymax`, pixels
//! are scaled without smoothing, the plot defaults to `coord_fixed()`. Under
//! `coord_flip` the bitmap is transposed and reversed axes mirror it, so each
//! pixel stays at its data position (as `geom_raster`).
//!
//! The PNG stays encoded in the scene with its orientation: SVG embeds it as
//! given (a transform turns it), raster outputs decode it.
//!
//! What it does not do: image formats other than PNG, `href` URLs that are not
//! `data:image/png;base64,...`, a legend key (the `paint_c` colorbar comes from
//! the color scale, the layer itself draws no key).

use super::{DrawLayer, Geom};
use crate::coord::Panel;
use crate::parameters::Parameters;
use crate::scene::{Bitmap, Color, EncodedPng, Image, Node, Orientation, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};
use crate::{Error, Result};

/// The image box.
const BOX_AESTHETICS: &[Aesthetic] = &[
    Aesthetic::Xmin,
    Aesthetic::Xmax,
    Aesthetic::Ymin,
    Aesthetic::Ymax,
];

/// The colorbar aesthetics (`color_by`); the image does not draw them.
const STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (
        Aesthetic::PaintA,
        StyleDefault::Color(Color::rgb(0x47, 0x47, 0x47)),
    ),
    (
        Aesthetic::PaintB,
        StyleDefault::Color(Color::rgb(0x47, 0x47, 0x47)),
    ),
    (
        Aesthetic::PaintC,
        StyleDefault::Color(Color::rgb(0x47, 0x47, 0x47)),
    ),
];

/// The only `href` form ggrust reads (what `geom_imshow` writes).
const PNG_DATA_PREFIX: &str = "data:image/png;base64,";

/// A bitmap image placed in data space.
pub struct ImageGeom;

impl Geom for ImageGeom {
    fn name(&self) -> &'static str {
        "image"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        BOX_AESTHETICS
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        STYLE_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["href", "color_by"]
    }

    fn default_coord(&self) -> Option<&'static str> {
        Some("fixed")
    }

    fn inherits_aes(&self) -> bool {
        false
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let corners = BOX_AESTHETICS
            .iter()
            .map(|&aesthetic| {
                layer
                    .data
                    .continuous(aesthetic)
                    .and_then(|column| column.get(0))
            })
            .collect::<Option<Vec<f64>>>();
        let Some(&[x0, x1, y0, y1]) = corners.as_deref() else {
            return Ok(Vec::new());
        };
        if !(x0.is_finite() && x1.is_finite() && y0.is_finite() && y1.is_finite()) {
            return Ok(Vec::new());
        }
        let png = encoded_href(&layer.parameters)?;

        // Pixel corners of the image's top-left (xmin, ymax) and bottom-right
        // (xmax, ymin); data x may run vertically (coord_flip), axes may be reversed.
        let top_left = panel.map(x0, y1);
        let bottom_right = panel.map(x1, y0);
        let rect = Rect {
            x: top_left.0.min(bottom_right.0),
            y: top_left.1.min(bottom_right.1),
            width: (top_left.0 - bottom_right.0).abs(),
            height: (top_left.1 - bottom_right.1).abs(),
        };
        // Image column runs along data x, image row along data y (top down).
        let orientation = Orientation {
            transposed: panel.flipped,
            mirror_horizontal: bottom_right.0 < top_left.0,
            mirror_vertical: bottom_right.1 < top_left.1,
        };
        let (columns, rows) = orientation.screen_size(png.columns(), png.rows());
        Ok(vec![Node::Image(Image {
            rect,
            columns,
            rows,
            bitmap: Bitmap::Png { png, orientation },
        })])
    }

    fn minimum_key_size(&self, _styles: &Styles) -> Result<(f64, f64)> {
        Ok((0.0, 0.0))
    }

    fn draw_key(&self, _styles: &Styles, _cell: Rect) -> Result<Vec<Node>> {
        Ok(Vec::new())
    }
}

/// The layer's `href` PNG, still encoded (its header checked).
fn encoded_href(parameters: &Parameters) -> Result<EncodedPng> {
    let path = parameters.path_of("href");
    let Some(href) = parameters.string("href")? else {
        return Err(Error::InvalidSpec(format!(
            "at '{path}': geom_image needs an 'href' (a '{PNG_DATA_PREFIX}...' data URL); \
             build the layer with geom_imshow()"
        )));
    };
    let Some(encoded) = href.strip_prefix(PNG_DATA_PREFIX) else {
        let start: String = href.chars().take(30).collect();
        return Err(Error::InvalidSpec(format!(
            "at '{path}': expected a '{PNG_DATA_PREFIX}...' data URL, got '{start}...'; \
             ggrust reads embedded PNG images only (as geom_imshow() writes them)"
        )));
    };
    EncodedPng::new(encoded.into())
        .map_err(|reason| Error::InvalidSpec(format!("at '{path}': {reason}")))
}
