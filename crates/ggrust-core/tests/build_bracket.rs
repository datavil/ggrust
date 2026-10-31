//! End-to-end: spec + Arrow data -> scene, for `geom_bracket` (Phase 2.6).

#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use arrow_array::{ArrayRef, Float64Array, RecordBatch, StringArray};
use ggrust_core::build::{BuiltPlot, PlotData, build_plot};
use ggrust_core::data::DataFrame;
use ggrust_core::scene::{Color, Group, Node, Path, Text};
use ggrust_core::spec::{Figure, PlotSpec};

fn spec(json: &str) -> PlotSpec {
    match Figure::from_json(json).unwrap() {
        Figure::Plot(plot) => *plot,
        Figure::Subplots(_) => panic!("expected a plot"),
    }
}

/// Two brackets: [1, 2] at 3 and [2, 4] at 4.
fn brackets() -> DataFrame {
    let batch = RecordBatch::try_from_iter(vec![
        (
            "lo",
            Arc::new(Float64Array::from(vec![1.0, 2.0])) as ArrayRef,
        ),
        (
            "hi",
            Arc::new(Float64Array::from(vec![2.0, 4.0])) as ArrayRef,
        ),
        (
            "at",
            Arc::new(Float64Array::from(vec![3.0, 4.0])) as ArrayRef,
        ),
        (
            "text",
            Arc::new(StringArray::from(vec!["*", "ns"])) as ArrayRef,
        ),
    ])
    .unwrap();
    DataFrame::new(batch.schema(), vec![batch]).unwrap()
}

fn build(plot_options: &str, layer_options: &str) -> BuiltPlot {
    try_build(plot_options, layer_options).unwrap()
}

fn try_build(plot_options: &str, layer_options: &str) -> ggrust_core::Result<BuiltPlot> {
    let json = format!(
        r#"{{"kind": "plot"{plot_options}, "layers": [{{"geom": "bracket",
            "mapping": {{"xmin": "lo", "xmax": "hi", "y": "at", "label": "text"}}
            {layer_options}}}]}}"#
    );
    build_plot(
        &spec(&json),
        &PlotData {
            plot: Some(brackets()),
            layers: vec![],
        },
    )
}

fn panel(built: &BuiltPlot) -> &Group {
    built
        .scene
        .groups_with_role("panel")
        .into_iter()
        .next()
        .expect("a panel group")
}

fn paths(built: &BuiltPlot) -> Vec<&Path> {
    panel(built)
        .flattened()
        .into_iter()
        .filter_map(|node| match node {
            Node::Path(path) => Some(path),
            _ => None,
        })
        .collect()
}

fn texts(built: &BuiltPlot) -> Vec<&Text> {
    panel(built)
        .flattened()
        .into_iter()
        .filter_map(|node| match node {
            Node::Text(text) => Some(text),
            _ => None,
        })
        .collect()
}

#[test]
fn brackets_are_drawn_before_their_labels_with_lets_plot_defaults() {
    let built = build("", "");
    let kinds: Vec<&str> = panel(&built)
        .flattened()
        .into_iter()
        .filter_map(|node| match node {
            Node::Path(_) => Some("path"),
            Node::Text(_) => Some("text"),
            _ => None,
        })
        .collect();
    assert_eq!(kinds, ["path", "path", "text", "text"]);
    let bracket = paths(&built)[0];
    assert_eq!(bracket.points.len(), 4);
    let stroke = bracket.stroke.as_ref().unwrap();
    assert_eq!(stroke.color, Color::rgb(0x47, 0x47, 0x47));
    assert!((stroke.width - 0.75 * 2.2).abs() < 1e-9);
    // Tips: 5 size units of 2.2 px below the bar.
    let [tip, left, right, end] = bracket.points[..] else {
        panic!("four points")
    };
    assert!((tip.1 - left.1 - 11.0).abs() < 1e-9);
    assert!((end.1 - right.1 - 11.0).abs() < 1e-9);
    assert_eq!((tip.0, end.0), (left.0, right.0));
    // The label sits at the middle, its baseline 2.1 px (lets-plot) above the bar.
    let label = texts(&built)[0];
    assert_eq!(label.content, "*");
    assert_eq!(label.size, 14.0);
    assert!((label.x - (left.0 + right.0) / 2.0).abs() < 1e-9);
    assert!(label.y < left.1);
}

#[test]
fn segment_paint_falls_back_to_color_and_alpha() {
    let built = build("", r##", "color": "red", "alpha": 0.5"##);
    let stroke = paths(&built)[0].stroke.clone().unwrap();
    assert_eq!(stroke.color, Color::rgba(255, 0, 0, 128));
    let built = build(
        "",
        r##", "color": "red", "alpha": 0.5, "segment_color": "blue", "segment_alpha": 1.0,
            "segment_size": 2"##,
    );
    let stroke = paths(&built)[0].stroke.clone().unwrap();
    assert_eq!(stroke.color, Color::rgb(0, 0, 255));
    assert!((stroke.width - 4.4).abs() < 1e-9);
    assert_eq!(texts(&built)[0].color, Color::rgba(255, 0, 0, 128));
}

#[test]
fn shorten_pulls_both_ends_to_the_middle() {
    let full = build("", "");
    let short = build("", r#", "bracket_shorten": 0.5"#);
    let width = |path: &Path| path.points[2].0 - path.points[1].0;
    let middle = |path: &Path| (path.points[2].0 + path.points[1].0) / 2.0;
    let (full, short) = (paths(&full)[0], paths(&short)[0]);
    assert!((width(short) - width(full) / 2.0).abs() < 1e-9);
    assert!((middle(short) - middle(full)).abs() < 1e-9);
}

#[test]
fn tip_units_follow_lets_plot() {
    let tip = |options: &str| {
        let built = build("", options);
        let path = paths(&built)[0];
        path.points[0].1 - path.points[1].1
    };
    assert!((tip(r#", "lenstart": 7, "tiplength_unit": "px""#) - 7.0).abs() < 1e-9);
    // Identity: one data unit of y; res: the y resolution (1 between 3 and 4).
    let identity = tip(r#", "lenstart": 1, "tiplength_unit": "identity""#);
    let resolution = tip(r#", "lenstart": 1, "tiplength_unit": "res""#);
    assert!((identity - resolution).abs() < 1e-9);
    let built = build("", "");
    let unit = paths(&built)[1].points[1].1 - paths(&built)[0].points[1].1;
    assert!((identity + unit).abs() < 1e-9, "{identity} vs {unit}");
    // Negative lengths point the tips up.
    assert!(tip(r#", "lenstart": -3"#) < 0.0);
}

#[test]
fn the_plot_mapping_is_not_inherited_unless_asked() {
    // A plot-level colour mapping to a missing column would fail if inherited.
    let built = build(r#", "mapping": {"color": "missing"}"#, "");
    assert_eq!(paths(&built).len(), 2);
    let error = try_build(
        r#", "mapping": {"color": "missing"}"#,
        r#", "inherit_aes": true"#,
    )
    .err()
    .unwrap()
    .to_string();
    assert!(error.contains("missing"), "{error}");
}

#[test]
fn flipped_labels_turn_with_the_coord() {
    let built = build(r#", "coord": {"name": "flip", "flip": true}"#, "");
    let bracket = paths(&built)[0];
    // Bar vertical, tips to the left (towards lower y).
    assert_eq!(bracket.points[1].0, bracket.points[2].0);
    assert!(bracket.points[0].0 < bracket.points[1].0);
    let label = texts(&built)[0];
    assert_eq!(label.angle, -90.0);
    // Lifted away from the tips, towards higher y.
    assert!(label.x > bracket.points[1].0);
}

#[test]
fn unknown_tip_unit_and_vertical_orientation_are_errors() {
    let error = try_build("", r#", "tiplength_unit": "cm""#)
        .err()
        .unwrap()
        .to_string();
    assert!(
        error.contains("tiplength_unit 'cm'") && error.contains("res, identity, size, px"),
        "{error}"
    );
    let error = try_build("", r#", "orientation": "y""#)
        .err()
        .unwrap()
        .to_string();
    assert!(error.contains("not supported by ggrust yet"), "{error}");
}
