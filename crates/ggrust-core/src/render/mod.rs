//! Renderers: [`Scene`](crate::scene::Scene) -> SVG text, PNG bytes or HTML.

mod canvas;
mod html;
mod png;
mod svg;

pub use html::{HtmlDocument, HtmlOptions, to_html};
pub use png::{svg_to_png, to_png};
pub use svg::{SvgOptions, to_svg, to_svg_with};
