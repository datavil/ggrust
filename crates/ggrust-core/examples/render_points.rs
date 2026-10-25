//! Time rendering N circles to SVG and PNG: `cargo run --release --example render_points -- 1000000`.

use std::time::Instant;

use ggrust_core::render::{to_png, to_svg};
use ggrust_core::scene::{Color, MarkerShape, Node, PerItem, Points, Scene};

fn main() {
    let count: usize = std::env::args()
        .nth(1)
        .and_then(|argument| argument.parse().ok())
        .unwrap_or(1_000_000);
    // Deterministic pseudo-random positions (LCG), no extra dependency.
    let mut state: u64 = 42;
    let mut next = || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        (state >> 11) as f64 / (1u64 << 53) as f64
    };
    let x: Vec<f64> = (0..count).map(|_| 20.0 + 560.0 * next()).collect();
    let y: Vec<f64> = (0..count).map(|_| 20.0 + 360.0 * next()).collect();
    let scene = Scene {
        width: 600.0,
        height: 400.0,
        background: Some(Color::WHITE),
        nodes: vec![Node::Points(Points {
            x,
            y,
            shape: PerItem::Uniform(MarkerShape::Circle),
            size: PerItem::Uniform(2.0),
            fill: PerItem::Uniform(Some(Color::rgb(55, 126, 184))),
            stroke: PerItem::Uniform(None),
            stroke_width: PerItem::Uniform(0.0),
        })],
    };
    let start = Instant::now();
    let svg = to_svg(&scene);
    println!(
        "{count} points -> SVG: {:.0} ms, {:.1} MB",
        start.elapsed().as_secs_f64() * 1000.0,
        svg.len() as f64 / 1e6
    );
    let start = Instant::now();
    let png = to_png(&scene, 1.0).expect("render");
    println!(
        "{count} points -> PNG: {:.0} ms, {:.2} MB",
        start.elapsed().as_secs_f64() * 1000.0,
        png.len() as f64 / 1e6
    );
}
