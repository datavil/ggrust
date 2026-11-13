//! Renderers: [`Scene`](crate::scene::Scene) -> SVG text, PNG / JPEG / TIFF bytes, vector PDF
//! bytes or HTML.
//!
//! Rasterized layers (`scene::Raster`) become images in SVG and PDF output through
//! [`rasterize_layers`] (`to_pdf` calls it; call it before `to_svg`).
//!
//! Every format records which ggrust made it ([`GENERATOR`]) in its usual metadata
//! slot when `Scene::metadata` is set. The tag is plain text, never drawn.

mod canvas;
mod html;
mod image;
mod pdf;
mod png;
mod raster;
mod sprite;
mod svg;
pub(crate) use svg::push_base64;
mod webfont;

/// The tag written into output metadata: `ggrust v` and the crate version.
pub const GENERATOR: &str = concat!("ggrust v", env!("CARGO_PKG_VERSION"));

pub use html::{HtmlDocument, HtmlOptions, to_html};
pub use image::{DEFAULT_JPEG_QUALITY, to_jpeg, to_tiff};
pub use pdf::to_pdf;
pub use png::{SCREEN_DPI, rgba_png_data_url, svg_to_png, to_png, to_png_with_dpi};
pub use raster::rasterize_layers;
pub use svg::{SvgOptions, to_svg, to_svg_with};
