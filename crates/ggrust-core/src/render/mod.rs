//! Renderers: [`Scene`](crate::scene::Scene) -> SVG text, PNG bytes, PDF bytes or HTML.

mod canvas;
mod html;
mod pdf;
mod png;
mod svg;

pub use html::{HtmlDocument, HtmlOptions, to_html};
pub use pdf::to_pdf;
pub use png::{SCREEN_DPI, rgba_png_data_url, svg_to_png, to_png, to_png_with_dpi};
pub use svg::{SvgOptions, to_svg, to_svg_with};
