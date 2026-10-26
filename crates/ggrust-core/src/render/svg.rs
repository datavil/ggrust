//! SVG writer.
//!
//! Uniform batch styles are written once on a wrapping `<g>`; per-item values go on
//! each element. Drawability, dash patterns and clipping of huge geometry follow the
//! shared rules in [`crate::scene::geometry`], so SVG and PNG agree.
//! Text is positioned by baseline computed from the embedded font metrics, never by
//! `dominant-baseline`, which renderers implement inconsistently.

use std::fmt::Write;

use crate::scene::geometry::{ClipBox, is_drawable_marker, is_drawable_stroke, is_drawable_width};
use crate::scene::{
    Color, Group, HorizontalAlign, MarkerShape, Node, Path, PerItem, Points, Rect, Rects,
    SUPERSCRIPT_RISE, SUPERSCRIPT_SCALE, Scene, Stroke, Text, VerticalAlign,
};
use crate::text::{CSS_FONT_FAMILY, FontFamily, measure, single_line};

/// Options for [`to_svg_with`].
#[derive(Debug, Clone, Default)]
pub struct SvgOptions {
    /// Prefix for element ids (clip paths). It must be unique per document when
    /// several SVGs share one HTML page. `None` (the default): `gg` + a hash of the
    /// document, so the same plot always gives the same bytes and different plots
    /// do not collide. Characters other than ASCII letters, digits, `-` and `_` are
    /// replaced by `_`.
    pub id_prefix: Option<String>,
}

/// Length of a content-hash prefix: `gg` + 12 hex digits.
const HASH_PREFIX_LENGTH: usize = 14;

/// Render a scene to SVG text with a content-hash id prefix.
pub fn to_svg(scene: &Scene) -> String {
    to_svg_with(scene, &SvgOptions::default())
}

/// Render a scene to SVG text.
pub fn to_svg_with(scene: &Scene, options: &SvgOptions) -> String {
    let mut writer = SvgWriter {
        output: String::with_capacity(4096),
        id_prefix: match &options.id_prefix {
            Some(prefix) => sanitize_id(prefix),
            None => "g".repeat(HASH_PREFIX_LENGTH),
        },
        id_positions: Vec::new(),
        clip_count: 0,
        clip_box: ClipBox::for_scene(scene.width, scene.height),
    };
    writer.document(scene);
    if options.id_prefix.is_none() && !writer.id_positions.is_empty() {
        // Hash the document with the placeholder in place, then write the prefix
        // over it (same length, so recorded positions stay valid).
        let prefix = format!("gg{:012x}", stable_hash(writer.output.as_bytes()) >> 16);
        for &position in &writer.id_positions {
            writer
                .output
                .replace_range(position..position + HASH_PREFIX_LENGTH, &prefix);
        }
    }
    writer.output
}

/// A fast hash that never changes between runs, platforms or Rust versions
/// (8 bytes at a time, multiply and xor-shift).
fn stable_hash(bytes: &[u8]) -> u64 {
    const MULTIPLIER: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut hash = 0xcbf2_9ce4_8422_2325_u64 ^ bytes.len() as u64;
    let mut chunks = bytes.chunks_exact(8);
    for chunk in &mut chunks {
        let mut word = [0; 8];
        word.copy_from_slice(chunk);
        hash = (hash ^ u64::from_le_bytes(word)).wrapping_mul(MULTIPLIER);
        hash ^= hash >> 29;
    }
    let mut word = [0; 8];
    word[..chunks.remainder().len()].copy_from_slice(chunks.remainder());
    hash = (hash ^ u64::from_le_bytes(word)).wrapping_mul(MULTIPLIER);
    hash ^ (hash >> 32)
}

/// A transparent SVG document containing only `texts`, positioned exactly as
/// [`to_svg`] would. The PNG renderer rasterises text runs through this.
pub(super) fn text_document(width: f64, height: f64, texts: &[&Text]) -> String {
    let mut writer = SvgWriter {
        output: String::with_capacity(256 * texts.len()),
        id_prefix: "text".into(),
        id_positions: Vec::new(),
        clip_count: 0,
        clip_box: ClipBox::for_scene(width, height),
    };
    writer.open(width, height);
    for text in texts {
        writer.text(text);
    }
    writer.raw("</svg>\n");
    writer.output
}

struct SvgWriter {
    output: String,
    id_prefix: String,
    /// Byte offsets where `id_prefix` was written (rewritten by a content hash).
    id_positions: Vec<usize>,
    clip_count: usize,
    clip_box: ClipBox,
}

impl SvgWriter {
    fn document(&mut self, scene: &Scene) {
        self.open(scene.width, scene.height);
        if let Some(background) = scene.background {
            self.raw("<rect");
            self.attribute_number("width", scene.width);
            self.attribute_number("height", scene.height);
            self.fill(Some(background));
            self.raw("/>\n");
        }
        for node in &scene.nodes {
            self.node(node);
        }
        self.raw("</svg>\n");
    }

    fn open(&mut self, width: f64, height: f64) {
        self.raw("<svg xmlns=\"http://www.w3.org/2000/svg\"");
        self.attribute_number("width", width);
        self.attribute_number("height", height);
        self.raw(" viewBox=\"0 0 ");
        self.number(width);
        self.raw(" ");
        self.number(height);
        self.raw("\"");
        self.attribute("font-family", CSS_FONT_FAMILY);
        self.raw(">\n");
    }

    fn node(&mut self, node: &Node) {
        match node {
            Node::Group(group) => self.group(group),
            Node::Points(points) => self.points(points),
            Node::Path(path) => self.path(path),
            Node::Rects(rects) => self.rects(rects),
            Node::Text(text) => self.text(text),
        }
    }

    fn group(&mut self, group: &Group) {
        let clip_number = group.clip.map(|clip| {
            let number = self.clip_count;
            self.clip_count += 1;
            self.raw("<clipPath id=\"");
            self.clip_id(number);
            self.raw("\">");
            self.rect_element(&clip);
            self.raw("/></clipPath>\n");
            number
        });
        self.raw("<g");
        if let Some(role) = group.role {
            self.attribute("class", role);
        }
        if let Some(number) = clip_number {
            self.raw(" clip-path=\"url(#");
            self.clip_id(number);
            self.raw(")\"");
        }
        self.raw(">\n");
        for node in &group.nodes {
            self.node(node);
        }
        self.raw("</g>\n");
    }

    /// `<prefix>-clip<number>`; the prefix is sanitized, so no escaping is needed.
    fn clip_id(&mut self, number: usize) {
        self.id_positions.push(self.output.len());
        self.output.push_str(&self.id_prefix);
        let _ = write!(self.output, "-clip{number}");
    }

    fn points(&mut self, points: &Points) {
        if points.is_empty() || !points.is_consistent() {
            return;
        }
        self.raw("<g");
        if let PerItem::Uniform(fill) = points.fill {
            self.fill(fill);
        }
        // One decision per marker stroke (color and width together), so the
        // `stroke` attribute is written exactly once.
        let uniform_stroke = match (&points.stroke, &points.stroke_width) {
            (PerItem::Uniform(color), PerItem::Uniform(width)) => {
                Some(marker_stroke(*color, *width))
            }
            _ => None,
        };
        if let Some(stroke) = uniform_stroke {
            self.marker_stroke(stroke);
        }
        self.raw(">\n");
        for index in 0..points.len() {
            let (x, y, size) = (points.x[index], points.y[index], points.size.get(index));
            if !is_drawable_marker(x, y, size) || !self.clip_box.contains(x, y) {
                continue;
            }
            let radius = size / 2.0;
            match points.shape.get(index) {
                MarkerShape::Circle => {
                    self.raw("<circle");
                    self.attribute_number("cx", x);
                    self.attribute_number("cy", y);
                    self.attribute_precise("r", radius);
                }
                MarkerShape::Square => {
                    self.raw("<rect");
                    self.attribute_number("x", x - radius);
                    self.attribute_number("y", y - radius);
                    self.attribute_precise("width", size);
                    self.attribute_precise("height", size);
                }
                MarkerShape::Triangle => {
                    // Equilateral, side = size, centroid at (x, y).
                    let height = size * 3f64.sqrt() / 2.0;
                    self.polygon_element(&[
                        (x, y - height * 2.0 / 3.0),
                        (x + radius, y + height / 3.0),
                        (x - radius, y + height / 3.0),
                    ]);
                }
                MarkerShape::Diamond => {
                    self.polygon_element(&[
                        (x, y - radius),
                        (x + radius, y),
                        (x, y + radius),
                        (x - radius, y),
                    ]);
                }
            }
            if let PerItem::Varying(fills) = &points.fill {
                self.fill(fills[index]);
            }
            if uniform_stroke.is_none() {
                self.marker_stroke(marker_stroke(
                    points.stroke.get(index),
                    points.stroke_width.get(index),
                ));
            }
            self.raw("/>\n");
        }
        self.raw("</g>\n");
    }

    fn path(&mut self, path: &Path) {
        let subpaths = self.clip_box.subpaths(&path.points, path.closed);
        if subpaths.is_empty() {
            return;
        }
        let mut data = String::with_capacity(path.points.len() * 14);
        for subpath in &subpaths {
            for (index, &(x, y)) in subpath.iter().enumerate() {
                data.push(if index == 0 { 'M' } else { 'L' });
                push_number(&mut data, x);
                data.push(' ');
                push_number(&mut data, y);
            }
            if path.closed {
                data.push('Z'); // every ring is closed, not only the last
            }
        }
        self.raw("<path");
        self.attribute("d", &data);
        self.fill(if path.closed { path.fill } else { None });
        self.stroke(path.stroke.as_ref());
        self.raw("/>\n");
    }

    fn rects(&mut self, rects: &Rects) {
        if rects.rects.is_empty() || !rects.is_consistent() {
            return;
        }
        self.raw("<g");
        if let PerItem::Uniform(fill) = rects.fill {
            self.fill(fill);
        }
        self.stroke(rects.stroke.as_ref());
        self.raw(">\n");
        for (index, rect) in rects.rects.iter().enumerate() {
            let Some(rect) = self.clip_box.clip_rect(rect) else {
                continue;
            };
            self.rect_element(&rect);
            if let PerItem::Varying(fills) = &rects.fill {
                self.fill(fills[index]);
            }
            self.raw("/>\n");
        }
        self.raw("</g>\n");
    }

    fn text(&mut self, text: &Text) {
        let drawable = text.size.is_finite() && text.size > 0.0 && !text.content.is_empty();
        if !drawable || !self.clip_box.contains(text.x, text.y) {
            return;
        }
        let metrics = measure("", text.size, text.font);
        let baseline_offset = match text.vertical {
            VerticalAlign::Top => metrics.ascent,
            VerticalAlign::Middle => (metrics.ascent - metrics.descent) / 2.0,
            VerticalAlign::Baseline => 0.0,
            VerticalAlign::Bottom => -metrics.descent,
        };
        self.raw("<text");
        self.attribute_number("x", text.x);
        self.attribute_number("y", text.y + baseline_offset);
        self.attribute_precise("font-size", text.size);
        // Keep spaces as measured: SVG collapses whitespace by default.
        self.raw(" xml:space=\"preserve\"");
        if text.font.family == FontFamily::Mono {
            self.attribute("font-family", text.font.family.css());
        }
        if text.font.face.is_bold() {
            self.raw(" font-weight=\"bold\"");
        }
        if text.font.face.is_italic() {
            self.raw(" font-style=\"italic\"");
        }
        self.fill(Some(text.color));
        match text.horizontal {
            HorizontalAlign::Start => {}
            HorizontalAlign::Middle => self.raw(" text-anchor=\"middle\""),
            HorizontalAlign::End => self.raw(" text-anchor=\"end\""),
        }
        if text.angle != 0.0 && text.angle.is_finite() {
            // SVG rotates clockwise; the scene angle is counter-clockwise.
            self.raw(" transform=\"rotate(");
            self.number(-text.angle);
            self.raw(" ");
            self.number(text.x);
            self.raw(" ");
            self.number(text.y);
            self.raw(")\"");
        }
        self.raw(">");
        escape_into(&mut self.output, &single_line(&text.content));
        if let Some(superscript) = &text.superscript {
            self.raw("<tspan");
            self.attribute_precise("font-size", text.size * SUPERSCRIPT_SCALE);
            self.attribute_precise("dy", -text.size * SUPERSCRIPT_RISE);
            self.raw(">");
            escape_into(&mut self.output, &single_line(superscript));
            self.raw("</tspan>");
        }
        self.raw("</text>\n");
    }

    fn rect_element(&mut self, rect: &Rect) {
        self.raw("<rect");
        self.attribute_number("x", rect.x);
        self.attribute_number("y", rect.y);
        self.attribute_number("width", rect.width.max(0.0));
        self.attribute_number("height", rect.height.max(0.0));
    }

    fn polygon_element(&mut self, vertices: &[(f64, f64)]) {
        self.raw("<path d=\"");
        for (index, &(x, y)) in vertices.iter().enumerate() {
            self.raw(if index == 0 { "M" } else { "L" });
            self.number(x);
            self.raw(" ");
            self.number(y);
        }
        self.raw("Z\"");
    }

    fn fill(&mut self, fill: Option<Color>) {
        self.paint("fill", fill);
    }

    /// `stroke` + `stroke-width` of a marker, or `stroke="none"`.
    fn marker_stroke(&mut self, stroke: Option<(Color, f64)>) {
        match stroke {
            Some((color, width)) => {
                self.paint("stroke", Some(color));
                self.attribute_precise("stroke-width", width);
            }
            None => self.attribute("stroke", "none"),
        }
    }

    fn stroke(&mut self, stroke: Option<&Stroke>) {
        let Some(stroke) = stroke.filter(|stroke| is_drawable_stroke(stroke)) else {
            self.raw(" stroke=\"none\"");
            return;
        };
        self.paint("stroke", Some(stroke.color));
        self.attribute_precise("stroke-width", stroke.width);
        if let Some(dash) = stroke.dash_pattern() {
            let mut pattern = String::new();
            for (index, &length) in dash.iter().enumerate() {
                if index > 0 {
                    pattern.push(' ');
                }
                push_number(&mut pattern, length);
            }
            self.attribute("stroke-dasharray", &pattern);
        }
    }

    /// `fill`/`stroke` plus `*-opacity` when translucent; `none` when absent.
    fn paint(&mut self, name: &str, color: Option<Color>) {
        match color {
            Some(color) if !color.is_transparent() => {
                self.attribute(name, &color.to_string());
                if !color.is_opaque() {
                    self.attribute_precise(&format!("{name}-opacity"), color.opacity());
                }
            }
            _ => self.attribute(name, "none"),
        }
    }

    fn attribute(&mut self, name: &str, value: &str) {
        self.output.push(' ');
        self.output.push_str(name);
        self.output.push_str("=\"");
        escape_into(&mut self.output, value);
        self.output.push('"');
    }

    fn attribute_number(&mut self, name: &str, value: f64) {
        self.output.push(' ');
        self.output.push_str(name);
        self.output.push_str("=\"");
        push_number(&mut self.output, value);
        self.output.push('"');
    }

    /// Like [`Self::attribute_number`] with 4 decimals, for widths, radii and
    /// opacities, where 2 decimals would turn small values into 0.
    fn attribute_precise(&mut self, name: &str, value: f64) {
        self.output.push(' ');
        self.output.push_str(name);
        self.output.push_str("=\"");
        push_decimal(&mut self.output, value, 4);
        self.output.push('"');
    }

    fn number(&mut self, value: f64) {
        push_number(&mut self.output, value);
    }

    fn raw(&mut self, text: &str) {
        self.output.push_str(text);
    }
}

/// The visible stroke of a marker: a non-transparent color with a drawable width.
fn marker_stroke(color: Option<Color>, width: f64) -> Option<(Color, f64)> {
    color
        .filter(|color| !color.is_transparent())
        .filter(|_| is_drawable_width(width))
        .map(|color| (color, width))
}

/// Append `value` rounded to 2 decimals, without trailing zeros (`1.5`, `-3`, `0.25`).
fn push_number(output: &mut String, value: f64) {
    push_decimal(output, value, 2);
}

/// Append `value` rounded to `decimals` (at most 6) decimals, without trailing zeros.
/// Non-finite values are written as `0`; magnitudes beyond 1e15 are written whole.
fn push_decimal(output: &mut String, value: f64, decimals: u32) {
    if !value.is_finite() {
        output.push('0');
        return;
    }
    if value.abs() >= 1e15 {
        let _ = write!(output, "{}", value.round());
        return;
    }
    let factor = 10_u64.pow(decimals.min(6));
    let scaled = (value * factor as f64).round() as i64;
    if scaled < 0 {
        output.push('-');
    }
    let magnitude = scaled.unsigned_abs();
    let _ = write!(output, "{}", magnitude / factor);
    let fraction = magnitude % factor;
    if fraction != 0 {
        let digits = format!("{fraction:0width$}", width = decimals.min(6) as usize);
        output.push('.');
        output.push_str(digits.trim_end_matches('0'));
    }
}

/// Ids keep only `[A-Za-z0-9_-]` and start with a letter, so `url(#id)` always works.
fn sanitize_id(prefix: &str) -> String {
    let cleaned: String = prefix
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.starts_with(|character: char| character.is_ascii_alphabetic()) {
        cleaned
    } else {
        format!("g{cleaned}")
    }
}

/// Append `text` with XML special characters escaped.
fn escape_into(output: &mut String, text: &str) {
    for character in text.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            other if is_xml_character(other) => output.push(other),
            _ => {} // XML 1.0 forbids it even escaped (controls, U+FFFE, U+FFFF)
        }
    }
}

/// XML 1.0 `Char` production.
fn is_xml_character(character: char) -> bool {
    matches!(character,
        '\t' | '\n' | '\r'
        | '\u{20}'..='\u{D7FF}'
        | '\u{E000}'..='\u{FFFD}'
        | '\u{10000}'..='\u{10FFFF}')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number(value: f64) -> String {
        let mut output = String::new();
        push_number(&mut output, value);
        output
    }

    #[test]
    fn numbers_are_compact() {
        assert_eq!(number(1.0), "1");
        assert_eq!(number(1.5), "1.5");
        assert_eq!(number(0.25), "0.25");
        assert_eq!(number(0.054), "0.05");
        assert_eq!(number(-3.0), "-3");
        assert_eq!(number(-0.001), "0");
        assert_eq!(number(-0.25), "-0.25");
        assert_eq!(number(f64::NAN), "0");
        assert_eq!(number(1234.567), "1234.57");
    }

    #[test]
    fn text_is_escaped_and_non_xml_characters_dropped() {
        let mut output = String::new();
        escape_into(
            &mut output,
            "a<b & \"c\" 'd'\u{1}\u{FFFE}\u{FFFF}é\u{1F600}",
        );
        assert_eq!(output, "a&lt;b &amp; &quot;c&quot; &#39;d&#39;é\u{1F600}");
    }

    #[test]
    fn huge_and_precise_numbers() {
        let mut output = String::new();
        push_decimal(&mut output, 1e20, 2);
        assert_eq!(output, "100000000000000000000");
        let mut output = String::new();
        push_decimal(&mut output, 1.0 / 255.0, 4);
        assert_eq!(output, "0.0039");
        let mut output = String::new();
        push_decimal(&mut output, 0.5, 4);
        assert_eq!(output, "0.5");
    }

    #[test]
    fn hash_prefixes_are_stable_and_follow_the_content() {
        use crate::scene::{Group, Node, Rect};
        let scene = |width: f64| Scene {
            width,
            height: 50.0,
            background: None,
            nodes: vec![Node::Group(Group {
                clip: Some(Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 10.0,
                    height: 10.0,
                }),
                role: None,
                nodes: vec![],
            })],
        };
        let first = to_svg(&scene(100.0));
        assert_eq!(first, to_svg(&scene(100.0)));
        assert_ne!(first, to_svg(&scene(101.0)));
        let id = first
            .split("id=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        assert!(id.starts_with("gg") && id.ends_with("-clip0"), "{id}");
        assert_eq!(id.len(), HASH_PREFIX_LENGTH + "-clip0".len());
        assert!(first.contains(&format!("url(#{id})")), "{first}");
        assert_eq!(stable_hash(b"ggrust"), stable_hash(b"ggrust"));
    }

    #[test]
    fn id_prefixes_are_sanitized() {
        assert_eq!(sanitize_id("my plot)"), "my_plot_");
        assert_eq!(sanitize_id("7up"), "g7up");
        assert_eq!(sanitize_id("gg3"), "gg3");
    }
}
