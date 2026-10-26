//! SVG/PNG parity on edge cases found by the Phase 0.4 bug hunt.
//!
//! Each case renders a 200 x 100 scene directly (`to_png`) and through the SVG
//! (`svg_to_png(to_svg(..))`) and checks both agree, plus the intended behavior.

#![allow(clippy::unwrap_used)]

use std::time::Instant;

use ggrust_core::render::{SvgOptions, svg_to_png, to_png, to_svg, to_svg_with};
use ggrust_core::scene::{
    Color, Group, HorizontalAlign, MarkerShape, Node, Path, PerItem, Points, Rect, Rects, Scene,
    Stroke, Text, VerticalAlign,
};
use ggrust_core::text::{FontFace, measure};
use resvg::tiny_skia::Pixmap;

fn scene(nodes: Vec<Node>) -> Scene {
    Scene {
        width: 200.0,
        height: 100.0,
        background: Some(Color::WHITE),
        nodes,
    }
}

/// Pixels with any channel below 200, and the ink bounding box (x range).
fn ink(png: &[u8]) -> (usize, Option<(u32, u32)>) {
    let pixmap = Pixmap::decode_png(png).unwrap();
    let mut count = 0;
    let mut range: Option<(u32, u32)> = None;
    for (index, pixel) in pixmap.pixels().iter().enumerate() {
        let color = pixel.demultiply();
        if color.red() < 200 || color.green() < 200 || color.blue() < 200 {
            count += 1;
            let x = index as u32 % pixmap.width();
            range = Some(match range {
                Some((low, high)) => (low.min(x), high.max(x)),
                None => (x, x),
            });
        }
    }
    (count, range)
}

/// Ink of both renderings; asserts they agree within 2%.
fn both(scene: &Scene) -> (usize, Option<(u32, u32)>) {
    let direct = ink(&to_png(scene, 1.0).unwrap());
    let via_svg = ink(&svg_to_png(&to_svg(scene), scene.width, scene.height, 1.0).unwrap());
    let tolerance = (direct.0.max(via_svg.0) / 50).max(3);
    assert!(
        direct.0.abs_diff(via_svg.0) <= tolerance,
        "direct ink {} vs svg ink {}",
        direct.0,
        via_svg.0
    );
    direct
}

fn line(points: Vec<(f64, f64)>, width: f64, dash: Vec<f64>) -> Node {
    Node::Path(Path {
        points,
        closed: false,
        fill: None,
        stroke: Some(Stroke {
            color: Color::BLACK,
            width,
            dash,
        }),
    })
}

fn text(content: &str, x: f64, horizontal: HorizontalAlign) -> Text {
    Text {
        content: content.into(),
        superscript: None,
        x,
        y: 50.0,
        size: 20.0,
        font: FontFace::Plain.into(),
        color: Color::BLACK,
        horizontal,
        vertical: VerticalAlign::Middle,
        angle: 0.0,
    }
}

#[test]
fn odd_dash_patterns_repeat_like_svg() {
    let solid = both(&scene(vec![line(
        vec![(10.0, 50.0), (190.0, 50.0)],
        4.0,
        vec![],
    )]))
    .0;
    for dash in [vec![10.0], vec![10.0, 5.0, 2.0]] {
        let dashed = both(&scene(vec![line(
            vec![(10.0, 50.0), (190.0, 50.0)],
            4.0,
            dash,
        )]))
        .0;
        assert!(dashed < solid * 3 / 4, "{dashed} vs solid {solid}");
    }
}

#[test]
fn zero_or_negative_rects_draw_nothing() {
    for width in [0.0, -30.0] {
        let rects = Node::Rects(Rects {
            rects: vec![Rect {
                x: 50.0,
                y: 20.0,
                width,
                height: 60.0,
            }],
            fill: PerItem::Uniform(Some(Color::BLACK)),
            stroke: Some(Stroke::solid(Color::BLACK, 4.0)),
        });
        assert_eq!(both(&scene(vec![rects])).0, 0);
    }
}

#[test]
fn non_positive_marker_sizes_draw_nothing() {
    for shape in [
        MarkerShape::Circle,
        MarkerShape::Square,
        MarkerShape::Triangle {
            up: true,
            centroid: true,
        },
        MarkerShape::Diamond,
    ] {
        let points = Node::Points(Points {
            x: vec![100.0, 50.0],
            y: vec![50.0, 50.0],
            shape: PerItem::Uniform(shape),
            size: PerItem::Varying(vec![-40.0, 0.0]),
            fill: PerItem::Uniform(Some(Color::BLACK)),
            stroke: PerItem::Uniform(Some(Color::BLACK)),
            stroke_width: PerItem::Uniform(1.0),
        });
        assert_eq!(both(&scene(vec![points])).0, 0, "{shape:?}");
    }
}

#[test]
fn huge_coordinates_keep_the_visible_part() {
    for far in [1e8, 1e9, 1e12, 1e20] {
        let (count, _) = both(&scene(vec![line(
            vec![(10.0, 50.0), (100.0, 50.0), (100.0, far)],
            3.0,
            vec![],
        )]));
        assert!(count > 400, "far={far}: ink {count}");
    }
    let polygon = Node::Path(Path {
        points: vec![(10.0, 10.0), (1e12, 10.0), (10.0, 90.0)],
        closed: true,
        fill: Some(Color::BLACK),
        stroke: None,
    });
    assert!(both(&scene(vec![polygon])).0 > 5_000);
}

#[test]
fn huge_slanted_line_keeps_its_slope_in_svg() {
    let svg = to_svg(&scene(vec![line(
        vec![(0.0, 0.0), (1e20, 2e20)],
        1.0,
        vec![],
    )]));
    // Clipped where it leaves the box (bottom edge y = 100 + 10000): y stays twice x.
    assert!(svg.contains("d=\"M0 0L5050 10100\""), "{svg}");
}

#[test]
fn every_ring_of_a_closed_path_is_closed() {
    let rings = Node::Path(Path {
        points: vec![
            (10.0, 10.0),
            (60.0, 10.0),
            (60.0, 60.0),
            (f64::NAN, 0.0),
            (110.0, 10.0),
            (160.0, 10.0),
            (160.0, 60.0),
        ],
        closed: true,
        fill: None,
        stroke: Some(Stroke::solid(Color::BLACK, 2.0)),
    });
    let svg = to_svg(&scene(vec![rings.clone()]));
    assert_eq!(svg.matches('Z').count(), 2, "{svg}");
    let open = Node::Path(Path {
        closed: false,
        ..match rings.clone() {
            Node::Path(path) => path,
            _ => unreachable!(),
        }
    });
    // Closing adds the two diagonals: noticeably more ink than the open version.
    assert!(both(&scene(vec![rings])).0 > both(&scene(vec![open])).0 + 100);
}

#[test]
fn whitespace_renders_as_measured() {
    // End-aligned "x   " must end three spaces before the anchor, like measure() says.
    let anchor = 190.0;
    let (_, range) = both(&scene(vec![Node::Text(text(
        "x   ",
        anchor,
        HorizontalAlign::End,
    ))]));
    let (_, right) = range.unwrap();
    let spaces = measure("   ", 20.0, FontFace::Plain).width;
    assert!(
        (f64::from(right) - (anchor - spaces)).abs() < 3.0,
        "ink ends at {right}, expected about {}",
        anchor - spaces
    );
    // Tabs and newlines render as single spaces, as measured.
    let (_, tab) = both(&scene(vec![Node::Text(text(
        "a\tb",
        10.0,
        HorizontalAlign::Start,
    ))]));
    let (_, space) = both(&scene(vec![Node::Text(text(
        "a b",
        10.0,
        HorizontalAlign::Start,
    ))]));
    assert_eq!(tab, space);
}

#[test]
fn non_xml_characters_do_not_break_rendering() {
    let scene = scene(vec![Node::Text(text(
        "a\u{FFFF}b\u{FFFE}",
        10.0,
        HorizontalAlign::Start,
    ))]);
    assert!(to_png(&scene, 1.0).is_ok());
    assert!(both(&scene).0 > 0);
}

#[test]
fn id_prefix_with_spaces_still_clips() {
    let clipped = Node::Group(Group {
        clip: Some(Rect {
            x: 0.0,
            y: 0.0,
            width: 50.0,
            height: 100.0,
        }),
        role: None,
        nodes: vec![Node::Rects(Rects {
            rects: vec![Rect {
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 100.0,
            }],
            fill: PerItem::Uniform(Some(Color::BLACK)),
            stroke: None,
        })],
    });
    let scene = scene(vec![clipped]);
    let svg = to_svg_with(
        &scene,
        &SvgOptions {
            id_prefix: Some("my plot)".into()),
        },
    );
    let (_, range) = ink(&svg_to_png(&svg, 200.0, 100.0, 1.0).unwrap());
    assert_eq!(range, Some((0, 49)));
}

#[test]
fn small_opacities_survive_in_svg() {
    let svg = to_svg(&scene(vec![Node::Rects(Rects {
        rects: vec![Rect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        }],
        fill: PerItem::Uniform(Some(Color::rgba(0, 0, 0, 1))),
        stroke: None,
    })]));
    assert!(svg.contains("fill-opacity=\"0.0039\""), "{svg}");
}

#[test]
fn empty_scene_error_says_empty() {
    let error = to_png(
        &Scene {
            width: 0.0,
            height: 0.0,
            background: None,
            nodes: vec![],
        },
        1.0,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("is empty"), "{error}");
}

#[test]
fn interleaved_text_under_a_clip_stays_fast() {
    let mut nodes = Vec::new();
    for index in 0..300 {
        let y = f64::from(index % 100) * 6.0;
        nodes.push(Node::Text(Text {
            y,
            ..text("label", 100.0, HorizontalAlign::Start)
        }));
        nodes.push(line(vec![(0.0, y), (200.0, y)], 1.0, vec![]));
    }
    let scene = Scene {
        width: 2000.0,
        height: 2000.0,
        background: Some(Color::WHITE),
        nodes: vec![Node::Group(Group {
            clip: Some(Rect {
                x: 0.0,
                y: 0.0,
                width: 300.0,
                height: 600.0,
            }),
            role: None,
            nodes,
        })],
    };
    let start = Instant::now();
    to_png(&scene, 2.0).unwrap();
    let elapsed = start.elapsed().as_secs_f64();
    // Was 27.6 s with full-canvas text layers; debug builds are slower, so the bound is loose.
    assert!(elapsed < 10.0, "took {elapsed:.1} s");
}
