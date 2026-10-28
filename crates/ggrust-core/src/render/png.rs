//! PNG rasterisation.
//!
//! Geometry is drawn straight from the scene with tiny-skia (no SVG round trip:
//! faster, and no parser node limit, so million-point scatters work). Text runs are
//! rendered by resvg from the same SVG text markup the SVG renderer writes, so both
//! outputs place text identically. Only the embedded fonts are visible to resvg, so
//! output is identical on every machine.

use std::sync::{Arc, LazyLock};

use resvg::tiny_skia::{
    self, FillRule, IntRect, LineCap, LineJoin, Mask, Paint, PathBuilder, Pixmap, Transform,
};
use resvg::usvg::{self, fontdb};

use crate::render::svg::text_document;
use crate::scene::geometry::{
    ClipBox, is_drawable_marker, is_drawable_rect, is_drawable_stroke, is_drawable_width,
};
use crate::scene::{
    Color, Group, Image, MarkerPart, Node, Path, Points, Rect, Rects, Scene, Stroke, Text,
};
use crate::text::{FONT_FAMILY, Font, MONO_FONT_FAMILY};
use crate::{Error, Result};

/// Render a scene to PNG bytes. `scale` multiplies the pixel size (2.0 for retina).
pub fn to_png(scene: &Scene, scale: f64) -> Result<Vec<u8>> {
    let mut pixmap = new_pixmap(scene.width, scene.height, scale)?;
    if let Some(background) = scene.background {
        pixmap.fill(skia_color(background));
    }
    let mut canvas = Canvas {
        pixmap,
        scale,
        transform: Transform::from_scale(scale as f32, scale as f32),
        scene_size: (scene.width, scene.height),
        clip_box: ClipBox::for_scene(scene.width, scene.height),
    };
    canvas.nodes(&scene.nodes, None)?;
    encode(&canvas.pixmap)
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
        return Err(Error::Render(format!(
            "scale must be positive, got {scale}"
        )));
    }
    let width = (width * scale).ceil();
    let height = (height * scale).ceil();
    if !(width >= 1.0 && height >= 1.0) {
        return Err(Error::Render(format!(
            "image size {width} x {height} pixels is empty; width and height must be at least 1"
        )));
    }
    let side_ok = |side: f64| (1.0..=MAX_SIDE).contains(&side);
    if !(side_ok(width) && side_ok(height) && width * height <= MAX_PIXELS) {
        return Err(Error::Render(format!(
            "image size {width} x {height} pixels is too large; limits are \
             {MAX_SIDE} pixels per side and {MAX_PIXELS} pixels in total"
        )));
    }
    Pixmap::new(width as u32, height as u32)
        .ok_or_else(|| Error::Render("could not allocate the image".into()))
}

/// The bitmap of an [`Image`] (`None` if it is inconsistent).
fn image_pixmap(image: &Image) -> Option<Pixmap> {
    if !image.is_consistent() {
        return None;
    }
    let mut pixmap = Pixmap::new(image.columns, image.rows)?;
    for (pixel, color) in pixmap.pixels_mut().iter_mut().zip(&image.pixels) {
        *pixel = tiny_skia::ColorU8::from_rgba(color.red, color.green, color.blue, color.alpha)
            .premultiply();
    }
    Some(pixmap)
}

/// An [`Image`]'s bitmap as PNG bytes, for the SVG writer.
pub(super) fn image_png(image: &Image) -> Option<Vec<u8>> {
    image_pixmap(image)?.encode_png().ok()
}

fn encode(pixmap: &Pixmap) -> Result<Vec<u8>> {
    pixmap
        .encode_png()
        .map_err(|error| Error::Render(format!("PNG encoding failed: {error}")))
}

fn parse_svg(svg: &str) -> Result<usvg::Tree> {
    let options = usvg::Options {
        font_family: FONT_FAMILY.to_owned(),
        fontdb: FONT_DATABASE.clone(),
        ..usvg::Options::default()
    };
    usvg::Tree::from_str(svg, &options)
        .map_err(|error| Error::Render(format!("internal SVG is invalid: {error}")))
}

struct Canvas {
    pixmap: Pixmap,
    scale: f64,
    transform: Transform,
    scene_size: (f64, f64),
    clip_box: ClipBox,
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
        let mut texts: Vec<&Text> = Vec::new();
        for node in nodes {
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
                Node::Image(image) => self.image(image, mask),
                Node::Text(_) => {}
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
                    .ok_or_else(|| Error::Render("could not allocate a clip mask".into()))?;
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
        let mut builder = PathBuilder::new();
        for subpath in self.clip_box.subpaths(&path.points, path.closed) {
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

    fn image(&mut self, image: &Image, mask: Option<&Mask>) {
        let rect = image.rect;
        let drawable = [rect.x, rect.y, rect.width, rect.height]
            .iter()
            .all(|value| value.is_finite())
            && rect.width > 0.0
            && rect.height > 0.0;
        let Some(bitmap) = image_pixmap(image).filter(|_| drawable) else {
            return;
        };
        let transform = self
            .transform
            .pre_translate(rect.x as f32, rect.y as f32)
            .pre_scale(
                (rect.width / f64::from(image.columns)) as f32,
                (rect.height / f64::from(image.rows)) as f32,
            );
        let paint = tiny_skia::PixmapPaint {
            quality: tiny_skia::FilterQuality::Nearest,
            ..tiny_skia::PixmapPaint::default()
        };
        self.pixmap
            .draw_pixmap(0, 0, bitmap.as_ref(), &paint, transform, mask);
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
            .ok_or_else(|| Error::Render("could not allocate a text layer".into()))?;
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
fn marker_path(part: &MarkerPart) -> Option<(tiny_skia::Path, bool)> {
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

static FONT_DATABASE: LazyLock<Arc<fontdb::Database>> = LazyLock::new(|| {
    let mut database = fontdb::Database::new();
    for font in Font::all() {
        database.load_font_data(font.font_data().to_vec());
    }
    database.set_sans_serif_family(FONT_FAMILY);
    database.set_monospace_family(MONO_FONT_FAMILY);
    Arc::new(database)
});
