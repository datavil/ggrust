//! PNG point batches of 1000+ markers are stamped from sprites (`render::sprite`);
//! smaller batches are traced. Both must give the same picture.

#![allow(clippy::unwrap_used)]

use ggrust_core::render::to_png;
use ggrust_core::scene::{Color, Group, MarkerShape, Node, PerItem, Points, Rect, Scene};
use resvg::tiny_skia::Pixmap;

const SHAPES: [MarkerShape; 17] = [
    MarkerShape::Circle,
    MarkerShape::Square,
    MarkerShape::Diamond,
    MarkerShape::Triangle {
        up: true,
        centroid: true,
    },
    MarkerShape::Triangle {
        up: false,
        centroid: false,
    },
    MarkerShape::Plus,
    MarkerShape::Cross,
    MarkerShape::SquareCross,
    MarkerShape::Star,
    MarkerShape::DiamondPlus,
    MarkerShape::CirclePlus,
    MarkerShape::TriangleUpDown,
    MarkerShape::SquarePlus,
    MarkerShape::CircleCross,
    MarkerShape::SquareTriangle,
    MarkerShape::Dot,
    MarkerShape::Circle,
];

/// 1200 markers of `shape` with varied sizes, colors (some translucent, some
/// missing) and outline widths, some off the canvas edge. Four geometries, so
/// each is seen often enough to be stamped.
fn markers(shape: MarkerShape) -> Points {
    let mut state: u64 = 7;
    let mut next = || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        (state >> 11) as f64 / (1u64 << 53) as f64
    };
    let count = 1200;
    let mut points = Points {
        keys: None,
        x: Vec::new(),
        y: Vec::new(),
        shape: PerItem::Uniform(shape),
        size: PerItem::Varying(Vec::new()),
        fill: PerItem::Varying(Vec::new()),
        stroke: PerItem::Varying(Vec::new()),
        stroke_width: PerItem::Varying(Vec::new()),
    };
    let palette = [
        Some(Color::rgb(228, 26, 28)),
        Some(Color::rgba(55, 126, 184, 120)),
        Some(Color::rgb(71, 71, 71)),
        Some(Color::rgba(77, 175, 74, 30)),
        None,
    ];
    let sizes = [2.5, 9.3];
    let widths = [0.3, 1.0];
    for _ in 0..count {
        points.x.push(-10.0 + 320.0 * next());
        points.y.push(-10.0 + 220.0 * next());
        let pick = |values: usize, random: f64| (random * values as f64) as usize % values;
        if let PerItem::Varying(values) = &mut points.size {
            values.push(sizes[pick(sizes.len(), next())]);
        }
        if let PerItem::Varying(values) = &mut points.fill {
            values.push(palette[pick(palette.len(), next())]);
        }
        if let PerItem::Varying(values) = &mut points.stroke {
            values.push(palette[pick(palette.len(), next())]);
        }
        if let PerItem::Varying(values) = &mut points.stroke_width {
            values.push(widths[pick(widths.len(), next())]);
        }
    }
    points
}

/// The same markers as batches of `chunk`, in order.
fn chunks(points: &Points, chunk: usize) -> Vec<Node> {
    let take = |range: std::ops::Range<usize>| Points {
        keys: None,
        x: points.x[range.clone()].to_vec(),
        y: points.y[range.clone()].to_vec(),
        shape: slice(&points.shape, range.clone()),
        size: slice(&points.size, range.clone()),
        fill: slice(&points.fill, range.clone()),
        stroke: slice(&points.stroke, range.clone()),
        stroke_width: slice(&points.stroke_width, range),
    };
    (0..points.len())
        .step_by(chunk)
        .map(|start| Node::Points(take(start..(start + chunk).min(points.len()))))
        .collect()
}

fn slice<T: Clone>(values: &PerItem<T>, range: std::ops::Range<usize>) -> PerItem<T> {
    match values {
        PerItem::Uniform(value) => PerItem::Uniform(value.clone()),
        PerItem::Varying(values) => PerItem::Varying(values[range].to_vec()),
    }
}

fn render(nodes: Vec<Node>, clipped: bool, scale: f64) -> Pixmap {
    let nodes = if clipped {
        vec![Node::Group(Group {
            key: None,
            clip: Some(Rect {
                x: 40.5,
                y: 30.25,
                width: 200.0,
                height: 120.0,
            }),
            role: None,
            anchor: None,
            nodes,
        })]
    } else {
        nodes
    };
    let scene = Scene {
        width: 300.0,
        height: 200.0,
        metadata: false,
        background: Some(Color::WHITE),
        nodes,
    };
    Pixmap::decode_png(&to_png(&scene, scale).unwrap()).unwrap()
}

/// Mean and maximum channel difference, and the share of pixels off by more than 32.
fn compare(left: &Pixmap, right: &Pixmap) -> (f64, u8, f64) {
    let (mut total, mut largest, mut differing) = (0u64, 0u8, 0usize);
    for (a, b) in left.pixels().iter().zip(right.pixels()) {
        let difference = [
            a.red().abs_diff(b.red()),
            a.green().abs_diff(b.green()),
            a.blue().abs_diff(b.blue()),
            a.alpha().abs_diff(b.alpha()),
        ]
        .into_iter()
        .max()
        .unwrap();
        total += u64::from(difference);
        largest = largest.max(difference);
        differing += usize::from(difference > 32);
    }
    let count = left.pixels().len() as f64;
    (total as f64 / count, largest, differing as f64 / count)
}

/// The markers moved by `shift` device pixels on both axes.
fn shifted(points: &Points, shift: f64, scale: f64) -> Points {
    let mut moved = points.clone();
    moved.x.iter_mut().for_each(|x| *x += shift / scale);
    moved.y.iter_mut().for_each(|y| *y += shift / scale);
    moved
}

#[test]
fn stamped_markers_match_traced_markers() {
    // Sprite centres move up to 1/32 px. tiny-skia's anti-aliasing samples
    // quarter-pixel scanlines, so any shift that small already flips edge pixels
    // by tens of levels: the bar for the mean is what a 1/32 px shift of the
    // traced markers does, not zero. Thin line tips (cross, star) flip whole
    // pixels on sub-1/32 px moves: up to 0.6% more strongly differing pixels
    // than the shift in this dense overlapping scene. Centres on exact 1/16 px steps match bit for bit.
    for (shape, clipped, scale) in SHAPES.iter().flat_map(|&shape| {
        [
            (shape, false, 1.0),
            (shape, true, 2.0),
            (shape, false, 1.37),
        ]
    }) {
        let points = markers(shape);
        {
            let stamped = render(vec![Node::Points(points.clone())], clipped, scale);
            let traced = render(chunks(&points, 999), clipped, scale);
            let nudge =
                |shift: f64| render(chunks(&shifted(&points, shift, scale), 999), clipped, scale);
            let (mean, largest, differing) = compare(&stamped, &traced);
            let (noise, _, noise_differing) = compare(&nudge(1.0 / 32.0), &traced);
            assert!(
                mean <= noise * 1.5 + 0.05 && differing < noise_differing + 0.006,
                "{shape:?}, clipped {clipped}, scale {scale}: mean {mean} (shift noise {noise}), \
                 max {largest}, {differing} differ (shift {noise_differing})"
            );
        }
    }
}

#[test]
fn small_batches_are_traced_exactly() {
    // Under the sprite threshold nothing changes: rendering in one batch or
    // many gives identical pixels.
    let points = markers(MarkerShape::SquareCross);
    let small = chunks(&points, 500);
    let traced = chunks(&points, 999);
    let (mean, largest, _) = compare(&render(small, true, 2.0), &render(traced, true, 2.0));
    assert_eq!((mean, largest), (0.0, 0));
}

#[test]
fn rare_geometries_are_traced() {
    // Every marker a different size (a continuous size scale): no sprites, the
    // picture is exactly the traced one.
    let mut points = markers(MarkerShape::Circle);
    points.size = PerItem::Varying((0..points.len()).map(|i| 2.0 + i as f64 * 0.01).collect());
    let stamped = render(vec![Node::Points(points.clone())], false, 1.0);
    let traced = render(chunks(&points, 999), false, 1.0);
    assert_eq!(compare(&stamped, &traced).1, 0);
}

#[test]
fn large_markers_centred_off_a_tiny_canvas_are_drawn() {
    let points = Points {
        keys: None,
        x: vec![60.0; 1000],
        y: vec![5.0; 1000],
        shape: PerItem::Uniform(MarkerShape::Circle),
        size: PerItem::Uniform(200.0),
        fill: PerItem::Uniform(Some(Color::rgb(10, 200, 30))),
        stroke: PerItem::Uniform(None),
        stroke_width: PerItem::Uniform(0.0),
    };
    let scene = Scene {
        width: 10.0,
        height: 10.0,
        metadata: false,
        background: Some(Color::WHITE),
        nodes: vec![Node::Points(points)],
    };
    let pixmap = Pixmap::decode_png(&to_png(&scene, 1.0).unwrap()).unwrap();
    let pixel = pixmap.pixel(5, 5).unwrap();
    assert_eq!((pixel.red(), pixel.green(), pixel.blue()), (10, 200, 30));
}

#[test]
fn centres_on_sprite_steps_match_traced_markers_exactly() {
    // A grid of thin crosses whose centres sit on exact 1/16 px offsets.
    let (x, y): (Vec<f64>, Vec<f64>) = (0..1000)
        .map(|i| {
            let x = 10.0 + (i % 40) as f64 * 7.0 + (i % 16) as f64 / 16.0;
            let y = 10.0 + (i / 40) as f64 * 7.0 + (i % 13) as f64 / 16.0;
            (x, y)
        })
        .unzip();
    let points = Points {
        keys: None,
        x,
        y,
        shape: PerItem::Uniform(MarkerShape::Cross),
        size: PerItem::Uniform(5.0),
        fill: PerItem::Uniform(None),
        stroke: PerItem::Uniform(Some(Color::BLACK)),
        stroke_width: PerItem::Uniform(1.0),
    };
    let stamped = render(vec![Node::Points(points.clone())], false, 1.0);
    let traced = render(chunks(&points, 999), false, 1.0);
    assert!(compare(&stamped, &traced).1 <= 1);
}
