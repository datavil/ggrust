//! PNG rasterisation.
//!
//! Geometry is drawn straight from the scene with tiny-skia (no SVG round trip:
//! faster, and no parser node limit, so million-point scatters work). Text runs are
//! rendered by resvg from the same SVG text markup the SVG renderer writes, so both
//! outputs place text identically. Only the embedded fonts are visible to resvg, so
//! output is identical on every machine.

use std::sync::{Arc, Mutex};

use resvg::tiny_skia::{
    self, FillRule, IntRect, LineCap, LineJoin, Mask, Paint, PathBuilder, Pixmap, Transform,
};
use resvg::usvg::{self, fontdb};

use crate::render::sprite::Sprites;
use crate::render::svg::text_document;
use crate::scene::geometry::{
    ClipBox, is_drawable_marker, is_drawable_rect, is_drawable_stroke, is_drawable_width,
};
use crate::scene::{
    Bitmap, Color, Group, Image, MarkerPart, Node, Path, Points, Rect, Rects, Scene, Stroke, Text,
};
use crate::text::{FONT_FAMILY, Font, MONO_FONT_FAMILY, registered_faces};
use crate::{Error, Result};

/// Point batches with at least this many markers are stamped from sprites
/// (see `render::sprite`); smaller ones are traced exactly.
const SPRITE_MIN_MARKERS: usize = 1000;

/// Open paths with more segments than this are stroked in chunks of
/// `STROKE_CHUNK_SEGMENTS` (see `Canvas::stroke_chunked`); shorter ones in one go.
/// Translucent strokes need a layer per path (about 6 ms at 3000 x 2400 px), which
/// only pays off once a single stroke is slow.
const STROKE_CHUNK_MIN_SEGMENTS: usize = 1024;
const STROKE_CHUNK_MIN_SEGMENTS_TRANSLUCENT: usize = 4096;
const STROKE_CHUNK_SEGMENTS: usize = 256;

/// Screen resolution (lets-plot's PNG default `pHYs`): one layout pixel per pixel.
pub const SCREEN_DPI: f64 = 96.0;

/// Render a scene to PNG bytes. `scale` multiplies the pixel size (2.0 for retina).
pub fn to_png(scene: &Scene, scale: f64) -> Result<Vec<u8>> {
    to_png_with_dpi(scene, scale, SCREEN_DPI)
}

/// [`to_png`] with the resolution written to the PNG (`pHYs`, dots per inch).
pub fn to_png_with_dpi(scene: &Scene, scale: f64, dpi: f64) -> Result<Vec<u8>> {
    encode_with_dpi(&render_pixmap(scene, scale)?, dpi)
}

/// Rasterise a scene; `scale` multiplies the pixel size.
pub(super) fn render_pixmap(scene: &Scene, scale: f64) -> Result<Pixmap> {
    let pixmap = new_pixmap(scene.width, scene.height, scale)?;
    draw_scene(scene, scale, pixmap)
}

/// Rasterise a scene into exactly `columns` x `rows` pixels at `scale` (a
/// rasterized layer, see `render::raster`), within the usual size limits.
pub(super) fn render_pixmap_sized(
    scene: &Scene,
    scale: f64,
    columns: u32,
    rows: u32,
) -> Result<Pixmap> {
    let (width, height) = (f64::from(columns), f64::from(rows));
    let side_ok = |side: f64| (1.0..=MAX_SIDE).contains(&side);
    if !(side_ok(width) && side_ok(height) && width * height <= MAX_PIXELS) {
        return Err(Error::render(format!(
            "a rasterized layer needs {width} x {height} pixels, too large; limits are \
             {MAX_SIDE} pixels per side and {MAX_PIXELS} pixels in total. Lower the \
             layer's rasterize(dpi=...)"
        )));
    }
    let pixmap = Pixmap::new(columns, rows)
        .ok_or_else(|| Error::render("could not allocate a rasterized layer"))?;
    draw_scene(scene, scale, pixmap)
}

fn draw_scene(scene: &Scene, scale: f64, mut pixmap: Pixmap) -> Result<Pixmap> {
    if let Some(background) = scene.background {
        pixmap.fill(skia_color(background));
    }
    let mut canvas = Canvas {
        pixmap,
        scale,
        transform: Transform::from_scale(scale as f32, scale as f32),
        scene_size: (scene.width, scene.height),
        clip_box: ClipBox::for_scene(scene.width, scene.height),
        sprites: Sprites::new(scale),
    };
    canvas.nodes(&scene.nodes, None)?;
    Ok(canvas.pixmap)
}

/// Straight-alpha RGBA rows (`width` x `height`, top row first) as a
/// `data:image/png;base64,...` URL, for `geom_imshow`. `compression` is a zlib
/// style level 0..=9; `None` is the fastest deflate (images are only
/// embedded, so speed beats size).
pub fn rgba_png_data_url(
    width: u32,
    height: u32,
    rgba: &[u8],
    compression: Option<u8>,
) -> Result<String> {
    let expected = u64::from(width) * u64::from(height) * 4;
    if width == 0 || height == 0 || rgba.len() as u64 != expected {
        return Err(Error::render(format!(
            "expected {width} x {height} RGBA pixels ({expected} bytes), got {} bytes",
            rgba.len()
        )));
    }
    let level = match compression {
        None | Some(1..=2) => ::png::Compression::Fastest,
        Some(0) => ::png::Compression::NoCompression,
        Some(3..=5) => ::png::Compression::Fast,
        Some(6..=7) => ::png::Compression::Balanced,
        Some(_) => ::png::Compression::High,
    };
    let failed =
        |error: ::png::EncodingError| Error::render(format!("PNG encoding failed: {error}"));
    let mut bytes = Vec::new();
    let mut encoder = ::png::Encoder::new(&mut bytes, width, height);
    encoder.set_color(::png::ColorType::Rgba);
    encoder.set_depth(::png::BitDepth::Eight);
    encoder.set_compression(level);
    let mut writer = encoder.write_header().map_err(failed)?;
    writer.write_image_data(rgba).map_err(failed)?;
    writer.finish().map_err(failed)?;
    let mut url = String::from("data:image/png;base64,");
    super::svg::push_base64(&mut url, &bytes);
    Ok(url)
}

/// Render SVG text to PNG bytes with resvg (slow path; usvg caps the node count).
///
/// Used to cross-check [`to_png`] against the SVG renderer, and for SVG input.
pub fn svg_to_png(svg: &str, width: f64, height: f64, scale: f64) -> Result<Vec<u8>> {
    let mut pixmap = new_pixmap(width, height, scale)?;
    let tree = parse_svg(svg)?;
    let scale = scale as f32;
    resvg::render(
        &tree,
        Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    encode(&pixmap)
}

/// Upper bound on each side: lets-plot's largest ggsize (10,000) at scale 3.
const MAX_SIDE: f64 = 32_768.0;

/// Upper bound on total pixels (1.6 GB of RGBA: a 10,000 x 10,000 plot at scale 2),
/// to fail fast instead of exhausting memory.
const MAX_PIXELS: f64 = 400_000_000.0;

fn new_pixmap(width: f64, height: f64, scale: f64) -> Result<Pixmap> {
    if !(scale.is_finite() && scale > 0.0) {
        return Err(Error::render(format!(
            "scale must be positive, got {scale}"
        )));
    }
    // As lets-plot (`PlotImageExport`): the layout size rounded up to whole pixels,
    // then scaled and rounded down; the epsilons keep 400.0000001 at 400 and
    // 1199.9999 at 1200.
    let pixels = |side: f64| ((side - 1e-6).ceil() * scale + 1e-6).floor();
    let (width, height) = (pixels(width), pixels(height));
    if !(width >= 1.0 && height >= 1.0) {
        return Err(Error::render(format!(
            "image size {width} x {height} pixels is empty; width and height must be at least 1"
        )));
    }
    let side_ok = |side: f64| (1.0..=MAX_SIDE).contains(&side);
    if !(side_ok(width) && side_ok(height) && width * height <= MAX_PIXELS) {
        return Err(Error::render(format!(
            "image size {width} x {height} pixels is too large; limits are \
             {MAX_SIDE} pixels per side and {MAX_PIXELS} pixels in total"
        )));
    }
    Pixmap::new(width as u32, height as u32)
        .ok_or_else(|| Error::render("could not allocate the image"))
}

/// The bitmap of an [`Image`] (`None` if it is inconsistent).
fn image_pixmap(image: &Image) -> Result<Option<Pixmap>> {
    if !image.is_consistent() {
        return Ok(None);
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
    let Some(mut pixmap) = Pixmap::new(image.columns, image.rows) else {
        return Ok(None);
    };
    for (pixel, color) in pixmap.pixels_mut().iter_mut().zip(pixels) {
        *pixel = tiny_skia::ColorU8::from_rgba(color.red, color.green, color.blue, color.alpha)
            .premultiply();
    }
    Ok(Some(pixmap))
}

/// An [`Image`]'s bitmap as straight-alpha RGBA PNG bytes, for the SVG writer
/// (no premultiplied round trip, which would shift faint colors). Only for
/// [`Bitmap::Pixels`]: an encoded PNG is embedded as given.
pub(super) fn image_png(image: &Image) -> Option<Vec<u8>> {
    let Bitmap::Pixels(pixels) = &image.bitmap else {
        return None;
    };
    if !image.is_consistent() {
        return None;
    }
    let mut bytes = Vec::new();
    let mut encoder = ::png::Encoder::new(&mut bytes, image.columns, image.rows);
    encoder.set_color(::png::ColorType::Rgba);
    encoder.set_depth(::png::BitDepth::Eight);
    let data: Vec<u8> = pixels
        .iter()
        .flat_map(|color| [color.red, color.green, color.blue, color.alpha])
        .collect();
    let mut writer = encoder.write_header().ok()?;
    writer.write_image_data(&data).ok()?;
    writer.finish().ok()?;
    Some(bytes)
}

fn encode(pixmap: &Pixmap) -> Result<Vec<u8>> {
    pixmap
        .encode_png()
        .map_err(|error| Error::render(format!("PNG encoding failed: {error}")))
}

/// Straight-alpha RGBA PNG with a `pHYs` chunk of `dpi` dots per inch.
fn encode_with_dpi(pixmap: &Pixmap, dpi: f64) -> Result<Vec<u8>> {
    if !(dpi.is_finite() && dpi > 0.0) {
        return Err(Error::render(format!("dpi must be positive, got {dpi}")));
    }
    let failed =
        |error: ::png::EncodingError| Error::render(format!("PNG encoding failed: {error}"));
    let data: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let color = pixel.demultiply();
            [color.red(), color.green(), color.blue(), color.alpha()]
        })
        .collect();
    let mut bytes = Vec::new();
    let mut encoder = ::png::Encoder::new(&mut bytes, pixmap.width(), pixmap.height());
    encoder.set_color(::png::ColorType::Rgba);
    encoder.set_depth(::png::BitDepth::Eight);
    // fdeflate: 25-30% faster renders than the default zlib; files about 2x larger.
    encoder.set_compression(::png::Compression::Fast);
    let per_meter = (dpi / 0.0254).round() as u32;
    encoder.set_pixel_dims(Some(::png::PixelDimensions {
        xppu: per_meter,
        yppu: per_meter,
        unit: ::png::Unit::Meter,
    }));
    let mut writer = encoder.write_header().map_err(failed)?;
    writer.write_image_data(&data).map_err(failed)?;
    writer.finish().map_err(failed)?;
    Ok(bytes)
}

fn parse_svg(svg: &str) -> Result<usvg::Tree> {
    let options = usvg::Options {
        font_family: FONT_FAMILY.to_owned(),
        fontdb: font_database(),
        font_resolver: usvg::FontResolver {
            select_font: usvg::FontResolver::default_font_selector(),
            select_fallback: fallback_selector(),
        },
        ..usvg::Options::default()
    };
    usvg::Tree::from_str(svg, &options)
        .map_err(|error| Error::render(format!("internal SVG is invalid: {error}")))
}

struct Canvas {
    pixmap: Pixmap,
    scale: f64,
    transform: Transform,
    scene_size: (f64, f64),
    clip_box: ClipBox,
    sprites: Sprites,
}

/// The active clip: a mask over the whole canvas plus the device-pixel bounds it
/// can be non-zero in (used to size text layers).
#[derive(Clone, Copy)]
struct Clip<'a> {
    mask: &'a Mask,
    bounds: IntRect,
}

impl Canvas {
    /// Draw nodes in order; consecutive text nodes are rendered as one batch.
    fn nodes(&mut self, nodes: &[Node], clip: Option<Clip>) -> Result<()> {
        // Unclipped groups (anchored labels) are drawn inline, so their texts
        // still join one batch.
        fn inline<'a>(nodes: &'a [Node], flat: &mut Vec<&'a Node>) {
            for node in nodes {
                match node {
                    Node::Group(group) if group.clip.is_none() => inline(&group.nodes, flat),
                    // PNG output is raster already: rasterized layers draw as they are.
                    Node::Raster(raster) => inline(&raster.nodes, flat),
                    other => flat.push(other),
                }
            }
        }
        let mut flat = Vec::with_capacity(nodes.len());
        inline(nodes, &mut flat);
        let mut texts: Vec<&Text> = Vec::new();
        for node in flat {
            if let Node::Text(text) = node {
                texts.push(text);
                continue;
            }
            self.texts(&texts, clip)?;
            texts.clear();
            let mask = clip.map(|clip| clip.mask);
            match node {
                Node::Group(group) => self.group(group, clip)?,
                Node::Points(points) => self.points(points, mask),
                Node::Path(path) => self.path(path, mask),
                Node::Rects(rects) => self.rects(rects, mask),
                Node::Image(image) => self.image(image, mask)?,
                Node::Text(_) | Node::Raster(_) => {}
            }
        }
        self.texts(&texts, clip)
    }

    fn group(&mut self, group: &Group, parent: Option<Clip>) -> Result<()> {
        let Some(clip) = group.clip else {
            return self.nodes(&group.nodes, parent);
        };
        let Some(clip) = self.clip_box.clip_rect(&clip) else {
            return Ok(()); // empty or invalid clip: nothing is visible
        };
        let Some(clip_path) = rect_path(&clip) else {
            return Ok(());
        };
        let Some(bounds) = self.device_bounds(&clip, parent.map(|parent| parent.bounds)) else {
            return Ok(());
        };
        let mut clip_mask = match parent {
            Some(parent) => parent.mask.clone(),
            None => {
                let mut full = Mask::new(self.pixmap.width(), self.pixmap.height())
                    .ok_or_else(|| Error::render("could not allocate a clip mask"))?;
                full.data_mut().fill(255);
                full
            }
        };
        clip_mask.intersect_path(&clip_path, FillRule::Winding, true, self.transform);
        self.nodes(
            &group.nodes,
            Some(Clip {
                mask: &clip_mask,
                bounds,
            }),
        )
    }

    /// Device-pixel bounds of a scene rectangle, within the canvas and the parent clip.
    fn device_bounds(&self, rect: &Rect, parent: Option<IntRect>) -> Option<IntRect> {
        let scale = self.scale;
        let mut left = (rect.x * scale).floor().max(0.0);
        let mut top = (rect.y * scale).floor().max(0.0);
        let mut right = ((rect.x + rect.width) * scale)
            .ceil()
            .min(f64::from(self.pixmap.width()));
        let mut bottom = ((rect.y + rect.height) * scale)
            .ceil()
            .min(f64::from(self.pixmap.height()));
        if let Some(parent) = parent {
            left = left.max(f64::from(parent.x()));
            top = top.max(f64::from(parent.y()));
            right = right.min(f64::from(parent.right()));
            bottom = bottom.min(f64::from(parent.bottom()));
        }
        IntRect::from_ltrb(left as i32, top as i32, right as i32, bottom as i32)
    }

    fn points(&mut self, points: &Points, mask: Option<&Mask>) {
        if points.is_empty() || !points.is_consistent() {
            return;
        }
        let mut fill_paint = paint(Color::BLACK);
        let mut stroke_paint = paint(Color::BLACK);
        let stamped = points.len() >= SPRITE_MIN_MARKERS;
        for index in 0..points.len() {
            let (x, y, size) = (points.x[index], points.y[index], points.size.get(index));
            if !is_drawable_marker(x, y, size) || !self.clip_box.contains(x, y) {
                continue;
            }
            let width = points.stroke_width.get(index);
            let fill = points.fill.get(index).filter(|fill| !fill.is_transparent());
            let stroke = points
                .stroke
                .get(index)
                .filter(|stroke| !stroke.is_transparent() && is_drawable_width(width));
            let shape = points.shape.get(index);
            if stamped
                && self.sprites.stamp(
                    &mut self.pixmap,
                    mask,
                    (x, y),
                    (shape, size, width),
                    fill,
                    stroke,
                )
            {
                continue;
            }
            shape.for_each_part(x, y, size, width, |part| {
                let Some((path, closed)) = marker_path(&part) else {
                    return;
                };
                if let Some(fill) = fill.filter(|_| closed) {
                    fill_paint.set_color(skia_color(fill));
                    self.pixmap.fill_path(
                        &path,
                        &fill_paint,
                        FillRule::Winding,
                        self.transform,
                        mask,
                    );
                }
                if let Some(stroke) = stroke {
                    stroke_paint.set_color(skia_color(stroke));
                    let style = tiny_skia::Stroke {
                        width: width as f32,
                        ..tiny_skia::Stroke::default()
                    };
                    self.pixmap
                        .stroke_path(&path, &stroke_paint, &style, self.transform, mask);
                }
            });
        }
    }

    fn path(&mut self, path: &Path, mask: Option<&Mask>) {
        let subpaths = self.clip_box.subpaths(&path.points, path.closed);
        if !path.closed
            && let Some(stroke) = path
                .stroke
                .as_ref()
                .filter(|stroke| is_drawable_stroke(stroke) && stroke.dash_pattern().is_none())
            && subpaths
                .iter()
                .map(|run| run.len().saturating_sub(1))
                .sum::<usize>()
                > if stroke.color.alpha == u8::MAX {
                    STROKE_CHUNK_MIN_SEGMENTS
                } else {
                    STROKE_CHUNK_MIN_SEGMENTS_TRANSLUCENT
                }
            && self.stroke_chunked(&subpaths, stroke, mask)
        {
            return;
        }
        let mut builder = PathBuilder::new();
        for subpath in subpaths {
            for (index, &(x, y)) in subpath.iter().enumerate() {
                if index == 0 {
                    builder.move_to(x as f32, y as f32);
                } else {
                    builder.line_to(x as f32, y as f32);
                }
            }
            if path.closed {
                builder.close(); // every ring is closed, not only the last
            }
        }
        let Some(geometry) = builder.finish() else {
            return;
        };
        if path.closed
            && let Some(fill) = path.fill.filter(|fill| !fill.is_transparent())
        {
            self.pixmap.fill_path(
                &geometry,
                &paint(fill),
                FillRule::Winding,
                self.transform,
                mask,
            );
        }
        if let Some(stroke) = &path.stroke {
            self.stroke(&geometry, stroke, mask);
        }
    }

    fn rects(&mut self, rects: &Rects, mask: Option<&Mask>) {
        if !rects.is_consistent() {
            return;
        }
        let mut fill_paint = paint(Color::BLACK);
        for (index, rect) in rects.rects.iter().enumerate() {
            let Some(geometry) = self
                .clip_box
                .clip_rect(rect)
                .and_then(|rect| rect_path(&rect))
            else {
                continue;
            };
            if let Some(fill) = rects.fill.get(index).filter(|fill| !fill.is_transparent()) {
                fill_paint.set_color(skia_color(fill));
                self.pixmap.fill_path(
                    &geometry,
                    &fill_paint,
                    FillRule::Winding,
                    self.transform,
                    mask,
                );
            }
            if let Some(stroke) = &rects.stroke {
                self.stroke(&geometry, stroke, mask);
            }
        }
    }

    fn image(&mut self, image: &Image, mask: Option<&Mask>) -> Result<()> {
        let rect = image.rect;
        let drawable = [rect.x, rect.y, rect.width, rect.height]
            .iter()
            .all(|value| value.is_finite())
            && rect.width > 0.0
            && rect.height > 0.0;
        if !drawable {
            return Ok(());
        }
        let Some(bitmap) = image_pixmap(image)? else {
            return Ok(());
        };
        let transform = self
            .transform
            .pre_translate(rect.x as f32, rect.y as f32)
            .pre_scale(
                (rect.width / f64::from(image.columns)) as f32,
                (rect.height / f64::from(image.rows)) as f32,
            );
        let paint = tiny_skia::PixmapPaint {
            quality: if image.smooth {
                tiny_skia::FilterQuality::Bilinear
            } else {
                tiny_skia::FilterQuality::Nearest
            },
            ..tiny_skia::PixmapPaint::default()
        };
        self.pixmap
            .draw_pixmap(0, 0, bitmap.as_ref(), &paint, transform, mask);
        Ok(())
    }

    fn stroke(&mut self, geometry: &tiny_skia::Path, stroke: &Stroke, mask: Option<&Mask>) {
        if !is_drawable_stroke(stroke) {
            return;
        }
        let dash = stroke.dash_pattern().and_then(|pattern| {
            tiny_skia::StrokeDash::new(pattern.iter().map(|&length| length as f32).collect(), 0.0)
        });
        let style = tiny_skia::Stroke {
            width: stroke.width as f32,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            dash,
            ..tiny_skia::Stroke::default()
        };
        self.pixmap
            .stroke_path(geometry, &paint(stroke.color), &style, self.transform, mask);
    }

    /// Stroke long open polylines in chunks of `STROKE_CHUNK_SEGMENTS` segments.
    ///
    /// tiny-skia walks every active edge of a path on every quarter-pixel scanline,
    /// through a linked list. A dense line (`geom_line` over 1M rows: 166k segments
    /// per group, each spanning much of the panel height) keeps ~100k edges active
    /// in a list far larger than the CPU cache, and stroking it in one go took 17 s.
    /// Chunks keep each list small: same picture, about 5x faster.
    ///
    /// Each chunk repeats the previous chunk's last segment, so the join at the
    /// boundary is drawn. Chunks overlap, so a translucent stroke is drawn opaque
    /// into a layer that is then composited at the stroke's alpha (no double
    /// blending); the layer covers the path's bounds only. Returns `false`, with
    /// nothing drawn, when that layer cannot be allocated: the caller then strokes
    /// the path in one go. Only anti-aliased edge pixels differ from a single stroke: where
    /// chunks overlap, coverage combines as `1 - (1 - a)(1 - b)` instead of the
    /// exact union.
    fn stroke_chunked(
        &mut self,
        subpaths: &[Vec<(f64, f64)>],
        stroke: &Stroke,
        mask: Option<&Mask>,
    ) -> bool {
        // Translucent: an opaque layer over the path's device bounds, padded by the
        // widest miter join (limit 4: twice the stroke width) and one pixel.
        let mut layer = None;
        if stroke.color.alpha < u8::MAX {
            let pad = 2.0 * stroke.width + 1.0 / self.scale;
            let (mut left, mut top) = (f64::INFINITY, f64::INFINITY);
            let (mut right, mut bottom) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
            for &(x, y) in subpaths.iter().flatten() {
                (left, right) = (left.min(x), right.max(x));
                (top, bottom) = (top.min(y), bottom.max(y));
            }
            let device = |value: f64, limit: u32| (value * self.scale).clamp(0.0, f64::from(limit));
            let (width, height) = (self.pixmap.width(), self.pixmap.height());
            let x0 = device(left - pad, width).floor();
            let y0 = device(top - pad, height).floor();
            let x1 = device(right + pad, width).ceil();
            let y1 = device(bottom + pad, height).ceil();
            if !(x1 > x0 && y1 > y0) {
                return true; // nothing on the canvas
            }
            let Some(pixmap) = Pixmap::new((x1 - x0) as u32, (y1 - y0) as u32) else {
                return false;
            };
            layer = Some((pixmap, x0 as i32, y0 as i32));
        }
        let style = tiny_skia::Stroke {
            width: stroke.width as f32,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            ..tiny_skia::Stroke::default()
        };
        let stroke_paint = paint(Color {
            alpha: u8::MAX,
            ..stroke.color
        });
        for subpath in subpaths {
            let mut start = 0;
            while start + 1 < subpath.len() {
                let end = (start + STROKE_CHUNK_SEGMENTS).min(subpath.len() - 1);
                let first = start.saturating_sub(1);
                let mut builder = PathBuilder::new();
                builder.move_to(subpath[first].0 as f32, subpath[first].1 as f32);
                for &(x, y) in &subpath[first + 1..=end] {
                    builder.line_to(x as f32, y as f32);
                }
                if let Some(geometry) = builder.finish() {
                    match layer.as_mut() {
                        Some((layer, x0, y0)) => layer.stroke_path(
                            &geometry,
                            &stroke_paint,
                            &style,
                            self.transform.post_translate(-*x0 as f32, -*y0 as f32),
                            None,
                        ),
                        None => self.pixmap.stroke_path(
                            &geometry,
                            &stroke_paint,
                            &style,
                            self.transform,
                            mask,
                        ),
                    }
                }
                start = end;
            }
        }
        if let Some((layer, x0, y0)) = layer {
            let layer_paint = tiny_skia::PixmapPaint {
                opacity: f32::from(stroke.color.alpha) / 255.0,
                quality: tiny_skia::FilterQuality::Nearest,
                ..tiny_skia::PixmapPaint::default()
            };
            self.pixmap.draw_pixmap(
                x0,
                y0,
                layer.as_ref(),
                &layer_paint,
                Transform::identity(),
                mask,
            );
        }
        true
    }

    /// Render a run of text nodes through resvg (glyph outlines and shaping).
    /// Under a clip, the run goes through a layer the size of the clip bounds.
    fn texts(&mut self, texts: &[&Text], clip: Option<Clip>) -> Result<()> {
        if texts.is_empty() {
            return Ok(());
        }
        let (width, height) = self.scene_size;
        let tree = parse_svg(&text_document(width, height, texts))?;
        let Some(clip) = clip else {
            resvg::render(&tree, self.transform, &mut self.pixmap.as_mut());
            return Ok(());
        };
        let bounds = clip.bounds;
        let mut layer = Pixmap::new(bounds.width(), bounds.height())
            .ok_or_else(|| Error::render("could not allocate a text layer"))?;
        let shifted = self
            .transform
            .post_translate(-(bounds.x() as f32), -(bounds.y() as f32));
        resvg::render(&tree, shifted, &mut layer.as_mut());
        self.pixmap.draw_pixmap(
            bounds.x(),
            bounds.y(),
            layer.as_ref(),
            &tiny_skia::PixmapPaint::default(),
            Transform::identity(),
            Some(clip.mask),
        );
        Ok(())
    }
}

/// A marker part as a path, and whether it is closed (fillable).
pub(super) fn marker_path(part: &MarkerPart) -> Option<(tiny_skia::Path, bool)> {
    match part {
        MarkerPart::Circle { x, y, radius } => {
            PathBuilder::from_circle(*x as f32, *y as f32, *radius as f32).map(|path| (path, true))
        }
        MarkerPart::Rect(rect) => rect_path(rect).map(|path| (path, true)),
        MarkerPart::Polygon(vertices) => {
            let mut builder = PathBuilder::new();
            for (index, &(vertex_x, vertex_y)) in vertices.iter().enumerate() {
                if index == 0 {
                    builder.move_to(vertex_x as f32, vertex_y as f32);
                } else {
                    builder.line_to(vertex_x as f32, vertex_y as f32);
                }
            }
            builder.close();
            builder.finish().map(|path| (path, true))
        }
        MarkerPart::Line(start, end) => {
            let mut builder = PathBuilder::new();
            builder.move_to(start.0 as f32, start.1 as f32);
            builder.line_to(end.0 as f32, end.1 as f32);
            builder.finish().map(|path| (path, false))
        }
    }
}

fn rect_path(rect: &Rect) -> Option<tiny_skia::Path> {
    if !is_drawable_rect(rect) {
        return None;
    }
    tiny_skia::Rect::from_xywh(
        rect.x as f32,
        rect.y as f32,
        rect.width as f32,
        rect.height as f32,
    )
    .map(PathBuilder::from_rect)
}

fn paint(color: Color) -> Paint<'static> {
    let mut paint = Paint {
        anti_alias: true,
        ..Paint::default()
    };
    paint.set_color(skia_color(color));
    paint
}

fn skia_color(color: Color) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba8(color.red, color.green, color.blue, color.alpha)
}

/// resvg's per-glyph fallback, except for registered faces: a character a
/// registered font lacks draws as its `.notdef` box, as `text::measure` measures
/// it and PDF draws it (a fallback glyph would not fit the measured width).
fn fallback_selector() -> usvg::FallbackSelectionFn<'static> {
    let default = usvg::FontResolver::default_fallback_selector();
    Box::new(move |character, used, database| {
        let registered = used
            .first()
            .and_then(|id| database.face(*id))
            .is_some_and(|face| {
                face.families
                    .first()
                    .is_some_and(|(family, _)| family.starts_with("ggrust-"))
            });
        if registered {
            None
        } else {
            default(character, used, database)
        }
    })
}

/// The font database resvg draws text with: the embedded fonts, plus every
/// registered face under its unique name (`LoadedFont::css_name`, which the SVG
/// writer puts first in `font-family`), so resvg picks exactly the measured face.
/// Rebuilt only when the font registry changes.
fn font_database() -> Arc<fontdb::Database> {
    static CACHE: Mutex<Option<(u64, Arc<fontdb::Database>)>> = Mutex::new(None);
    let mut cache = CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (generation, faces) = registered_faces();
    if let Some((built, database)) = cache.as_ref()
        && *built == generation
    {
        return Arc::clone(database);
    }
    let mut database = fontdb::Database::new();
    for font in Font::all() {
        database.load_font_source(fontdb::Source::Binary(Arc::new(font.font_data())));
    }
    database.set_sans_serif_family(FONT_FAMILY);
    database.set_monospace_family(MONO_FONT_FAMILY);
    for face in faces {
        database.push_face_info(fontdb::FaceInfo {
            id: fontdb::ID::dummy(),
            source: fontdb::Source::Binary(Arc::new(face.data)),
            index: face.index,
            families: vec![(
                face.css_name.clone(),
                fontdb::Language::English_UnitedStates,
            )],
            post_script_name: face.postscript_name.clone(),
            // The SVG writer gives registered text no weight or slant: the face is
            // chosen by its unique name alone.
            style: fontdb::Style::Normal,
            weight: fontdb::Weight::NORMAL,
            stretch: fontdb::Stretch::Normal,
            monospaced: face.monospaced,
        });
    }
    let database = Arc::new(database);
    *cache = Some((generation, Arc::clone(&database)));
    database
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_faces_get_no_glyph_fallback() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fonts/LiberationSerif-Regular.ttf");
        crate::text::register_font(&path, Some("Fallback Test Serif")).expect("registers");
        let mut database = font_database();
        let id_of = |prefix: &str, database: &fontdb::Database| {
            database
                .faces()
                .find(|face| {
                    face.families
                        .iter()
                        .any(|(name, _)| name.starts_with(prefix))
                })
                .map(|face| face.id)
                .expect("face in the database")
        };
        let registered = id_of("ggrust-", &database);
        let embedded = id_of("Liberation Mono", &database);
        let select = fallback_selector();
        let default = usvg::FontResolver::default_fallback_selector();
        // A character the serif lacks would come from another face by default.
        assert_eq!(select('\u{2318}', &[registered], &mut database), None);
        assert_eq!(
            select('A', &[embedded], &mut database),
            default('A', &[embedded], &mut database)
        );
    }
}
