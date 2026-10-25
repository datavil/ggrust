//! PNG rasterisation.
//!
//! Geometry is drawn straight from the scene with tiny-skia (no SVG round trip:
//! faster, and no parser node limit, so million-point scatters work). Text runs are
//! rendered by resvg from the same SVG text markup the SVG renderer writes, so both
//! outputs place text identically. Only the embedded fonts are visible to resvg, so
//! output is identical on every machine.

use std::sync::{Arc, LazyLock};

use resvg::tiny_skia::{
    self, FillRule, LineCap, LineJoin, Mask, Paint, PathBuilder, Pixmap, Transform,
};
use resvg::usvg::{self, fontdb};

use crate::render::svg::text_document;
use crate::scene::{
    Color, Group, MarkerShape, Node, Path, Points, Rect, Rects, Scene, Stroke, Text,
};
use crate::text::{FONT_FAMILY, FontFace};
use crate::{Error, Result};

/// Render a scene to PNG bytes. `scale` multiplies the pixel size (2.0 for retina).
pub fn to_png(scene: &Scene, scale: f64) -> Result<Vec<u8>> {
    let mut pixmap = new_pixmap(scene.width, scene.height, scale)?;
    if let Some(background) = scene.background {
        pixmap.fill(skia_color(background));
    }
    let mut canvas = Canvas {
        pixmap,
        transform: Transform::from_scale(scale as f32, scale as f32),
        scene_size: (scene.width, scene.height),
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

/// Upper bound on each side, to fail fast on absurd sizes.
const MAX_SIDE: f64 = 16_384.0;

/// Upper bound on total pixels (400 MB of RGBA), to fail fast instead of exhausting memory.
const MAX_PIXELS: f64 = 100_000_000.0;

fn new_pixmap(width: f64, height: f64, scale: f64) -> Result<Pixmap> {
    if !(scale.is_finite() && scale > 0.0) {
        return Err(Error::Render(format!(
            "scale must be positive, got {scale}"
        )));
    }
    let width = (width * scale).ceil();
    let height = (height * scale).ceil();
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
    transform: Transform,
    scene_size: (f64, f64),
}

impl Canvas {
    /// Draw nodes in order; consecutive text nodes are rendered as one batch.
    fn nodes(&mut self, nodes: &[Node], mask: Option<&Mask>) -> Result<()> {
        let mut texts: Vec<&Text> = Vec::new();
        for node in nodes {
            if let Node::Text(text) = node {
                texts.push(text);
                continue;
            }
            self.texts(&texts, mask)?;
            texts.clear();
            match node {
                Node::Group(group) => self.group(group, mask)?,
                Node::Points(points) => self.points(points, mask),
                Node::Path(path) => self.path(path, mask),
                Node::Rects(rects) => self.rects(rects, mask),
                Node::Text(_) => {}
            }
        }
        self.texts(&texts, mask)
    }

    fn group(&mut self, group: &Group, mask: Option<&Mask>) -> Result<()> {
        let Some(clip) = group.clip else {
            return self.nodes(&group.nodes, mask);
        };
        let Some(clip_path) = rect_path(&clip) else {
            return Ok(()); // empty or invalid clip: nothing is visible
        };
        let mut clip_mask = match mask {
            Some(parent) => parent.clone(),
            None => {
                let mut full = Mask::new(self.pixmap.width(), self.pixmap.height())
                    .ok_or_else(|| Error::Render("could not allocate a clip mask".into()))?;
                full.data_mut().fill(255);
                full
            }
        };
        clip_mask.intersect_path(&clip_path, FillRule::Winding, true, self.transform);
        self.nodes(&group.nodes, Some(&clip_mask))
    }

    fn points(&mut self, points: &Points, mask: Option<&Mask>) {
        if points.is_empty() || !points.is_consistent() {
            return;
        }
        let mut fill_paint = paint(Color::BLACK);
        let mut stroke_paint = paint(Color::BLACK);
        for index in 0..points.len() {
            let (x, y, size) = (points.x[index], points.y[index], points.size.get(index));
            if !(x.is_finite() && y.is_finite() && size.is_finite() && size > 0.0) {
                continue;
            }
            let Some(path) = marker_path(points.shape.get(index), x, y, size) else {
                continue;
            };
            if let Some(fill) = points.fill.get(index).filter(|fill| !fill.is_transparent()) {
                fill_paint.set_color(skia_color(fill));
                self.pixmap
                    .fill_path(&path, &fill_paint, FillRule::Winding, self.transform, mask);
            }
            let width = points.stroke_width.get(index);
            if let Some(stroke) = points
                .stroke
                .get(index)
                .filter(|stroke| !stroke.is_transparent())
                && width.is_finite()
                && width > 0.0
            {
                stroke_paint.set_color(skia_color(stroke));
                let style = tiny_skia::Stroke {
                    width: width as f32,
                    ..tiny_skia::Stroke::default()
                };
                self.pixmap
                    .stroke_path(&path, &stroke_paint, &style, self.transform, mask);
            }
        }
    }

    fn path(&mut self, path: &Path, mask: Option<&Mask>) {
        let mut builder = PathBuilder::new();
        let mut drawing = false;
        for &(x, y) in &path.points {
            if !(x.is_finite() && y.is_finite()) {
                drawing = false;
                continue;
            }
            if drawing {
                builder.line_to(x as f32, y as f32);
            } else {
                builder.move_to(x as f32, y as f32);
                drawing = true;
            }
        }
        if path.closed {
            builder.close();
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
            let Some(geometry) = rect_path(rect) else {
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

    fn stroke(&mut self, geometry: &tiny_skia::Path, stroke: &Stroke, mask: Option<&Mask>) {
        if stroke.color.is_transparent() || !(stroke.width.is_finite() && stroke.width > 0.0) {
            return;
        }
        let dash = if stroke.dash.is_empty() {
            None
        } else {
            tiny_skia::StrokeDash::new(
                stroke.dash.iter().map(|&length| length as f32).collect(),
                0.0,
            )
        };
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
    fn texts(&mut self, texts: &[&Text], mask: Option<&Mask>) -> Result<()> {
        if texts.is_empty() {
            return Ok(());
        }
        let (width, height) = self.scene_size;
        let tree = parse_svg(&text_document(width, height, texts))?;
        match mask {
            None => resvg::render(&tree, self.transform, &mut self.pixmap.as_mut()),
            Some(mask) => {
                let mut layer = Pixmap::new(self.pixmap.width(), self.pixmap.height())
                    .ok_or_else(|| Error::Render("could not allocate a text layer".into()))?;
                resvg::render(&tree, self.transform, &mut layer.as_mut());
                self.pixmap.draw_pixmap(
                    0,
                    0,
                    layer.as_ref(),
                    &tiny_skia::PixmapPaint::default(),
                    Transform::identity(),
                    Some(mask),
                );
            }
        }
        Ok(())
    }
}

/// Marker outline in scene pixels, matching the SVG renderer's geometry.
fn marker_path(shape: MarkerShape, x: f64, y: f64, size: f64) -> Option<tiny_skia::Path> {
    let radius = size / 2.0;
    let polygon = |vertices: &[(f64, f64)]| {
        let mut builder = PathBuilder::new();
        for (index, &(vertex_x, vertex_y)) in vertices.iter().enumerate() {
            if index == 0 {
                builder.move_to(vertex_x as f32, vertex_y as f32);
            } else {
                builder.line_to(vertex_x as f32, vertex_y as f32);
            }
        }
        builder.close();
        builder.finish()
    };
    match shape {
        MarkerShape::Circle => PathBuilder::from_circle(x as f32, y as f32, radius as f32),
        MarkerShape::Square => rect_path(&Rect {
            x: x - radius,
            y: y - radius,
            width: size,
            height: size,
        }),
        MarkerShape::Triangle => {
            let height = size * 3f64.sqrt() / 2.0;
            polygon(&[
                (x, y - height * 2.0 / 3.0),
                (x + radius, y + height / 3.0),
                (x - radius, y + height / 3.0),
            ])
        }
        MarkerShape::Diamond => polygon(&[
            (x, y - radius),
            (x + radius, y),
            (x, y + radius),
            (x - radius, y),
        ]),
    }
}

fn rect_path(rect: &Rect) -> Option<tiny_skia::Path> {
    let values = [rect.x, rect.y, rect.width, rect.height];
    if !values.iter().all(|value| value.is_finite()) {
        return None;
    }
    tiny_skia::Rect::from_xywh(
        rect.x as f32,
        rect.y as f32,
        rect.width.max(0.0) as f32,
        rect.height.max(0.0) as f32,
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
    for face in FontFace::ALL {
        database.load_font_data(face.font_data().to_vec());
    }
    database.set_sans_serif_family(FONT_FAMILY);
    Arc::new(database)
});
