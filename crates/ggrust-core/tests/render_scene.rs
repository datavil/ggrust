//! Scene -> SVG / PNG rendering, checked against a golden SVG and by pixel probes.
//!
//! Re-bless the golden file after an intended change with
//! `GGRUST_BLESS=1 cargo test -p ggrust-core --test render_scene` and review the diff.
//! The PNG is written to `tests/visual/output/` for visual inspection.

#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::{Path as FilePath, PathBuf};

use ggrust_core::render::{SvgOptions, svg_to_png, to_png, to_svg_with};
use ggrust_core::scene::{
    Color, Group, HorizontalAlign, MarkerShape, Node, Path, PerItem, Points, Rect, Rects, Scene,
    Stroke, Text, VerticalAlign,
};
use ggrust_core::text::{FontFace, measure};
use resvg::tiny_skia::Pixmap;

const PANEL: Rect = Rect {
    x: 60.0,
    y: 20.0,
    width: 300.0,
    height: 200.0,
};

fn text(content: &str, x: f64, y: f64, size: f64) -> Text {
    Text {
        content: content.into(),
        superscript: None,
        x,
        y,
        size,
        face: FontFace::Plain,
        color: Color::BLACK,
        horizontal: HorizontalAlign::Start,
        vertical: VerticalAlign::Baseline,
        angle: 0.0,
    }
}

/// A hand-built scene exercising every primitive: a scatter panel with grid lines,
/// clipped markers of each shape, a dashed path, a polygon, tiles, and text with
/// every alignment, rotation and face.
fn showcase() -> Scene {
    let grey = Color::rgb(235, 235, 235);
    let mut grid = Vec::new();
    for step in 1..6 {
        let x = PANEL.x + PANEL.width * f64::from(step) / 6.0;
        grid.push(Node::Path(Path {
            points: vec![(x, PANEL.y), (x, PANEL.y + PANEL.height)],
            closed: false,
            fill: None,
            stroke: Some(Stroke::solid(grey, 1.0)),
        }));
    }
    let markers = Points {
        x: vec![100.0, 160.0, 220.0, 280.0, 340.0, 365.0, f64::NAN],
        y: vec![60.0, 120.0, 80.0, 60.0, 100.0, 100.0, 50.0],
        shape: PerItem::Varying(vec![
            MarkerShape::Circle,
            MarkerShape::Square,
            MarkerShape::Triangle,
            MarkerShape::Diamond,
            MarkerShape::Circle,
            MarkerShape::Circle, // centre outside the panel: clipped
            MarkerShape::Circle, // NaN: skipped
        ]),
        size: PerItem::Varying(vec![16.0, 16.0, 18.0, 18.0, 30.0, 30.0, 10.0]),
        fill: PerItem::Varying(vec![
            Some(Color::rgb(228, 26, 28)),
            Some(Color::rgb(55, 126, 184)),
            Some(Color::rgb(77, 175, 74)),
            None,
            Some(Color::rgb(152, 78, 163).with_opacity(0.5)),
            Some(Color::rgb(255, 127, 0)),
            Some(Color::BLACK),
        ]),
        stroke: PerItem::Uniform(Some(Color::BLACK)),
        stroke_width: PerItem::Uniform(1.0),
    };
    let panel = Group {
        clip: Some(PANEL),
        role: Some("panel"),
        nodes: vec![
            Node::Rects(Rects {
                rects: vec![PANEL],
                fill: PerItem::Uniform(Some(Color::rgb(250, 250, 250))),
                stroke: None,
            }),
            Node::Group(Group {
                clip: None,
                role: Some("grid"),
                nodes: grid,
            }),
            Node::Rects(Rects {
                rects: (0..4)
                    .map(|index| Rect {
                        x: 80.0 + 20.0 * f64::from(index),
                        y: 180.0,
                        width: 20.0,
                        height: 20.0,
                    })
                    .collect(),
                fill: PerItem::Varying(vec![
                    Some(Color::from_hex("#440154").unwrap()),
                    Some(Color::from_hex("#31688e").unwrap()),
                    Some(Color::from_hex("#35b779").unwrap()),
                    Some(Color::from_hex("#fde725").unwrap()),
                ]),
                stroke: Some(Stroke::solid(Color::WHITE, 1.0)),
            }),
            Node::Path(Path {
                points: vec![
                    (70.0, 210.0),
                    (150.0, 40.0),
                    (f64::NAN, 0.0), // gap: the line restarts
                    (200.0, 200.0),
                    (350.0, 30.0),
                ],
                closed: false,
                fill: None,
                stroke: Some(Stroke {
                    color: Color::rgb(80, 80, 80),
                    width: 1.5,
                    dash: vec![6.0, 3.0],
                }),
            }),
            Node::Path(Path {
                points: vec![(250.0, 190.0), (300.0, 140.0), (340.0, 200.0)],
                closed: true,
                fill: Some(Color::rgb(255, 255, 51).with_opacity(0.6)),
                stroke: Some(Stroke::solid(Color::BLACK, 1.0)),
            }),
            Node::Points(markers),
        ],
    };
    let mut nodes = vec![
        Node::Group(panel),
        Node::Rects(Rects {
            rects: vec![PANEL],
            fill: PerItem::Uniform(None),
            stroke: Some(Stroke::solid(Color::BLACK, 1.0)),
        }),
    ];
    let mut labels = vec![
        Text {
            horizontal: HorizontalAlign::Middle,
            vertical: VerticalAlign::Top,
            ..text("UMAP1 <x & y>", PANEL.x + PANEL.width / 2.0, 228.0, 13.0)
        },
        Text {
            horizontal: HorizontalAlign::Middle,
            vertical: VerticalAlign::Bottom,
            angle: 90.0,
            ..text("UMAP2", 40.0, PANEL.y + PANEL.height / 2.0, 13.0)
        },
        Text {
            face: FontFace::Bold,
            ..text("Bold title", 380.0, 40.0, 16.0)
        },
        Text {
            face: FontFace::Italic,
            color: Color::rgb(120, 120, 120),
            ..text("italic subtitle", 380.0, 60.0, 11.0)
        },
        Text {
            horizontal: HorizontalAlign::End,
            vertical: VerticalAlign::Middle,
            angle: 45.0,
            ..text("rotated 45", 520.0, 140.0, 11.0)
        },
    ];
    // Measurement probe: text with a rect of exactly its measured width behind it.
    let probe = text("Measured", 380.0, 200.0, 20.0);
    let width = measure(&probe.content, probe.size, probe.face).width;
    nodes.push(Node::Rects(Rects {
        rects: vec![Rect {
            x: probe.x,
            y: 182.0,
            width,
            height: 24.0,
        }],
        fill: PerItem::Uniform(Some(Color::rgb(198, 219, 239))),
        stroke: None,
    }));
    labels.push(probe);
    nodes.extend(labels.into_iter().map(Node::Text));
    Scene {
        width: 540.0,
        height: 250.0,
        background: Some(Color::WHITE),
        nodes,
    }
}

fn repository_root() -> PathBuf {
    FilePath::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn render() -> (String, Pixmap) {
    let scene = showcase();
    let svg = to_svg_with(
        &scene,
        &SvgOptions {
            id_prefix: "golden".into(),
        },
    );
    let png = to_png(&scene, 2.0).unwrap();
    let output = repository_root().join("tests/visual/output");
    fs::create_dir_all(&output).unwrap();
    fs::write(output.join("scene_showcase.png"), &png).unwrap();
    fs::write(output.join("scene_showcase.svg"), &svg).unwrap();
    (svg, Pixmap::decode_png(&png).unwrap())
}

/// RGBA of a pixel at scene coordinates (the PNG is rendered at 2x).
fn pixel(pixmap: &Pixmap, x: f64, y: f64) -> [u8; 4] {
    let color = pixmap
        .pixel((x * 2.0) as u32, (y * 2.0) as u32)
        .unwrap()
        .demultiply();
    [color.red(), color.green(), color.blue(), color.alpha()]
}

#[test]
fn svg_matches_golden() {
    let (svg, _) = render();
    let golden = FilePath::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/scene_showcase.svg");
    if std::env::var_os("GGRUST_BLESS").is_some() {
        fs::write(&golden, &svg).unwrap();
    }
    let expected = fs::read_to_string(&golden).expect("golden exists; bless with GGRUST_BLESS=1");
    assert_eq!(
        svg, expected,
        "SVG differs from golden; review and re-bless if intended"
    );
}

#[test]
fn svg_is_well_formed_and_has_no_nan() {
    let (svg, _) = render();
    assert!(!svg.contains("NaN") && !svg.contains("inf"));
    assert!(svg.contains("&lt;x &amp; y&gt;"));
    assert!(svg.contains("clip-path=\"url(#golden-clip0)\""));
    assert!(svg.starts_with("<svg ") && svg.ends_with("</svg>\n"));
}

#[test]
fn png_pixels_land_where_the_scene_says() {
    let (_, pixmap) = render();
    assert_eq!((pixmap.width(), pixmap.height()), (1080, 500));
    // Background and panel fill.
    assert_eq!(pixel(&pixmap, 5.0, 5.0), [255, 255, 255, 255]);
    // Marker centres take their fill colors.
    assert_eq!(pixel(&pixmap, 100.0, 60.0), [228, 26, 28, 255]);
    assert_eq!(pixel(&pixmap, 160.0, 120.0), [55, 126, 184, 255]);
    // Hollow diamond: centre shows the panel background.
    assert_eq!(pixel(&pixmap, 280.0, 60.0), [250, 250, 250, 255]);
    // Half-transparent purple over the light panel.
    let [red, green, blue, _] = pixel(&pixmap, 340.0, 100.0);
    assert!(
        red > 190 && green > 150 && green < 175 && blue > 200,
        "{red} {green} {blue}"
    );
    // Clipped marker: inside the panel it is drawn, outside (x > 360) it is not.
    assert_eq!(pixel(&pixmap, 357.0, 100.0), [255, 127, 0, 255]);
    assert_eq!(pixel(&pixmap, 372.0, 100.0), [255, 255, 255, 255]);
    // Tile colors.
    assert_eq!(pixel(&pixmap, 90.0, 190.0), [68, 1, 84, 255]);
}

#[test]
fn rendered_text_fits_its_measured_width() {
    let (_, pixmap) = render();
    // Dark ink of "Measured" must lie within [x, x + measured width] (+/- 1.5px for
    // side bearings and anti-aliasing), and reach close to both ends.
    let width = measure("Measured", 20.0, FontFace::Plain).width;
    let (left, right) = (380.0, 380.0 + width);
    let mut ink_min = f64::MAX;
    let mut ink_max = f64::MIN;
    for py in (182 * 2)..(206 * 2) {
        for px in (370 * 2)..(540 * 2) {
            let color = pixmap.pixel(px, py).unwrap().demultiply();
            if color.red() < 100 && color.green() < 100 && color.blue() < 100 {
                ink_min = ink_min.min(f64::from(px) / 2.0);
                ink_max = ink_max.max(f64::from(px) / 2.0);
            }
        }
    }
    assert!(
        ink_min >= left - 1.5 && ink_min <= left + 2.5,
        "ink starts at {ink_min}, text at {left}"
    );
    assert!(
        ink_max <= right + 1.5 && ink_max >= right - 2.5,
        "ink ends at {ink_max}, measured {right}"
    );
}

#[test]
fn invalid_png_scale_and_size_are_errors() {
    let scene = showcase();
    assert!(to_png(&scene, 0.0).is_err());
    assert!(to_png(&scene, f64::NAN).is_err());
    let huge = Scene {
        width: 1e6,
        ..scene
    };
    assert!(to_png(&huge, 1.0).is_err());
}

#[test]
fn direct_png_matches_resvg_rendering_of_the_svg() {
    // The PNG renderer draws geometry itself; the SVG goes through resvg. Both must
    // agree, or SVG and PNG exports of the same plot would look different.
    let scene = showcase();
    let direct = Pixmap::decode_png(&to_png(&scene, 2.0).unwrap()).unwrap();
    let svg = to_svg_with(&scene, &SvgOptions::default());
    let via_svg =
        Pixmap::decode_png(&svg_to_png(&svg, scene.width, scene.height, 2.0).unwrap()).unwrap();
    let mut total_difference: u64 = 0;
    let mut differing_pixels = 0usize;
    for (left, right) in direct.pixels().iter().zip(via_svg.pixels()) {
        let difference = [
            left.red().abs_diff(right.red()),
            left.green().abs_diff(right.green()),
            left.blue().abs_diff(right.blue()),
            left.alpha().abs_diff(right.alpha()),
        ]
        .into_iter()
        .max()
        .unwrap_or(0);
        total_difference += u64::from(difference);
        if difference > 32 {
            differing_pixels += 1;
        }
    }
    let pixel_count = direct.pixels().len();
    let mean = total_difference as f64 / pixel_count as f64;
    // Anti-aliasing may differ by a few levels along edges; shapes must not.
    assert!(mean < 0.5, "mean channel difference {mean}");
    assert!(
        differing_pixels * 1000 < pixel_count,
        "{differing_pixels} of {pixel_count} pixels differ noticeably"
    );
}
