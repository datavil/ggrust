//! Renderers: [`Scene`](crate::scene::Scene) -> SVG text, PNG bytes, vector PDF bytes or HTML.
//!
//! Rasterized layers (`scene::Raster`) become images in SVG and PDF output through
//! [`rasterize_layers`] (`to_pdf` calls it; call it before `to_svg`).

mod canvas;
mod html;
mod pdf;
mod png;
mod raster;
mod sprite;
mod svg;
mod webfont;

pub use html::{HtmlDocument, HtmlOptions, to_html};
pub use pdf::to_pdf;
pub use png::{SCREEN_DPI, rgba_png_data_url, svg_to_png, to_png, to_png_with_dpi};
pub use raster::rasterize_layers;
pub use svg::{SvgOptions, to_svg, to_svg_with};
