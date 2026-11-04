//! Rasterized layers in vector output (ggrastr's `rasterize()`).
//!
//! What it does: [`rasterize_layers`] replaces every [`Raster`] node of a scene
//! with an [`Image`] of its children, drawn by the PNG backend. The bitmap covers
//! the clip region the raster sits in (the panel), snapped outward to the pixel
//! grid of a PNG of the whole scene at the raster's resolution, so its pixels are
//! that PNG's pixels and line up with the vector axes. The image stays inside the
//! same clip group, so panel clips and facets hold. SVG and PDF output call it;
//! everything else in the scene stays vector.
//!
//! What it does not do: change PNG output (raster already) or HTML output (the
//! layer stays vector there, keeping tooltips, zoom and canvas points).

use std::borrow::Cow;

use crate::scene::{Bitmap, Color, Group, Image, Node, Raster, Rect, Scene};
use crate::{Error, Result};

use super::png::render_pixmap_sized;

/// The scene with every [`Raster`] drawn into an embedded [`Image`]. A layout
/// pixel is `inches_per_pixel` inches of output (1/96 for SVG, `scale / dpi` for
/// PDF), so a raster of `dpi` draws `dpi * inches_per_pixel` pixels per layout
/// pixel. Borrows the scene when it has no rasters.
pub fn rasterize_layers(scene: &Scene, inches_per_pixel: f64) -> Result<Cow<'_, Scene>> {
    if !scene.nodes.iter().any(has_raster) {
        return Ok(Cow::Borrowed(scene));
    }
    if !(inches_per_pixel.is_finite() && inches_per_pixel > 0.0) {
        return Err(Error::render(format!(
            "inches per pixel must be positive, got {inches_per_pixel}"
        )));
    }
    let whole = Rect {
        x: 0.0,
        y: 0.0,
        width: scene.width,
        height: scene.height,
    };
    let context = Context {
        inches_per_pixel,
        scene_size: (scene.width, scene.height),
    };
    Ok(Cow::Owned(Scene {
        width: scene.width,
        height: scene.height,
        background: scene.background,
        nodes: context.nodes(&scene.nodes, whole)?,
    }))
}

fn has_raster(node: &Node) -> bool {
    match node {
        Node::Raster(_) => true,
        Node::Group(group) => group.nodes.iter().any(has_raster),
        _ => false,
    }
}

struct Context {
    inches_per_pixel: f64,
    scene_size: (f64, f64),
}

impl Context {
    /// `nodes` with rasters replaced; `region` is the visible area (the
    /// intersection of the enclosing clips and the scene).
    fn nodes(&self, nodes: &[Node], region: Rect) -> Result<Vec<Node>> {
        let mut result = Vec::with_capacity(nodes.len());
        for node in nodes {
            match node {
                Node::Group(group) if group.nodes.iter().any(has_raster) => {
                    let inner = match group.clip {
                        Some(clip) => match intersect(&region, &clip) {
                            Some(inner) => inner,
                            None => continue, // nothing visible
                        },
                        None => region,
                    };
                    result.push(Node::Group(Group {
                        nodes: self.nodes(&group.nodes, inner)?,
                        ..group.clone_without_nodes()
                    }));
                }
                Node::Raster(raster) => {
                    if let Some(image) = self.image(raster, region)? {
                        result.push(Node::Image(image));
                    }
                }
                other => result.push(other.clone()),
            }
        }
        Ok(result)
    }

    /// The raster's children over `region`, or `None` when nothing is visible.
    fn image(&self, raster: &Raster, region: Rect) -> Result<Option<Image>> {
        let dpi = raster.dpi;
        if !(dpi.is_finite() && dpi > 0.0) {
            return Err(Error::render(format!(
                "rasterize dpi must be a positive number, got {dpi}"
            )));
        }
        let scale = dpi * self.inches_per_pixel;
        // Snap outward to the whole-scene pixel grid at `scale`, within the scene.
        let (scene_width, scene_height) = self.scene_size;
        let left = (region.x * scale).floor().max(0.0);
        let top = (region.y * scale).floor().max(0.0);
        let right = ((region.x + region.width) * scale)
            .ceil()
            .min((scene_width * scale).ceil());
        let bottom = ((region.y + region.height) * scale)
            .ceil()
            .min((scene_height * scale).ceil());
        if !(right > left && bottom > top) {
            return Ok(None);
        }
        let (columns, rows) = (right - left, bottom - top);
        let too_large = || {
            Error::render(format!(
                "a rasterized layer at {dpi} dpi needs {columns} x {rows} pixels, too large; \
                 lower its rasterize(dpi=...)"
            ))
        };
        if columns > f64::from(u32::MAX) || rows > f64::from(u32::MAX) {
            return Err(too_large());
        }
        let (origin_x, origin_y) = (left / scale, top / scale);
        let sub_scene = Scene {
            width: columns / scale,
            height: rows / scale,
            background: None,
            nodes: raster
                .nodes
                .iter()
                .map(|node| translated(node, -origin_x, -origin_y))
                .collect(),
        };
        let pixmap = render_pixmap_sized(&sub_scene, scale, columns as u32, rows as u32)?;
        if pixmap.pixels().iter().all(|pixel| pixel.alpha() == 0) {
            return Ok(None);
        }
        let pixels = pixmap
            .pixels()
            .iter()
            .map(|pixel| {
                let color = pixel.demultiply();
                Color::rgba(color.red(), color.green(), color.blue(), color.alpha())
            })
            .collect();
        Ok(Some(Image {
            rect: Rect {
                x: origin_x,
                y: origin_y,
                width: columns / scale,
                height: rows / scale,
            },
            columns: columns as u32,
            rows: rows as u32,
            bitmap: Bitmap::Pixels(pixels),
            upsample: false,
        }))
    }
}

impl Group {
    /// The group's settings with no children.
    fn clone_without_nodes(&self) -> Group {
        Group {
            clip: self.clip,
            role: self.role,
            anchor: self.anchor,
            nodes: Vec::new(),
        }
    }
}

/// The overlap of two rectangles, `None` when empty or not finite.
fn intersect(a: &Rect, b: &Rect) -> Option<Rect> {
    let left = a.x.max(b.x.min(b.x + b.width));
    let top = a.y.max(b.y.min(b.y + b.height));
    let right = (a.x + a.width).min(b.x.max(b.x + b.width));
    let bottom = (a.y + a.height).min(b.y.max(b.y + b.height));
    let finite = [left, top, right, bottom]
        .iter()
        .all(|value| value.is_finite());
    (finite && right > left && bottom > top).then_some(Rect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

/// A copy of `node` moved by (`dx`, `dy`) pixels.
fn translated(node: &Node, dx: f64, dy: f64) -> Node {
    let move_rect = |rect: &Rect| Rect {
        x: rect.x + dx,
        y: rect.y + dy,
        ..*rect
    };
    match node {
        Node::Group(group) => Node::Group(Group {
            clip: group.clip.as_ref().map(move_rect),
            role: group.role,
            anchor: group.anchor.map(|(x, y)| (x + dx, y + dy)),
            nodes: group
                .nodes
                .iter()
                .map(|node| translated(node, dx, dy))
                .collect(),
        }),
        Node::Raster(raster) => Node::Raster(Raster {
            dpi: raster.dpi,
            nodes: raster
                .nodes
                .iter()
                .map(|node| translated(node, dx, dy))
                .collect(),
        }),
        Node::Points(points) => {
            let mut points = points.clone();
            points.x.iter_mut().for_each(|x| *x += dx);
            points.y.iter_mut().for_each(|y| *y += dy);
            Node::Points(points)
        }
        Node::Path(path) => {
            let mut path = path.clone();
            path.points.iter_mut().for_each(|(x, y)| {
                *x += dx;
                *y += dy;
            });
            Node::Path(path)
        }
        Node::Rects(rects) => {
            let mut rects = rects.clone();
            rects
                .rects
                .iter_mut()
                .for_each(|rect| *rect = move_rect(rect));
            Node::Rects(rects)
        }
        Node::Text(text) => {
            let mut text = text.clone();
            text.x += dx;
            text.y += dy;
            Node::Text(text)
        }
        Node::Image(image) => {
            let mut image = image.clone();
            image.rect = move_rect(&image.rect);
            Node::Image(image)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::png::render_pixmap;
    use crate::scene::{MarkerShape, PerItem, Points, Text};
    use crate::text::Font;

    const PANEL: Rect = Rect {
        x: 20.3,
        y: 10.7,
        width: 60.55,
        height: 40.2,
    };

    /// A scene like a plot: a clipped panel holding a raster of points (some
    /// straddling the clip edge) and a vector title outside it.
    fn scene(dpi: f64) -> Scene {
        let count = 3000;
        let x: Vec<f64> = (0..count)
            .map(|i| 15.0 + (i as f64 * 0.731_379_1) % 75.0)
            .collect();
        let y: Vec<f64> = (0..count)
            .map(|i| 5.0 + (i as f64 * 1.317_423_7) % 52.0)
            .collect();
        let fill: Vec<Option<Color>> = (0..count)
            .map(|i| Some(Color::rgba((i * 7 % 256) as u8, 90, 200, 180)))
            .collect();
        let points = Node::Points(Points {
            x,
            y,
            shape: PerItem::Uniform(MarkerShape::Circle),
            size: PerItem::Uniform(2.3),
            fill: PerItem::Varying(fill),
            stroke: PerItem::Uniform(None),
            stroke_width: PerItem::Uniform(0.0),
        });
        let title = Node::Text(Text {
            content: "title".into(),
            superscript: None,
            suffix: None,
            x: 5.0,
            y: 8.0,
            size: 7.0,
            font: Font::default(),
            color: Color::BLACK,
            horizontal: Default::default(),
            vertical: Default::default(),
            angle: 0.0,
            halo: None,
        });
        Scene {
            width: 100.0,
            height: 60.0,
            background: Some(Color::WHITE),
            nodes: vec![
                title,
                Node::Group(Group {
                    clip: Some(PANEL),
                    role: Some("panel"),
                    anchor: None,
                    nodes: vec![Node::Raster(Raster {
                        dpi,
                        nodes: vec![points],
                    })],
                }),
            ],
        }
    }

    fn images(nodes: &[Node], found: &mut Vec<Image>) {
        for node in nodes {
            match node {
                Node::Image(image) => found.push(image.clone()),
                Node::Group(group) => images(&group.nodes, found),
                Node::Raster(_) => panic!("a raster was left in the scene"),
                _ => {}
            }
        }
    }

    #[test]
    fn scene_without_rasters_is_borrowed() {
        let mut plain = scene(300.0);
        plain.nodes.truncate(1);
        let result = rasterize_layers(&plain, 1.0 / 96.0).unwrap();
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn raster_covers_the_clip_on_the_pixel_grid() {
        let dpi = 300.0;
        let scale = dpi / 96.0;
        let original = scene(dpi);
        let result = rasterize_layers(&original, 1.0 / 96.0).unwrap();
        let mut found = Vec::new();
        images(&result.nodes, &mut found);
        assert_eq!(found.len(), 1);
        let image = &found[0];
        assert!(!image.upsample);
        // Snapped outward: whole device pixels at `scale`, covering the panel.
        let left = (PANEL.x * scale).floor();
        let right = ((PANEL.x + PANEL.width) * scale).ceil();
        assert_eq!(f64::from(image.columns), right - left);
        assert!((image.rect.x * scale - left).abs() < 1e-9);
        assert!((image.rect.width * scale - f64::from(image.columns)).abs() < 1e-9);
        assert!(image.rect.x <= PANEL.x && image.rect.y <= PANEL.y);
        assert!(image.rect.x + image.rect.width >= PANEL.x + PANEL.width);
        // The image stays inside the panel's clip group; the title stays text.
        let Node::Group(panel) = &result.nodes[1] else {
            panic!("expected the panel group");
        };
        assert_eq!(panel.clip, Some(PANEL));
        assert!(matches!(result.nodes[0], Node::Text(_)));
    }

    /// The pixels of the rasterized scene are the PNG's pixels: drawn back at the
    /// raster's scale, both match exactly (only clip-edge pixels may differ, where
    /// the clip mask's coverage is applied once more).
    #[test]
    fn raster_pixels_equal_the_png_at_its_scale() {
        for dpi in [96.0, 300.0, 217.0] {
            let scale = dpi / 96.0;
            let original = scene(dpi);
            let rasterized = rasterize_layers(&original, 1.0 / 96.0).unwrap();
            let expected = render_pixmap(&original, scale).unwrap();
            let actual = render_pixmap(&rasterized, scale).unwrap();
            assert_eq!(expected.width(), actual.width());
            let edge = |column: u32, row: u32| {
                let (x, y) = (f64::from(column), f64::from(row));
                let near = |value: f64, edge: f64| (value - edge * scale).abs() <= 1.5;
                near(x, PANEL.x)
                    || near(x + 1.0, PANEL.x + PANEL.width)
                    || near(y, PANEL.y)
                    || near(y + 1.0, PANEL.y + PANEL.height)
            };
            let mut compared = 0;
            let mut total = 0u64;
            let mut bad = Vec::new();
            for row in 0..expected.height() {
                for column in 0..expected.width() {
                    if edge(column, row) {
                        continue;
                    }
                    let index = (row * expected.width() + column) as usize;
                    let (a, b) = (expected.pixels()[index], actual.pixels()[index]);
                    let difference = [
                        a.red().abs_diff(b.red()),
                        a.green().abs_diff(b.green()),
                        a.blue().abs_diff(b.blue()),
                        a.alpha().abs_diff(b.alpha()),
                    ];
                    // Translucent markers composite onto transparent first, then the
                    // bitmap over the background: 8-bit rounding, a few levels at most.
                    // A one-pixel shift differs by up to 255 at marker edges.
                    total += difference.iter().map(|&d| u64::from(d)).sum::<u64>();
                    if difference.iter().any(|&d| d > 3) {
                        bad.push((column, row, a.red(), b.red()));
                    }
                    compared += 1;
                }
            }
            assert!(
                bad.is_empty(),
                "dpi {dpi}: {} bad of {compared}: {:?}",
                bad.len(),
                &bad[..bad.len().min(30)]
            );
            assert!(compared > 1000);
            let mean = total as f64 / (compared as f64 * 4.0);
            assert!(mean < 0.25, "dpi {dpi}: mean difference {mean}");
        }
    }

    #[test]
    fn hidden_or_empty_rasters_draw_nothing() {
        let mut hidden = scene(300.0);
        if let Node::Group(group) = &mut hidden.nodes[1] {
            group.clip = Some(Rect {
                x: 500.0,
                y: 500.0,
                width: 10.0,
                height: 10.0,
            });
        }
        let result = rasterize_layers(&hidden, 1.0 / 96.0).unwrap();
        let mut found = Vec::new();
        images(&result.nodes, &mut found);
        assert!(found.is_empty());
    }

    #[test]
    fn bad_dpi_and_huge_rasters_are_errors() {
        let error = rasterize_layers(&scene(f64::NAN), 1.0 / 96.0)
            .unwrap_err()
            .to_string();
        assert!(error.contains("dpi"), "{error}");
        let error = rasterize_layers(&scene(4800.0), 10.0)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("too large") && error.contains("dpi"),
            "{error}"
        );
    }
}
