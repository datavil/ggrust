//! PDF output: the scene as true vector graphics on one page.
//!
//! What it does: writes paths, rectangles and markers as PDF path operators, text as
//! real text in the embedded Liberation fonts (subset, with a `ToUnicode` map, so it
//! is selectable and searchable), and raster images as image XObjects (alpha as a
//! soft mask, drawn without smoothing). Glyphs come from the same shaper as text
//! measurement ([`crate::text::shape`]), so widths match the layout and the PNG.
//! Translucency uses `ExtGState` alpha; clips nest with `q ... re W n ... Q`.
//! The page size follows lets-plot's raster PDF (the PNG's pixel size at `dpi`), so
//! a figure has the same physical size as before.
//!
//! Low-level PDF syntax comes from `pdf-writer` and font subsetting from
//! `subsetter` (both from the typst project, pure Rust, a handful of small
//! dependencies); streams are deflated with `flate2`.
//!
//! What it does not do: metadata beyond the producer, multi-page documents,
//! vertical offsets of shaped glyphs (marks; the embedded Latin fonts do not need
//! them), the HTML runtime's canvas points (points are always vector here).

use std::collections::HashMap;
use std::f64::consts::FRAC_PI_2;
use std::io::Write as _;

use flate2::Compression;
use flate2::write::ZlibEncoder;
use pdf_writer::types::{
    CidFontType, FontFlags, LineCapStyle, LineJoinStyle, SystemInfo, UnicodeCmap,
};
use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect as PdfRect, Ref, Str, TextStr};
use read_fonts::types::GlyphId;
use read_fonts::{FontRef, TableProvider};
use subsetter::GlyphRemapper;

use super::svg::marker_stroke;
use crate::scene::geometry::{ClipBox, is_drawable_marker, is_drawable_rect, is_drawable_stroke};
use crate::scene::{
    Bitmap, Color, Group, HorizontalAlign, Image, MarkerPart, MarkerShape, Node, Path, Points,
    Rect, Rects, SUPERSCRIPT_RISE, SUPERSCRIPT_SCALE, Scene, Stroke, Text, VerticalAlign,
};
use crate::text::{Font, FontFace, FontFamily, measure, measure_label, shape, single_line};
use crate::{Error, Result};

/// PDF user space units (points) per inch.
const POINTS_PER_INCH: f64 = 72.0;

/// Largest page side in points (PDF 1.7 implementation limit, Annex C).
const MAX_PAGE_SIDE: f64 = 14_400.0;

/// Bezier control distance of a quarter circle, as a fraction of the radius.
const KAPPA: f64 = 0.552_284_749_830_793_4;

/// Render a scene to a one-page vector PDF. The page is the PNG of the same
/// `scale` laid out at `dpi` (pixels / dpi inches), as lets-plot's PDF.
pub fn to_pdf(scene: &Scene, scale: f64, dpi: f64) -> Result<Vec<u8>> {
    if !(dpi.is_finite() && dpi > 0.0) {
        return Err(Error::render(format!("dpi must be positive, got {dpi}")));
    }
    if !(scale.is_finite() && scale > 0.0) {
        return Err(Error::render(format!(
            "scale must be positive, got {scale}"
        )));
    }
    // The PNG's pixel size (layout size rounded up, scaled, rounded down).
    let pixels = |side: f64| ((side - 1e-6).ceil() * scale + 1e-6).floor();
    let (pixel_width, pixel_height) = (pixels(scene.width), pixels(scene.height));
    if !(pixel_width >= 1.0 && pixel_height >= 1.0) {
        return Err(Error::render(format!(
            "image size {pixel_width} x {pixel_height} pixels is empty; width and height \
             must be at least 1"
        )));
    }
    let points_per_pixel = POINTS_PER_INCH / dpi;
    let page_width = pixel_width * points_per_pixel;
    let page_height = pixel_height * points_per_pixel;
    if page_width > MAX_PAGE_SIDE || page_height > MAX_PAGE_SIDE {
        return Err(Error::render(format!(
            "PDF page {page_width:.0} x {page_height:.0} points is too large; PDF viewers \
             accept at most {MAX_PAGE_SIDE} points (200 inches) per side: lower `scale`, \
             `w`/`h` or raise `dpi`"
        )));
    }

    let mut writer = Writer::new(scene);
    // Scene pixels (y down) -> page points (y up).
    let factor = (scale * points_per_pixel) as f32;
    writer
        .content
        .transform([factor, 0.0, 0.0, -factor, 0.0, page_height as f32]);
    // SVG stroke defaults: butt caps, miter joins, miter limit 4.
    writer
        .content
        .set_line_cap(LineCapStyle::ButtCap)
        .set_line_join(LineJoinStyle::MiterJoin)
        .set_miter_limit(4.0);
    if let Some(background) = scene.background.filter(|color| !color.is_transparent()) {
        writer.set_fill(background);
        writer
            .content
            .rect(0.0, 0.0, scene.width as f32, scene.height as f32)
            .fill_nonzero();
    }
    writer.nodes(&scene.nodes)?;
    writer.finish(page_width as f32, page_height as f32)
}

/// Paint state already set in the content stream (to skip redundant operators).
#[derive(Clone, Default, PartialEq)]
struct State {
    fill: Option<[u8; 3]>,
    stroke: Option<[u8; 3]>,
    /// Fill and stroke alpha.
    alpha: (u8, u8),
    line_width: Option<f64>,
    dash: Vec<f64>,
}

/// Glyphs used from one embedded font.
struct FontUse {
    font: Font,
    remapper: GlyphRemapper,
    /// Text of each new glyph id (by index), for `ToUnicode`.
    texts: Vec<Option<String>>,
}

/// An image XObject waiting to be written.
struct ImageObject {
    columns: u32,
    rows: u32,
    rgb: Vec<u8>,
    alpha: Option<Vec<u8>>,
}

struct Writer {
    content: Content,
    clip_box: ClipBox,
    state: State,
    saved: Vec<State>,
    /// Distinct (fill, stroke) alpha pairs, `/A{index}`.
    alphas: Vec<(u8, u8)>,
    /// Fonts in first-use order, `/F{index}`.
    fonts: Vec<FontUse>,
    /// Images in drawing order, `/I{index}`.
    images: Vec<ImageObject>,
    /// Marker forms in first-use order, `/M{index}`.
    markers: Vec<MarkerForm>,
    marker_index: HashMap<MarkerKey, usize>,
}

/// What a marker form draws: geometry, outline width and which paints.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct MarkerKey {
    shape: MarkerShape,
    /// `f64` bits of the size.
    size: u64,
    /// `f64` bits of the outline width.
    width: u64,
    fill: bool,
    stroke: bool,
    /// Fill or outline is translucent (see [`paint_operators`]).
    translucent: bool,
}

/// A marker drawn around (0, 0), reused by every point that looks the same.
struct MarkerForm {
    /// Half side of the bounding box (generous: the box only clips).
    extent: f32,
    content: Vec<u8>,
}

/// At most this many distinct marker forms per document; more distinct markers
/// (a continuous size scale) are drawn inline.
const MAX_MARKER_FORMS: usize = 1024;

impl Writer {
    fn new(scene: &Scene) -> Self {
        Writer {
            content: Content::new(),
            clip_box: ClipBox::for_scene(scene.width, scene.height),
            state: State {
                alpha: (255, 255),
                ..State::default()
            },
            saved: Vec::new(),
            alphas: vec![(255, 255)],
            fonts: Vec::new(),
            images: Vec::new(),
            markers: Vec::new(),
            marker_index: HashMap::new(),
        }
    }

    fn nodes(&mut self, nodes: &[Node]) -> Result<()> {
        for node in nodes {
            match node {
                Node::Group(group) => self.group(group)?,
                Node::Points(points) => self.points(points),
                Node::Path(path) => self.path(path),
                Node::Rects(rects) => self.rects(rects),
                Node::Text(text) => self.text(text),
                Node::Image(image) => self.image(image)?,
            }
        }
        Ok(())
    }

    fn group(&mut self, group: &Group) -> Result<()> {
        let Some(clip) = group.clip else {
            return self.nodes(&group.nodes);
        };
        let Some(clip) = self.clip_box.clip_rect(&clip).filter(is_drawable_rect) else {
            return Ok(()); // empty or invalid clip: nothing is visible
        };
        self.save();
        self.content
            .rect(
                clip.x as f32,
                clip.y as f32,
                clip.width as f32,
                clip.height as f32,
            )
            .clip_nonzero()
            .end_path();
        let result = self.nodes(&group.nodes);
        self.restore();
        result
    }

    fn save(&mut self) {
        self.content.save_state();
        self.saved.push(self.state.clone());
    }

    fn restore(&mut self) {
        self.content.restore_state();
        if let Some(state) = self.saved.pop() {
            self.state = state;
        }
    }

    fn set_alpha(&mut self, alpha: (u8, u8)) {
        if self.state.alpha == alpha {
            return;
        }
        let index = match self.alphas.iter().position(|&known| known == alpha) {
            Some(index) => index,
            None => {
                self.alphas.push(alpha);
                self.alphas.len() - 1
            }
        };
        let name = format!("A{index}");
        self.content.set_parameters(Name(name.as_bytes()));
        self.state.alpha = alpha;
    }

    fn set_fill(&mut self, color: Color) {
        let rgb = [color.red, color.green, color.blue];
        if self.state.fill != Some(rgb) {
            let [red, green, blue] = rgb.map(channel);
            self.content.set_fill_rgb(red, green, blue);
            self.state.fill = Some(rgb);
        }
        self.set_alpha((color.alpha, self.state.alpha.1));
    }

    fn set_stroke_color(&mut self, color: Color) {
        let rgb = [color.red, color.green, color.blue];
        if self.state.stroke != Some(rgb) {
            let [red, green, blue] = rgb.map(channel);
            self.content.set_stroke_rgb(red, green, blue);
            self.state.stroke = Some(rgb);
        }
        self.set_alpha((self.state.alpha.0, color.alpha));
    }

    fn set_stroke(&mut self, color: Color, width: f64, dash: &[f64]) {
        self.set_stroke_color(color);
        if self.state.line_width != Some(width) {
            self.content.set_line_width(width as f32);
            self.state.line_width = Some(width);
        }
        if self.state.dash != dash {
            self.content
                .set_dash_pattern(dash.iter().map(|&length| length as f32), 0.0);
            self.state.dash = dash.to_vec();
        }
        self.set_alpha((self.state.alpha.0, color.alpha));
    }

    /// Write `geometry` and paint it: fill, outline, or both.
    fn paint(
        &mut self,
        geometry: impl Fn(&mut Content),
        fill: Option<Color>,
        stroke: Option<(Color, f64, &[f64])>,
    ) {
        if let Some(fill) = fill {
            self.set_fill(fill);
        }
        if let Some((color, width, dash)) = stroke {
            self.set_stroke(color, width, dash);
        }
        let translucent = fill.is_some_and(|fill| !fill.is_opaque())
            || stroke.is_some_and(|(color, _, _)| !color.is_opaque());
        paint_operators(
            &mut self.content,
            &geometry,
            fill.is_some(),
            stroke.is_some(),
            translucent,
        );
    }

    fn points(&mut self, points: &Points) {
        if points.is_empty() || !points.is_consistent() {
            return;
        }
        for index in 0..points.len() {
            let (x, y, size) = (points.x[index], points.y[index], points.size.get(index));
            if !is_drawable_marker(x, y, size) || !self.clip_box.contains(x, y) {
                continue;
            }
            let width = points.stroke_width.get(index);
            let fill = points.fill.get(index).filter(|fill| !fill.is_transparent());
            let stroke = marker_stroke(points.stroke.get(index), width);
            let shape = points.shape.get(index);
            let key = MarkerKey {
                shape,
                size: size.to_bits(),
                width: width.to_bits(),
                fill: fill.is_some(),
                stroke: stroke.is_some(),
                translucent: fill.is_some_and(|fill| !fill.is_opaque())
                    || stroke.is_some_and(|(color, _)| !color.is_opaque()),
            };
            if let Some(form) = self.marker_form(key) {
                // Colours and alpha are inherited by the form.
                if let Some(fill) = fill {
                    self.set_fill(fill);
                }
                if let Some((color, _)) = stroke {
                    self.set_stroke_color(color);
                }
                let name = format!("M{form}");
                self.content.save_state();
                self.content
                    .transform([1.0, 0.0, 0.0, 1.0, number(x), number(y)]);
                self.content.x_object(Name(name.as_bytes()));
                self.content.restore_state();
                continue;
            }
            shape.for_each_part(x, y, size, width, |part| {
                let closed = !matches!(part, MarkerPart::Line(..));
                let stroke = stroke.map(|(color, width)| (color, width, &[][..]));
                self.paint(
                    |content| {
                        marker_part(content, &part);
                    },
                    fill.filter(|_| closed),
                    stroke,
                );
            });
        }
    }

    /// The index of the marker form for `key`, drawn around (0, 0) without colours
    /// (one short `Do` per point instead of the marker's path); `None` once
    /// [`MAX_MARKER_FORMS`] distinct markers exist (they are then drawn inline).
    fn marker_form(&mut self, key: MarkerKey) -> Option<usize> {
        if let Some(&index) = self.marker_index.get(&key) {
            return Some(index);
        }
        if self.markers.len() >= MAX_MARKER_FORMS {
            return None;
        }
        let size = f64::from_bits(key.size);
        let width = f64::from_bits(key.width);
        let mut content = Content::new();
        if key.stroke {
            content
                .set_line_width(width as f32)
                .set_dash_pattern([], 0.0);
        }
        key.shape.for_each_part(0.0, 0.0, size, width, |part| {
            let closed = !matches!(part, MarkerPart::Line(..));
            let geometry = |content: &mut Content| {
                marker_part(content, &part);
            };
            paint_operators(
                &mut content,
                &geometry,
                key.fill && closed,
                key.stroke,
                key.translucent,
            );
        });
        let index = self.markers.len();
        self.markers.push(MarkerForm {
            extent: (size + 2.0 * width + 1.0) as f32,
            content: content.finish().into_vec(),
        });
        self.marker_index.insert(key, index);
        Some(index)
    }

    fn path(&mut self, path: &Path) {
        let subpaths = self.clip_box.subpaths(&path.points, path.closed);
        if subpaths.is_empty() {
            return;
        }
        let fill = path
            .fill
            .filter(|fill| path.closed && !fill.is_transparent());
        let stroke = path.stroke.as_ref().and_then(drawable_stroke);
        if fill.is_none() && stroke.is_none() {
            return;
        }
        let closed = path.closed;
        let geometry = |content: &mut Content| {
            for subpath in &subpaths {
                // every ring is closed, not only the last
                polyline(content, subpath, closed);
            }
        };
        let dash = stroke.as_ref().map(|(_, dash)| dash.clone());
        let stroke = stroke.map(|(stroke, _)| (stroke.color, stroke.width));
        self.paint(
            geometry,
            fill,
            stroke.map(|(color, width)| (color, width, dash.as_deref().unwrap_or(&[]))),
        );
    }

    fn rects(&mut self, rects: &Rects) {
        if rects.rects.is_empty() || !rects.is_consistent() {
            return;
        }
        let stroke = rects.stroke.as_ref().and_then(drawable_stroke);
        for (index, rect) in rects.rects.iter().enumerate() {
            let Some(rect) = self.clip_box.clip_rect(rect).filter(is_drawable_rect) else {
                continue;
            };
            let fill = rects.fill.get(index).filter(|fill| !fill.is_transparent());
            if fill.is_none() && stroke.is_none() {
                continue;
            }
            let geometry = |content: &mut Content| rect_path(content, &rect);
            match &stroke {
                Some((stroke, dash)) => {
                    let dash = dash.clone();
                    self.paint(geometry, fill, Some((stroke.color, stroke.width, &dash)));
                }
                None => self.paint(geometry, fill, None),
            }
        }
    }

    fn image(&mut self, image: &Image) -> Result<()> {
        let rect = image.rect;
        let drawable = [rect.x, rect.y, rect.width, rect.height]
            .iter()
            .all(|value| value.is_finite())
            && rect.width > 0.0
            && rect.height > 0.0;
        if !drawable || !image.is_consistent() {
            return Ok(());
        }
        let decoded;
        let pixels = match &image.bitmap {
            Bitmap::Pixels(pixels) => pixels,
            Bitmap::Png { png, orientation } => {
                let pixels = png.decode().map_err(|reason| {
                    Error::render(format!("could not draw an embedded image: {reason}"))
                })?;
                decoded = orientation.apply(&pixels, png.columns() as usize, png.rows() as usize);
                &decoded
            }
        };
        // Viewers that ignore `/Interpolate false` (macOS Quartz) blur large cells:
        // repeat each pixel `factor` times so the blur stays within a cell's edge.
        let factor = upsampling(image);
        let (columns, rows) = (image.columns as usize, image.rows as usize);
        let repeated = |channels: &dyn Fn(&Color, &mut Vec<u8>), width: usize| {
            let mut bytes = Vec::with_capacity(columns * rows * factor * factor * width);
            for row in pixels.chunks(columns.max(1)) {
                let start = bytes.len();
                for color in row {
                    for _ in 0..factor {
                        channels(color, &mut bytes);
                    }
                }
                let line = bytes[start..].to_vec();
                for _ in 1..factor {
                    bytes.extend_from_slice(&line);
                }
            }
            bytes
        };
        let rgb = repeated(
            &|color, bytes| bytes.extend_from_slice(&[color.red, color.green, color.blue]),
            3,
        );
        let alpha = pixels
            .iter()
            .any(|color| !color.is_opaque())
            .then(|| repeated(&|color, bytes| bytes.push(color.alpha), 1));
        let index = self.images.len();
        let scaled = |side: u32| side.saturating_mul(factor as u32);
        self.images.push(ImageObject {
            columns: scaled(image.columns),
            rows: scaled(image.rows),
            rgb,
            alpha,
        });
        // The unit square, top row first, onto `rect` (scene y points down).
        let name = format!("I{index}");
        // Images composite with the fill alpha: opaque, as SVG and PNG draw them.
        self.set_alpha((255, self.state.alpha.1));
        self.save();
        self.content.transform([
            rect.width as f32,
            0.0,
            0.0,
            -rect.height as f32,
            rect.x as f32,
            (rect.y + rect.height) as f32,
        ]);
        self.content.x_object(Name(name.as_bytes()));
        self.restore();
        Ok(())
    }

    fn text(&mut self, text: &Text) {
        let drawable = text.size.is_finite() && text.size > 0.0 && !text.content.is_empty();
        if !drawable || !self.clip_box.contains(text.x, text.y) || text.color.is_transparent() {
            return;
        }
        // Placement as the SVG writer: baseline from the font metrics, the anchor
        // shifted along the baseline, then rotated around (x, y).
        let metrics = measure("", text.size, text.font);
        let baseline_offset = match text.vertical {
            VerticalAlign::Top => metrics.ascent,
            VerticalAlign::Middle => (metrics.ascent - metrics.descent) / 2.0,
            VerticalAlign::Baseline => 0.0,
            VerticalAlign::Bottom => -metrics.descent,
        };
        let content = single_line(&text.content);
        let superscript = text.superscript.as_deref().map(single_line);
        let suffix = text.suffix.as_deref().map(single_line);
        let width = measure_label(
            &content,
            superscript.as_deref(),
            superscript.as_ref().and(suffix.as_deref()),
            text.size,
            text.font,
        );
        let shift = match text.horizontal {
            HorizontalAlign::Start => 0.0,
            HorizontalAlign::Middle => width / 2.0,
            HorizontalAlign::End => width,
        };
        let angle = if text.angle.is_finite() {
            text.angle.to_radians()
        } else {
            0.0
        };
        // Exact right angles: no 6e-17 cosines in the stream.
        let (sin, cos) = if (angle.abs() - FRAC_PI_2).abs() < 1e-12 {
            (angle.signum(), 0.0)
        } else {
            angle.sin_cos()
        };
        // Text space: x along the baseline, y up (the page CTM flips scene y).
        let (a, b, c, d) = (cos, -sin, -sin, -cos);
        let (u, v) = (-shift, -baseline_offset);
        let matrix = [
            a as f32,
            b as f32,
            c as f32,
            d as f32,
            number(text.x + u * a + v * c),
            number(text.y + u * b + v * d),
        ];
        self.set_fill(text.color);
        let font = self.font_index(text.font);
        let name = format!("F{font}");
        self.content.begin_text();
        self.content.set_text_matrix(matrix);
        self.content
            .set_font(Name(name.as_bytes()), number(text.size));
        self.glyph_run(font, &content);
        if let Some(superscript) = &superscript {
            self.content
                .set_font(Name(name.as_bytes()), number(text.size * SUPERSCRIPT_SCALE));
            self.content.set_rise(number(text.size * SUPERSCRIPT_RISE));
            self.glyph_run(font, superscript);
            // Back to the baseline: the rise is text state, which outlives `ET`.
            self.content.set_rise(0.0);
            if let Some(suffix) = &suffix {
                // Back to the base size.
                self.content
                    .set_font(Name(name.as_bytes()), number(text.size));
                self.glyph_run(font, suffix);
            }
        }
        self.content.end_text();
    }

    fn font_index(&mut self, font: Font) -> usize {
        if let Some(index) = self.fonts.iter().position(|used| used.font == font) {
            return index;
        }
        self.fonts.push(FontUse {
            font,
            remapper: GlyphRemapper::new(),
            texts: vec![None],
        });
        self.fonts.len() - 1
    }

    /// Show `text` with the shaped glyphs: two-byte CIDs (subset glyph ids), with
    /// `TJ` adjustments where shaping (kerning) differs from the advance widths.
    fn glyph_run(&mut self, font_index: usize, text: &str) {
        let used = &mut self.fonts[font_index];
        let (glyphs, units_per_em) = shape(text, used.font);
        if glyphs.is_empty() {
            return;
        }
        let font = font_ref(used.font);
        let metrics = font.hmtx().ok();
        let mut items: Vec<(Vec<u8>, f32)> = vec![(Vec::new(), 0.0)];
        for (index, glyph) in glyphs.iter().enumerate() {
            let new_id = used.remapper.remap(glyph.glyph_id);
            let slot = usize::from(new_id);
            if used.texts.len() <= slot {
                used.texts.resize(slot + 1, None);
            }
            // `.notdef` (characters the font lacks) maps to no text: one glyph
            // cannot stand for every missing character.
            if used.texts[slot].is_none() && glyph.glyph_id != 0 {
                // The cluster's text goes on its first glyph.
                let starts_cluster = index == 0 || glyphs[index - 1].cluster != glyph.cluster;
                if starts_cluster {
                    let end = glyphs[index + 1..]
                        .iter()
                        .map(|next| next.cluster)
                        .find(|&cluster| cluster != glyph.cluster)
                        .filter(|&cluster| cluster > glyph.cluster)
                        .unwrap_or(text.len());
                    used.texts[slot] = text.get(glyph.cluster..end).map(str::to_owned);
                }
            }
            let Some(item) = items.last_mut() else {
                continue;
            };
            item.0.extend_from_slice(&new_id.to_be_bytes());
            let advance = metrics
                .as_ref()
                .and_then(|hmtx| hmtx.advance(GlyphId::new(u32::from(glyph.glyph_id))))
                .map_or(glyph.x_advance, i32::from);
            if advance != glyph.x_advance {
                // TJ numbers are thousandths of text space, positive moves left.
                let adjust = (advance - glyph.x_advance) as f32 * 1000.0 / f32::from(units_per_em);
                item.1 = adjust;
                items.push((Vec::new(), 0.0));
            }
        }
        let mut positioned = self.content.show_positioned();
        let mut sink = positioned.items();
        for (bytes, adjust) in &items {
            if !bytes.is_empty() {
                sink.show(Str(bytes));
            }
            if *adjust != 0.0 {
                sink.adjust(*adjust);
            }
        }
    }

    /// Write the document: page, content, fonts, images and graphics states.
    fn finish(self, page_width: f32, page_height: f32) -> Result<Vec<u8>> {
        let mut next = 1;
        let mut new_ref = || {
            let id = Ref::new(next);
            next += 1;
            id
        };
        let catalog_id = new_ref();
        let pages_id = new_ref();
        let page_id = new_ref();
        let content_id = new_ref();
        let info_id = new_ref();
        let alpha_ids: Vec<Ref> = self.alphas.iter().map(|_| new_ref()).collect();
        // Type0 font, CID font, descriptor, font file, ToUnicode map.
        let font_ids: Vec<[Ref; 5]> = self
            .fonts
            .iter()
            .map(|_| [new_ref(), new_ref(), new_ref(), new_ref(), new_ref()])
            .collect();
        let marker_ids: Vec<Ref> = self.markers.iter().map(|_| new_ref()).collect();
        let image_ids: Vec<(Ref, Option<Ref>)> = self
            .images
            .iter()
            .map(|image| (new_ref(), image.alpha.as_ref().map(|_| new_ref())))
            .collect();

        let mut pdf = Pdf::new();
        pdf.catalog(catalog_id).pages(pages_id);
        pdf.pages(pages_id).kids([page_id]).count(1);
        pdf.document_info(info_id).producer(TextStr("ggrust"));
        {
            let mut page = pdf.page(page_id);
            page.media_box(PdfRect::new(0.0, 0.0, page_width, page_height))
                .parent(pages_id)
                .contents(content_id);
            let mut resources = page.resources();
            {
                let mut states = resources.ext_g_states();
                for (index, id) in alpha_ids.iter().enumerate() {
                    let name = format!("A{index}");
                    states.pair(Name(name.as_bytes()), *id);
                }
            }
            if !font_ids.is_empty() {
                let mut fonts = resources.fonts();
                for (index, ids) in font_ids.iter().enumerate() {
                    let name = format!("F{index}");
                    fonts.pair(Name(name.as_bytes()), ids[0]);
                }
            }
            if !image_ids.is_empty() || !marker_ids.is_empty() {
                let mut objects = resources.x_objects();
                for (index, (id, _)) in image_ids.iter().enumerate() {
                    let name = format!("I{index}");
                    objects.pair(Name(name.as_bytes()), *id);
                }
                for (index, id) in marker_ids.iter().enumerate() {
                    let name = format!("M{index}");
                    objects.pair(Name(name.as_bytes()), *id);
                }
            }
        }

        let content = deflate(&self.content.finish().into_vec())?;
        pdf.stream(content_id, &content).filter(Filter::FlateDecode);

        for (&(fill, stroke), id) in self.alphas.iter().zip(&alpha_ids) {
            pdf.ext_graphics(*id)
                .non_stroking_alpha(f32::from(fill) / 255.0)
                .stroking_alpha(f32::from(stroke) / 255.0);
        }

        for (marker, id) in self.markers.iter().zip(&marker_ids) {
            let extent = marker.extent;
            pdf.form_xobject(*id, &marker.content)
                .bbox(PdfRect::new(-extent, -extent, extent, extent));
        }

        for (used, ids) in self.fonts.iter().zip(&font_ids) {
            write_font(&mut pdf, used, ids)?;
        }

        for (image, (id, mask_id)) in self.images.iter().zip(&image_ids) {
            let rgb = deflate(&image.rgb)?;
            let mut object = pdf.image_xobject(*id, &rgb);
            object.filter(Filter::FlateDecode);
            object
                .width(dimension(image.columns))
                .height(dimension(image.rows))
                .bits_per_component(8)
                .interpolate(false);
            object.color_space().device_rgb();
            if let Some(mask_id) = mask_id {
                object.s_mask(*mask_id);
            }
            object.finish();
            if let (Some(alpha), Some(mask_id)) = (&image.alpha, mask_id) {
                let alpha = deflate(alpha)?;
                let mut mask = pdf.image_xobject(*mask_id, &alpha);
                mask.filter(Filter::FlateDecode);
                mask.width(dimension(image.columns))
                    .height(dimension(image.rows))
                    .bits_per_component(8)
                    .interpolate(false);
                mask.color_space().device_gray();
            }
        }
        Ok(pdf.finish())
    }
}

/// A subset Type0/CIDFontType2 font with Identity-H encoding: CIDs are the subset's
/// glyph ids, widths come from `hmtx`, `ToUnicode` maps glyphs back to text.
fn write_font(pdf: &mut Pdf, used: &FontUse, ids: &[Ref; 5]) -> Result<()> {
    let [type0_id, cid_id, descriptor_id, file_id, cmap_id] = *ids;
    let font = font_ref(used.font);
    let failed = |table: &str| {
        Error::render(format!(
            "embedded font {} has no readable {table} table (a build defect)",
            postscript_name(used.font)
        ))
    };
    let head = font.head().map_err(|_| failed("head"))?;
    let hhea = font.hhea().map_err(|_| failed("hhea"))?;
    let hmtx = font.hmtx().map_err(|_| failed("hmtx"))?;
    let post = font.post().map_err(|_| failed("post"))?;
    let units = 1000.0 / f32::from(head.units_per_em());
    let to_pdf_units = |value: i16| f32::from(value) * units;

    let subset = subsetter::subset(used.font.font_data(), 0, &used.remapper).map_err(|error| {
        Error::render(format!(
            "could not subset font {}: {error}",
            postscript_name(used.font)
        ))
    })?;
    // A subset tag from the glyph set: six letters, stable for the same glyphs.
    let mut hash: u32 = 2_166_136_261;
    for glyph in used.remapper.remapped_gids() {
        for byte in glyph.to_be_bytes() {
            hash = (hash ^ u32::from(byte)).wrapping_mul(16_777_619);
        }
    }
    let tag: String = (0..6)
        .map(|position| char::from(b'A' + ((hash >> (position * 5)) % 26) as u8))
        .collect();
    let base_name = format!("{tag}+{}", postscript_name(used.font));
    let base_name = Name(base_name.as_bytes());
    let system_info = SystemInfo {
        registry: Str(b"Adobe"),
        ordering: Str(b"Identity"),
        supplement: 0,
    };

    pdf.type0_font(type0_id)
        .base_font(base_name)
        .encoding_predefined(Name(b"Identity-H"))
        .descendant_font(cid_id)
        .to_unicode(cmap_id);

    let widths: Vec<f32> = used
        .remapper
        .remapped_gids()
        .map(|glyph| {
            let advance = hmtx.advance(GlyphId::new(u32::from(glyph))).unwrap_or(0);
            f32::from(advance) * units
        })
        .collect();
    let mut cid = pdf.cid_font(cid_id);
    cid.subtype(CidFontType::Type2)
        .base_font(base_name)
        .system_info(system_info)
        .font_descriptor(descriptor_id)
        .cid_to_gid_map_predefined(Name(b"Identity"));
    cid.widths().consecutive(0, widths);
    cid.finish();

    let mut flags = FontFlags::NON_SYMBOLIC;
    if used.font.family == FontFamily::Mono {
        flags |= FontFlags::FIXED_PITCH;
    }
    if used.font.face.is_italic() {
        flags |= FontFlags::ITALIC;
    }
    let cap_height = font
        .os2()
        .ok()
        .and_then(|os2| os2.s_cap_height())
        .unwrap_or(hhea.ascender().to_i16());
    pdf.font_descriptor(descriptor_id)
        .name(base_name)
        .flags(flags)
        .bbox(PdfRect::new(
            to_pdf_units(head.x_min()),
            to_pdf_units(head.y_min()),
            to_pdf_units(head.x_max()),
            to_pdf_units(head.y_max()),
        ))
        .italic_angle(post.italic_angle().to_f32())
        .ascent(to_pdf_units(hhea.ascender().to_i16()))
        .descent(to_pdf_units(hhea.descender().to_i16()))
        .cap_height(to_pdf_units(cap_height))
        .stem_v(if used.font.face.is_bold() {
            140.0
        } else {
            80.0
        })
        .font_file2(file_id);

    let compressed = deflate(&subset)?;
    pdf.stream(file_id, &compressed)
        .filter(Filter::FlateDecode)
        .pair(Name(b"Length1"), subset.len() as i32);

    let mut cmap = UnicodeCmap::new(Name(b"Custom"), system_info);
    for (glyph, text) in used.texts.iter().enumerate() {
        if let (Ok(glyph), Some(text)) = (u16::try_from(glyph), text)
            && !text.is_empty()
        {
            cmap.pair_with_multiple(glyph, text.chars());
        }
    }
    let cmap = cmap.finish();
    pdf.cmap(cmap_id, &cmap);
    Ok(())
}

/// The embedded font parsed with read-fonts (known-good data).
fn font_ref(font: Font) -> FontRef<'static> {
    // The embedded fonts parse (checked by the text module at first use).
    FontRef::new(font.font_data()).unwrap_or_else(|_| unreachable!("embedded fonts parse"))
}

/// PostScript name of an embedded font.
fn postscript_name(font: Font) -> &'static str {
    match (font.family, font.face) {
        (FontFamily::Sans, FontFace::Plain) => "LiberationSans",
        (FontFamily::Sans, FontFace::Bold) => "LiberationSans-Bold",
        (FontFamily::Sans, FontFace::Italic) => "LiberationSans-Italic",
        (FontFamily::Sans, FontFace::BoldItalic) => "LiberationSans-BoldItalic",
        (FontFamily::Mono, FontFace::Plain) => "LiberationMono",
        (FontFamily::Mono, FontFace::Bold) => "LiberationMono-Bold",
        (FontFamily::Mono, FontFace::Italic) => "LiberationMono-Italic",
        (FontFamily::Mono, FontFace::BoldItalic) => "LiberationMono-BoldItalic",
    }
}

/// How many times to repeat each image pixel along each side: about 4 device
/// pixels per scene pixel of a cell, 16 at most, the long side at most 4096.
fn upsampling(image: &Image) -> usize {
    let cell = (image.rect.width / f64::from(image.columns))
        .max(image.rect.height / f64::from(image.rows));
    let longest = image.columns.max(image.rows).max(1) as f64;
    let factor = (cell * 4.0)
        .ceil()
        .min(16.0)
        .min((4096.0 / longest).floor());
    if factor.is_finite() && factor > 1.0 {
        factor as usize
    } else {
        1
    }
}

/// Append a marker part to the current path; whether it is closed (fillable).
fn marker_part(content: &mut Content, part: &MarkerPart) -> bool {
    match part {
        MarkerPart::Circle { x, y, radius } => {
            let (x, y, r) = (*x, *y, *radius);
            let k = r * KAPPA;
            content.move_to(number(x + r), number(y));
            content.cubic_to(
                number(x + r),
                number(y + k),
                number(x + k),
                number(y + r),
                number(x),
                number(y + r),
            );
            content.cubic_to(
                number(x - k),
                number(y + r),
                number(x - r),
                number(y + k),
                number(x - r),
                number(y),
            );
            content.cubic_to(
                number(x - r),
                number(y - k),
                number(x - k),
                number(y - r),
                number(x),
                number(y - r),
            );
            content.cubic_to(
                number(x + k),
                number(y - r),
                number(x + r),
                number(y - k),
                number(x + r),
                number(y),
            );
            content.close_path();
            true
        }
        MarkerPart::Rect(rect) => {
            rect_path(content, rect);
            true
        }
        MarkerPart::Polygon(vertices) => {
            polyline(content, vertices, true);
            true
        }
        MarkerPart::Line(start, end) => {
            polyline(content, &[*start, *end], false);
            false
        }
    }
}

fn rect_path(content: &mut Content, rect: &Rect) {
    content.rect(
        number(rect.x),
        number(rect.y),
        number(rect.width),
        number(rect.height),
    );
}

fn polyline(content: &mut Content, vertices: &[(f64, f64)], closed: bool) {
    for (index, &(x, y)) in vertices.iter().enumerate() {
        if index == 0 {
            content.move_to(number(x), number(y));
        } else {
            content.line_to(number(x), number(y));
        }
    }
    if closed {
        content.close_path();
    }
}

/// Write `geometry` with its paint operator. A translucent fill and outline are
/// painted as two operators on the geometry written twice: one `B` composites them
/// as a knockout group (the outline's inner half replaces the fill), while SVG and
/// PNG blend the outline over the fill.
fn paint_operators(
    content: &mut Content,
    geometry: &dyn Fn(&mut Content),
    fill: bool,
    stroke: bool,
    translucent: bool,
) {
    geometry(content);
    match (fill, stroke) {
        (true, true) if translucent => {
            content.fill_nonzero();
            geometry(content);
            content.stroke();
        }
        (true, true) => {
            content.fill_nonzero_and_stroke();
        }
        (true, false) => {
            content.fill_nonzero();
        }
        (false, true) => {
            content.stroke();
        }
        (false, false) => {
            content.end_path();
        }
    }
}

/// A drawable stroke and its dash pattern (empty when solid).
fn drawable_stroke(stroke: &Stroke) -> Option<(&Stroke, Vec<f64>)> {
    is_drawable_stroke(stroke).then(|| (stroke, stroke.dash_pattern().unwrap_or_default()))
}

/// A coordinate rounded to 1/100 pixel: short numbers in the stream, far below
/// what any viewer can show.
fn number(value: f64) -> f32 {
    ((value * 100.0).round() / 100.0) as f32
}

fn channel(value: u8) -> f32 {
    f32::from(value) / 255.0
}

fn dimension(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

/// zlib-compress a stream (fast level: plots are large and mostly numbers).
fn deflate(data: &[u8]) -> Result<Vec<u8>> {
    let mut encoder =
        ZlibEncoder::new(Vec::with_capacity(data.len() / 4 + 64), Compression::fast());
    encoder
        .write_all(data)
        .and_then(|()| encoder.finish())
        .map_err(|error| Error::render(format!("PDF compression failed: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Color, MarkerShape, Node, PerItem, Rect, Rects};
    use flate2::read::ZlibDecoder;
    use std::io::Read as _;

    fn scene(nodes: Vec<Node>) -> Scene {
        Scene {
            width: 40.0,
            height: 30.0,
            background: Some(Color::WHITE),
            nodes,
        }
    }

    fn red_rect() -> Node {
        Node::Rects(Rects {
            rects: vec![Rect {
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 30.0,
            }],
            fill: PerItem::Uniform(Some(Color::rgb(255, 0, 0))),
            stroke: None,
        })
    }

    fn label(content: &str) -> Node {
        Node::Text(Text {
            content: content.to_owned(),
            superscript: None,
            suffix: None,
            x: 10.0,
            y: 20.0,
            size: 12.0,
            font: Font::default(),
            color: Color::BLACK,
            horizontal: HorizontalAlign::Start,
            vertical: VerticalAlign::Baseline,
            angle: 0.0,
        })
    }

    /// Every deflated stream of the document, inflated, in order.
    fn streams(pdf: &[u8]) -> Vec<Vec<u8>> {
        let mut found = Vec::new();
        let mut rest = pdf;
        while let Some(start) = find(rest, b"stream\n") {
            let body = &rest[start + 7..];
            let end = find(body, b"\nendstream").unwrap();
            let mut inflated = Vec::new();
            if ZlibDecoder::new(&body[..end])
                .read_to_end(&mut inflated)
                .is_ok()
            {
                found.push(inflated);
            } else {
                found.push(body[..end].to_vec());
            }
            rest = &body[end + b"\nendstream".len()..];
        }
        found
    }

    fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    #[test]
    fn page_size_follows_the_pixel_size_at_its_dpi() {
        let pdf = to_pdf(&scene(vec![red_rect()]), 2.0, 96.0).unwrap();
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.starts_with("%PDF-"));
        assert!(text.trim_end().ends_with("%%EOF"));
        // 80 x 60 pixels at 96 dpi: 60 x 45 points.
        assert!(text.contains("/MediaBox [0 0 60 45]"), "{text}");
        assert!(to_pdf(&scene(vec![]), 1.0, 0.0).is_err());
        assert!(to_pdf(&scene(vec![]), 0.0, 96.0).is_err());
        let error = to_pdf(&scene(vec![]), 1000.0, 1.0).unwrap_err().to_string();
        assert!(
            error.contains("too large") && error.contains("dpi"),
            "{error}"
        );
    }

    #[test]
    fn shapes_are_vector_operators() {
        let points = Node::Points(Points {
            x: vec![5.0, 15.0],
            y: vec![5.0, 15.0],
            shape: PerItem::Uniform(MarkerShape::Circle),
            size: PerItem::Uniform(4.0),
            fill: PerItem::Uniform(Some(Color::rgba(0, 0, 255, 128))),
            stroke: PerItem::Uniform(None),
            stroke_width: PerItem::Uniform(0.0),
        });
        let pdf = to_pdf(&scene(vec![red_rect(), points]), 1.0, 72.0).unwrap();
        let content = String::from_utf8(streams(&pdf)[0].clone()).unwrap();
        // Page transform, white background, the red rect, then two circles.
        assert!(content.starts_with("1 0 0 -1 0 30 cm"), "{content}");
        assert!(content.contains("1 0 0 rg\n0 0 20 30 re\nf"), "{content}");
        // Both circles reuse one marker form: four curves around (0, 0), filled.
        assert!(
            content.contains("q\n1 0 0 1 5 5 cm\n/M0 Do\nQ"),
            "{content}"
        );
        assert!(
            content.contains("q\n1 0 0 1 15 15 cm\n/M0 Do\nQ"),
            "{content}"
        );
        let form = String::from_utf8(streams(&pdf)[1].clone()).unwrap();
        assert!(form.starts_with("2 0 m\n2 1.1 1.1 2 0 2 c\n"), "{form}");
        assert_eq!(form.matches(" c\n").count(), 4, "{form}");
        assert!(form.ends_with("h\nf"), "{form}");
        // Translucent fill through a graphics state; the colour is set once.
        assert!(content.contains("/A1 gs"), "{content}");
        assert_eq!(content.matches("0 0 1 rg").count(), 1, "{content}");
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.contains("/ca 0.5019608"), "{text}");
        assert!(!text.contains("/Subtype /Image"));
    }

    #[test]
    fn text_is_real_text_in_a_subset_font() {
        let pdf = to_pdf(&scene(vec![label("AV x")]), 1.0, 72.0).unwrap();
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.contains("/Subtype /Type0"), "{text}");
        assert!(text.contains("/Encoding /Identity-H"));
        assert!(text.contains("+LiberationSans"));
        assert!(text.contains("/FontFile2"));
        let streams = streams(&pdf);
        let content = String::from_utf8_lossy(&streams[0]);
        assert!(
            content.contains("BT\n1 0 0 -1 10 20 Tm\n/F0 12 Tf\n["),
            "{content}"
        );
        // "AV" kerns: a positive TJ adjustment after A.
        assert!(content.contains("TJ\nET"), "{content}");
        // ToUnicode maps the four glyphs (A, V, space, x) back to text.
        let cmap = streams
            .iter()
            .map(|stream| String::from_utf8_lossy(stream).into_owned())
            .find(|stream| stream.contains("beginbfchar"))
            .unwrap();
        for unicode in ["<0041>", "<0056>", "<0020>", "<0078>"] {
            assert!(cmap.contains(unicode), "{unicode} missing in {cmap}");
        }
    }

    #[test]
    fn kerning_follows_the_shaper() {
        let mut writer = Writer::new(&scene(vec![]));
        let font = writer.font_index(Font::default());
        writer.glyph_run(font, "AV");
        let content = String::from_utf8_lossy(&writer.content.finish().into_vec()).into_owned();
        // [(A) adjust (V)] TJ: a positive adjustment moves V left.
        let adjust: f32 = content
            .split([')', '('])
            .nth(2)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let (glyphs, units) = shape("AV", Font::default());
        let advance = font_ref(Font::default())
            .hmtx()
            .unwrap()
            .advance(GlyphId::new(u32::from(glyphs[0].glyph_id)))
            .unwrap();
        let expected =
            (i32::from(advance) - glyphs[0].x_advance) as f32 * 1000.0 / f32::from(units);
        assert!(
            adjust > 0.0 && (adjust - expected).abs() < 1e-3,
            "{content}"
        );
    }

    #[test]
    fn many_distinct_markers_fall_back_to_inline_paths() {
        let count = MAX_MARKER_FORMS + 10;
        let points = Points {
            x: vec![5.0; count],
            y: vec![5.0; count],
            shape: PerItem::Uniform(MarkerShape::Circle),
            size: PerItem::Varying((0..count).map(|index| 1.0 + index as f64 / 100.0).collect()),
            fill: PerItem::Uniform(Some(Color::BLACK)),
            stroke: PerItem::Uniform(None),
            stroke_width: PerItem::Uniform(0.0),
        };
        let mut writer = Writer::new(&scene(vec![]));
        writer.points(&points);
        assert_eq!(writer.markers.len(), MAX_MARKER_FORMS);
        let content = String::from_utf8(writer.content.finish().into_vec()).unwrap();
        assert_eq!(content.matches(" Do\n").count(), MAX_MARKER_FORMS);
        assert_eq!(content.matches(" c\n").count(), 4 * 10);
    }

    #[test]
    fn translucent_fill_and_outline_are_painted_separately() {
        let rects = |alpha| {
            Node::Rects(Rects {
                rects: vec![Rect {
                    x: 1.0,
                    y: 1.0,
                    width: 5.0,
                    height: 5.0,
                }],
                fill: PerItem::Uniform(Some(Color::rgba(255, 0, 0, alpha))),
                stroke: Some(Stroke::solid(Color::rgba(0, 0, 255, alpha), 2.0)),
            })
        };
        let content = |alpha| {
            let pdf = to_pdf(&scene(vec![rects(alpha)]), 1.0, 72.0).unwrap();
            String::from_utf8_lossy(&streams(&pdf)[0]).into_owned()
        };
        // Opaque: one `B`. Translucent: `f`, then the outline blended over it.
        assert!(content(255).contains("1 1 5 5 re\nB"));
        let translucent = content(128);
        assert!(!translucent.contains("\nB"), "{translucent}");
        assert!(
            translucent.contains("1 1 5 5 re\nf\n1 1 5 5 re\nS"),
            "{translucent}"
        );
        // Marker forms do the same.
        let points = |alpha| Points {
            x: vec![5.0],
            y: vec![5.0],
            shape: PerItem::Uniform(MarkerShape::Square),
            size: PerItem::Uniform(4.0),
            fill: PerItem::Uniform(Some(Color::rgba(255, 0, 0, alpha))),
            stroke: PerItem::Uniform(Some(Color::rgba(0, 0, 255, alpha))),
            stroke_width: PerItem::Uniform(1.0),
        };
        let mut writer = Writer::new(&scene(vec![]));
        writer.points(&points(255));
        writer.points(&points(128));
        let form =
            |index: usize| String::from_utf8_lossy(&writer.markers[index].content).into_owned();
        assert!(form(0).ends_with("re\nB"), "{}", form(0));
        assert!(form(1).ends_with("re\nf\n-2 -2 4 4 re\nS"), "{}", form(1));
    }

    #[test]
    fn images_are_opaque_after_translucent_fills() {
        let image = Node::Image(Image {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            columns: 1,
            rows: 1,
            bitmap: Bitmap::Pixels(vec![Color::rgb(255, 0, 0)]),
        });
        let faint = Node::Rects(Rects {
            rects: vec![Rect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            }],
            fill: PerItem::Uniform(Some(Color::rgba(0, 0, 0, 25))),
            stroke: None,
        });
        let pdf = to_pdf(&scene(vec![faint, image]), 1.0, 72.0).unwrap();
        let content = String::from_utf8_lossy(&streams(&pdf)[0]).into_owned();
        let image_at = content.find("/I0 Do").unwrap();
        let opaque = content[..image_at].rfind("/A0 gs").unwrap();
        assert!(opaque > content.find("/A1 gs").unwrap(), "{content}");
    }

    #[test]
    fn missing_glyphs_map_to_no_text() {
        let mut writer = Writer::new(&scene(vec![]));
        let font = writer.font_index(Font::default());
        writer.glyph_run(font, "日本a");
        assert_eq!(writer.fonts[font].texts[0], None);
        assert!(writer.fonts[font].texts.contains(&Some("a".to_owned())));
    }

    #[test]
    fn superscript_rise_does_not_leak_into_the_next_text() {
        let mut power = label("10");
        if let Node::Text(text) = &mut power {
            text.superscript = Some("4".to_owned());
        }
        let pdf = to_pdf(&scene(vec![power, label("next")]), 1.0, 72.0).unwrap();
        let content = String::from_utf8_lossy(&streams(&pdf)[0]).into_owned();
        // Raised by 0.4 em, then back to 0 before the text object ends.
        let rise = content.find("4.8 Ts").unwrap();
        let reset = content.find("0 Ts").unwrap();
        let end = content.find("ET").unwrap();
        assert!(rise < reset && reset < end, "{content}");
        assert_eq!(content.matches(" Ts").count(), 2, "{content}");
    }

    #[test]
    fn rotated_text_turns_counter_clockwise() {
        let mut node = label("y");
        if let Node::Text(text) = &mut node {
            text.angle = 90.0;
            text.horizontal = HorizontalAlign::Middle;
        }
        let pdf = to_pdf(&scene(vec![node]), 1.0, 72.0).unwrap();
        let content = String::from_utf8_lossy(&streams(&pdf)[0]).into_owned();
        // Baseline up the page (scene -y), glyph tops to the left; centred along y.
        let half = measure("y", 12.0, Font::default()).width / 2.0;
        let expected = format!("0 -1 -1 0 10 {} Tm", number(20.0 + half));
        assert!(content.contains(&expected), "{expected} in {content}");
    }

    #[test]
    fn clips_nest_and_images_carry_alpha() {
        let image = Node::Image(Image {
            rect: Rect {
                x: 2.0,
                y: 3.0,
                width: 4.0,
                height: 2.0,
            },
            columns: 2,
            rows: 1,
            bitmap: Bitmap::Pixels(vec![Color::rgb(255, 0, 0), Color::rgba(0, 0, 255, 0)]),
        });
        let group = Node::Group(Group {
            clip: Some(Rect {
                x: 1.0,
                y: 1.0,
                width: 10.0,
                height: 10.0,
            }),
            role: Some("panel"),
            anchor: None,
            nodes: vec![image],
        });
        let pdf = to_pdf(&scene(vec![group]), 1.0, 72.0).unwrap();
        let streams = streams(&pdf);
        let content = String::from_utf8_lossy(&streams[0]);
        assert!(content.contains("q\n1 1 10 10 re\nW\nn\nq\n4 0 0 -2 2 5 cm\n/I0 Do\nQ\nQ"));
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.contains("/SMask"), "{text}");
        assert!(text.contains("/Interpolate false"));
        // Cells of 2 scene pixels: each pixel repeated 8 times along each side.
        assert!(text.contains("/Width 16"), "{text}");
        assert!(text.contains("/Height 8"));
        let line: Vec<u8> = [[255, 0, 0]; 8]
            .into_iter()
            .chain([[0, 0, 255]; 8])
            .flatten()
            .collect();
        assert!(streams.contains(&line.repeat(8)));
        let mask: Vec<u8> = [255; 8].into_iter().chain([0; 8]).collect();
        assert!(streams.contains(&mask.repeat(8)));
    }

    #[test]
    fn graphics_state_is_restored_after_a_clip() {
        let fill = |color| {
            Node::Rects(Rects {
                rects: vec![Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 5.0,
                    height: 5.0,
                }],
                fill: PerItem::Uniform(Some(color)),
                stroke: None,
            })
        };
        let red = Color::rgb(255, 0, 0);
        let group = Node::Group(Group {
            clip: Some(Rect {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            }),
            role: None,
            anchor: None,
            nodes: vec![fill(Color::rgb(0, 255, 0))],
        });
        let pdf = to_pdf(&scene(vec![fill(red), group, fill(red)]), 1.0, 72.0).unwrap();
        let content = String::from_utf8_lossy(&streams(&pdf)[0]).into_owned();
        // The restore brings back red in the PDF and in the cached state: the last
        // rect sets no colour, and red is written once.
        let after = content.rsplit("Q\n").next().unwrap();
        assert!(!after.contains("rg"), "{content}");
        assert_eq!(content.matches("1 0 0 rg").count(), 1, "{content}");
    }
}
