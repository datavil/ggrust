//! HTML writer: the SVG of a scene inside a root `<div>`, plus the embedded
//! JavaScript runtime (`js/ggrust.js`) that mounts it.
//!
//! Ids are explicit (one per display), never the content hash of [`to_svg`]: the
//! same plot shown twice on one page must not share clip-path ids once the runtime
//! edits them. Interactivity lives in the runtime; the tooltips' targets and text
//! ([`Interaction`]) ride along as JSON in a
//! `<script type="application/vnd.ggrust-tooltips+json">` inside the root element. Dense point batches are drawn on a canvas by the
//! runtime from a third payload (see `canvas`): without JavaScript they are
//! missing from the static view. Point targets that are exactly a canvas
//! batch's markers refer to it (`"canvas": n`) instead of repeating x and y.

use std::fmt::Write as _;

use serde_json::{Value, json};

use super::canvas::Batch;
use super::svg::{SvgOptions, fragment, sanitize_id, to_svg_with_canvas};
use crate::layout::zoom::{Zoom, ZoomAxis};
use crate::scene::{Node, Scene};
use crate::tooltip::{HoverTargets, Interaction, WithCanvasTargets, write_script_json};

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
    let zoom = interaction.and_then(|interaction| interaction.zoom.as_ref());
    // The toolbar sits above the plot (lets-plot `DefaultToolbar`).
    let toolbar_height = if zoom.is_some() { TOOLBAR_HEIGHT } else { 0.0 };
    match options.document {
        HtmlDocument::Fragment => write_fragment(String::new(), &id, scene, interaction),
        HtmlDocument::Page => {
            let mut document =
                write_fragment(page_start("", scene.metadata), &id, scene, interaction);
            document.push_str(PAGE_END);
            document
        }
        HtmlDocument::Iframe => {
            let mut document = write_fragment(
                page_start(" overflow: hidden;", scene.metadata),
                &id,
                scene,
                interaction,
            );
            document.push_str(PAGE_END);
            format!(
                "<iframe src=\"about:blank\" style=\"border:none !important;\" \
                 width=\"{width}\" height=\"{height}\" srcdoc=\"{srcdoc}\"></iframe>\n",
                width = scene.width.ceil(),
                height = (scene.height + toolbar_height).ceil(),
                srcdoc = escape_attribute(&document),
            )
        }
    }
}

/// Append the root `<div>` (SVG and JSON payloads) and its mounting script to
/// `output`. Everything is written into the one buffer: a million-point display
/// is never held twice.
fn write_fragment(
    mut output: String,
    id: &str,
    scene: &Scene,
    interaction: Option<&Interaction>,
) -> String {
    // `id` is sanitized to [A-Za-z0-9_-], so it needs no escaping in HTML or JS.
    // A plot wider than its container shrinks to fit, keeping its aspect ratio
    // (lets-plot's notebook sizing), instead of scrolling the container.
    let _ = writeln!(
        output,
        "<div id=\"{id}\" class=\"ggrust-plot\" \
         style=\"display:inline-block;position:relative;line-height:0;vertical-align:top;max-width:100%\">\n\
         <style>#{id}>svg{{max-width:100%;height:auto}}</style>"
    );
    let (svg, canvas_batches) = to_svg_with_canvas(
        scene,
        &SvgOptions {
            id_prefix: Some(id.to_owned()),
            zoomable: interaction.is_some_and(|interaction| interaction.zoom.is_some()),
        },
        output,
    );
    let mut output = svg.into_bytes();
    // Escaped so that they cannot end their script elements (see `write_script_json`).
    if let Some(interaction) = interaction.filter(|interaction| !interaction.layers.is_empty()) {
        let canvas = canvas_targets(interaction, &canvas_batches);
        let canvas_keyed: Vec<bool> = canvas
            .iter()
            .map(|batch| batch.is_some_and(|b| canvas_batches[b].payload.get("key").is_some()))
            .collect();
        script_element(
            &mut output,
            "ggrust-tooltips",
            &WithCanvasTargets {
                interaction,
                canvas: &canvas,
                canvas_keyed: &canvas_keyed,
            },
        );
    }
    if !canvas_batches.is_empty() {
        script_element(&mut output, "ggrust-canvas", &canvas_batches);
    }
    drop(canvas_batches);
    if let Some(zoom) = interaction.and_then(|interaction| interaction.zoom.as_ref()) {
        script_element(&mut output, "ggrust-zoom", &zoom_json(zoom, scene));
    }
    output.extend_from_slice(
        format!(
            "</div>\n\
             <script type=\"text/javascript\">\n\
             {RUNTIME}(\"{version}\");\n\
             window.ggrust.mount(\"{id}\");\n\
             </script>\n",
            version = crate::VERSION,
        )
        .as_bytes(),
    );
    // Only `&str` text and serde_json output were written.
    String::from_utf8(output)
        .unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned())
}

/// Per tooltip layer, the canvas batch whose markers are exactly its point
/// targets (same count, order and Float32 centres), so the tooltip payload can
/// refer to it instead of repeating the positions. Anything else (culled or
/// non-finite markers, other geoms, SVG points) keeps its x and y inline.
fn canvas_targets(interaction: &Interaction, batches: &[Batch]) -> Vec<Option<usize>> {
    interaction
        .layers
        .iter()
        .map(|layer| match &layer.targets {
            HoverTargets::Points { x, y, .. } => {
                batches.iter().position(|batch| batch.same_points(x, y))
            }
            _ => None,
        })
        .collect()
}

/// `<script type="application/vnd.{class}+json" class="{class}">{json}</script>`.
///
/// The runtime finds the payload by its `type`, not its `class`: Google Colab
/// re-creates every output `<script>` with only its type and text, dropping the class.
fn script_element<T: serde::Serialize + ?Sized>(output: &mut Vec<u8>, class: &str, value: &T) {
    output.extend_from_slice(
        format!("<script type=\"application/vnd.{class}+json\" class=\"{class}\">").as_bytes(),
    );
    let start = output.len();
    if write_script_json(output, value).is_err() {
        output.truncate(start);
        output.extend_from_slice(b"null");
    }
    output.extend_from_slice(b"</script>\n");
}

/// lets-plot `DefaultToolbarJs.EXPECTED_TOOLBAR_HEIGHT`: 22 px buttons, 2 px
/// padding, 1 px border, 5 px below.
const TOOLBAR_HEIGHT: f64 = 33.0;

/// The zoom payload as JSON; templates are SVG fragments written by the SVG writer.
fn zoom_json(zoom: &Zoom, scene: &Scene) -> Value {
    let fragment = |node: &Option<Node>| match node {
        Some(node) => Value::String(fragment(
            std::slice::from_ref(node),
            scene.width,
            scene.height,
        )),
        None => Value::Null,
    };
    let axis = |axis: &ZoomAxis| {
        let fixed = axis.fixed.as_ref().map(|breaks| {
            breaks
                .iter()
                .map(|item| json!([item.position, item.pixel, fragment(&item.label)]))
                .collect::<Vec<_>>()
        });
        json!({
            "domain": [axis.domain.0, axis.domain.1],
            "pixels": [axis.pixels.0, axis.pixels.1],
            "transform": axis.transform.spec_name(),
            "exponent": {
                "style": axis.exponent.style.name(),
                "min": axis.exponent.min,
                "max": axis.exponent.max,
            },
            "count": axis.count,
            "fixed": fixed,
            "major": fragment(&axis.major),
            "minor": fragment(&axis.minor),
            "tick": fragment(&axis.tick),
            "minor_tick": fragment(&axis.minor_tick),
            "label": fragment(&axis.label),
            "label_superscript": fragment(&axis.label_superscript),
            "line": fragment(&axis.line),
        })
    };
    let plots: Vec<Value> = zoom
        .plots
        .iter()
        .map(|plot| {
            let tiles: Vec<Value> = plot
                .tiles
                .iter()
                .map(|tile| {
                    let panel = tile.panel;
                    json!({
                        "panel": [panel.x, panel.y, panel.width, panel.height],
                        "horizontal": axis(&tile.horizontal),
                        "vertical": axis(&tile.vertical),
                        "secondary": [
                            tile.secondary[0].as_ref().map(axis),
                            tile.secondary[1].as_ref().map(axis),
                        ],
                    })
                })
                .collect();
            let mut value = json!({
                "tiles": tiles,
                "shared": [plot.shared.0, plot.shared.1],
                "fixed_aspect": plot.fixed_aspect,
            });
            if let Some(labels) = zoom.layer_toggle_labels(plot) {
                value["layers"] = json!(labels);
            }
            value
        })
        .collect();
    json!({
        "size_basis": zoom.size_basis.as_str(),
        "size_zoomin": zoom.size_zoomin,
        "plots": plots,
    })
}

/// A standalone page up to its fragment; [`PAGE_END`] closes it. `metadata` adds
/// `<meta name="generator">` holding [`GENERATOR`](super::GENERATOR).
fn page_start(extra_body_style: &str, metadata: bool) -> String {
    let generator = if metadata {
        format!(
            "<meta name=\"generator\" content=\"{}\">\n",
            super::GENERATOR
        )
    } else {
        String::new()
    };
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"UTF-8\">\n\
         {generator}<title>ggrust plot</title>\n\
         <style> html, body {{ margin: 0; padding: 0;{extra_body_style} }} </style>\n\
         </head>\n<body>\n"
    )
}

/// The end of a [`page_start`] page.
const PAGE_END: &str = "</body>\n</html>\n";

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
            metadata: false,
            background: None,
            nodes: vec![Node::Group(Group {
                key: None,
                clip: Some(Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 10.0,
                    height: 10.0,
                }),
                role: None,
                anchor: None,
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
        assert!(html.contains("<style>#plot_1>svg{max-width:100%;height:auto}</style>"));
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

    /// A scene with one dense point batch, and tooltips on points at `x`, `y`.
    fn dense_points(x: Vec<f64>, y: Vec<f64>) -> (Scene, Interaction) {
        use crate::scene::{MarkerShape, PerItem, Points};
        use crate::tooltip::{LayerContent, LayerInteraction, Lookup, TextColumn};
        let count = super::super::canvas::CANVAS_THRESHOLD + 1;
        let points = Points {
            keys: None,
            x: (0..count)
                .map(|index| (index % 100) as f64 + 0.25)
                .collect(),
            y: (0..count).map(|index| (index / 100) as f64 + 0.5).collect(),
            shape: PerItem::Uniform(MarkerShape::Circle),
            size: PerItem::Uniform(2.0),
            fill: PerItem::Uniform(None),
            stroke: PerItem::Uniform(None),
            stroke_width: PerItem::Uniform(0.0),
        };
        let (x, y) = (
            if x.is_empty() { points.x.clone() } else { x },
            if y.is_empty() { points.y.clone() } else { y },
        );
        let scene = Scene {
            width: 200.0,
            height: 200.0,
            metadata: false,
            background: None,
            nodes: vec![Node::Points(points)],
        };
        let mut content = LayerContent::default();
        content
            .lines
            .push(TextColumn::new(None, vec![Some("row".into()); count]));
        let interaction = Interaction {
            layers: vec![LayerInteraction {
                panel: [0.0, 0.0, 200.0, 200.0],
                layer: 0,
                lookup: Lookup::Nearest,
                targets: HoverTargets::Points {
                    radius: vec![1.0; x.len()],
                    rows: (0..x.len() as u32).collect(),
                    x,
                    y,
                },
                content,
                anchor: None,
                min_width: None,
                keys: None,
                side: None,
            }],
            zoom: None,
        };
        (scene, interaction)
    }

    fn tooltip_targets(scene: &Scene, interaction: &Interaction) -> Value {
        let html = to_html(
            scene,
            Some(interaction),
            &HtmlOptions {
                id: "t".into(),
                document: HtmlDocument::Fragment,
            },
        );
        let payload = html
            .split("class=\"ggrust-tooltips\">")
            .nth(1)
            .and_then(|rest| rest.split("</script>").next())
            .unwrap_or_default();
        let parsed: Value = serde_json::from_str(payload).unwrap_or_default();
        parsed["layers"][0]["targets"].clone()
    }

    #[test]
    fn canvas_point_targets_refer_to_their_batch() {
        let (scene, interaction) = dense_points(Vec::new(), Vec::new());
        let targets = tooltip_targets(&scene, &interaction);
        assert_eq!(targets["kind"], "points");
        assert_eq!(targets["canvas"], 0);
        assert!(targets.get("x").is_none() && targets.get("y").is_none());
        assert_eq!(targets["rows"].as_array().map(Vec::len), Some(5001));
        assert_eq!(targets["radius"][0], 1.0);
        // Serialized on its own (no canvas), the targets keep x and y.
        let json = serde_json::to_value(&interaction).unwrap_or_default();
        assert_eq!(json["layers"][0]["targets"]["x"][1], 1.25);
    }

    #[test]
    fn targets_that_differ_from_the_batch_stay_inline() {
        let (scene, _) = dense_points(Vec::new(), Vec::new());
        let Some(Node::Points(points)) = scene.nodes.first() else {
            unreachable!()
        };
        // One moved point, then one point missing: not the same points.
        let mut moved = points.x.clone();
        moved[7] += 0.5;
        let missing = points.x[1..].to_vec();
        for x in [moved, missing] {
            let y = points.y[..x.len()].to_vec();
            let (scene, interaction) = dense_points(x, y);
            let targets = tooltip_targets(&scene, &interaction);
            assert!(targets.get("canvas").is_none(), "{targets}");
            assert_eq!(
                targets["x"].as_array().map(Vec::len),
                Some(target_count(&interaction))
            );
        }
    }

    fn target_count(interaction: &Interaction) -> usize {
        match &interaction.layers[0].targets {
            HoverTargets::Points { x, .. } => x.len(),
            _ => 0,
        }
    }

    #[test]
    fn escape_attribute_handles_markup() {
        assert_eq!(
            escape_attribute(r#"<a href="x">&</a>"#),
            "&lt;a href=&quot;x&quot;&gt;&amp;&lt;/a&gt;"
        );
    }
}
