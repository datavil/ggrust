//! HTML writer: the SVG of a scene inside a root `<div>`, plus the embedded
//! JavaScript runtime (`js/ggrust.js`) that mounts it.
//!
//! Ids are explicit (one per display), never the content hash of [`to_svg`]: the
//! same plot shown twice on one page must not share clip-path ids once the runtime
//! edits them. Interactivity lives in the runtime; the tooltips' targets and text
//! ([`Interaction`]) ride along as JSON in a `<script type="application/json">`
//! inside the root element.

use super::svg::{SvgOptions, sanitize_id, to_svg_with};
use crate::scene::Scene;
use crate::tooltip::Interaction;

/// The runtime: one JavaScript function expression called with the engine version.
const RUNTIME: &str = include_str!("../../../../js/ggrust.js");

/// Which HTML document [`to_html`] writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HtmlDocument {
    /// The root `<div>` and its `<script>`, for notebooks (`_repr_html_`) and
    /// embedding in other pages.
    Fragment,
    /// A complete standalone page.
    Page,
    /// The page inside an `<iframe srcdoc>` sized to the plot (as lets-plot's
    /// `to_html(iframe=True)`), isolating it from the host page's CSS.
    Iframe,
}

/// Options for [`to_html`].
#[derive(Debug, Clone)]
pub struct HtmlOptions {
    /// Id of the root element and prefix of every element id inside the SVG.
    /// Must be unique on the page. Sanitized like [`SvgOptions::id_prefix`].
    pub id: String,
    /// The document to write.
    pub document: HtmlDocument,
}

/// Render a scene to HTML, with the hover tooltips of `interaction` if given.
pub fn to_html(scene: &Scene, interaction: Option<&Interaction>, options: &HtmlOptions) -> String {
    let id = sanitize_id(&options.id);
    let svg = to_svg_with(
        scene,
        &SvgOptions {
            id_prefix: Some(id.clone()),
        },
    );
    // Escaped so that it cannot end its script element (see `to_script_json`).
    let data = interaction
        .filter(|interaction| !interaction.layers.is_empty())
        .map(|interaction| {
            format!(
                "<script type=\"application/json\" class=\"ggrust-tooltips\">{}</script>\n",
                interaction.to_script_json()
            )
        })
        .unwrap_or_default();
    // `id` is sanitized to [A-Za-z0-9_-], so it needs no escaping in HTML or JS.
    let fragment = format!(
        "<div id=\"{id}\" class=\"ggrust-plot\" \
         style=\"display:inline-block;position:relative;line-height:0;vertical-align:top\">\n\
         {svg}{data}</div>\n\
         <script type=\"text/javascript\">\n\
         {RUNTIME}(\"{version}\");\n\
         window.ggrust.mount(\"{id}\");\n\
         </script>\n",
        version = crate::VERSION,
    );
    match options.document {
        HtmlDocument::Fragment => fragment,
        HtmlDocument::Page => page(&fragment, ""),
        HtmlDocument::Iframe => {
            let document = page(&fragment, " overflow: hidden;");
            format!(
                "<iframe src=\"about:blank\" style=\"border:none !important;\" \
                 width=\"{width}\" height=\"{height}\" srcdoc=\"{srcdoc}\"></iframe>\n",
                width = scene.width.ceil(),
                height = scene.height.ceil(),
                srcdoc = escape_attribute(&document),
            )
        }
    }
}

fn page(fragment: &str, extra_body_style: &str) -> String {
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"UTF-8\">\n\
         <title>ggrust plot</title>\n\
         <style> html, body {{ margin: 0; padding: 0;{extra_body_style} }} </style>\n\
         </head>\n<body>\n{fragment}</body>\n</html>\n"
    )
}

/// Escape text for a double-quoted HTML attribute value.
fn escape_attribute(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len() + text.len() / 8);
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '"' => escaped.push_str("&quot;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            other => escaped.push(other),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Group, Node, Rect};

    fn clipped_scene() -> Scene {
        Scene {
            width: 200.5,
            height: 100.0,
            background: None,
            nodes: vec![Node::Group(Group {
                clip: Some(Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 10.0,
                    height: 10.0,
                }),
                role: None,
                nodes: Vec::new(),
            })],
        }
    }

    fn render(id: &str, document: HtmlDocument) -> String {
        to_html(
            &clipped_scene(),
            None,
            &HtmlOptions {
                id: id.into(),
                document,
            },
        )
    }

    #[test]
    fn runtime_is_safe_to_inline() {
        assert!(
            !RUNTIME.contains("</"),
            "the runtime would close its <script>"
        );
        assert!(RUNTIME.trim_start().starts_with("//"));
        // No trailing semicolon: the renderer appends the call `("<version>")`.
        assert!(RUNTIME.trim_end().ends_with("})"));
        assert!(!RUNTIME.trim_end().ends_with(";"));
    }

    #[test]
    fn fragment_uses_the_display_id_everywhere() {
        let html = render("plot 1", HtmlDocument::Fragment);
        assert!(html.starts_with("<div id=\"plot_1\" class=\"ggrust-plot\""));
        assert!(html.contains("<clipPath id=\"plot_1-clip0\""), "{html}");
        assert!(html.contains("url(#plot_1-clip0)"), "{html}");
        assert!(html.contains("window.ggrust.mount(\"plot_1\");"));
        assert!(html.contains(&format!("(\"{}\");", crate::VERSION)));
        assert!(!html.contains("<!DOCTYPE"));
    }

    #[test]
    fn two_displays_of_one_scene_do_not_share_ids() {
        let first = render("a", HtmlDocument::Fragment);
        let second = render("b", HtmlDocument::Fragment);
        assert!(first.contains("id=\"a-clip0\"") && !first.contains("b-clip0"));
        assert!(second.contains("id=\"b-clip0\"") && !second.contains("a-clip0"));
    }

    #[test]
    fn page_wraps_the_fragment() {
        let html = render("p", HtmlDocument::Page);
        assert!(html.starts_with("<!DOCTYPE html>\n<html lang=\"en\">"));
        assert!(html.contains(&render("p", HtmlDocument::Fragment)));
        assert!(html.trim_end().ends_with("</html>"));
    }

    #[test]
    fn iframe_escapes_the_page_and_takes_the_plot_size() {
        let html = render("f", HtmlDocument::Iframe);
        assert!(html.starts_with("<iframe "), "{html}");
        assert!(html.contains("width=\"201\" height=\"100\""), "{html}");
        let srcdoc = html
            .split("srcdoc=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap_or_default();
        assert!(srcdoc.starts_with("&lt;!DOCTYPE html&gt;"), "{srcdoc}");
        assert!(srcdoc.contains("overflow: hidden;"));
        assert!(!srcdoc.contains('<') && !srcdoc.contains('"'));
        assert_eq!(html.matches("<script").count(), 0);
    }

    #[test]
    fn escape_attribute_handles_markup() {
        assert_eq!(
            escape_attribute(r#"<a href="x">&</a>"#),
            "&lt;a href=&quot;x&quot;&gt;&amp;&lt;/a&gt;"
        );
    }
}
