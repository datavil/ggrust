//! Regressions found by the Phase 0.5 bug hunt (pipeline semantics vs lets-plot).

#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use arrow_array::{ArrayRef, Float64Array, RecordBatch};
use ggrust_core::build::{BuiltPlot, PlotData, build_plot};
use ggrust_core::data::DataFrame;
use ggrust_core::render::to_svg;
use ggrust_core::scene::{Group, Node, Points};
use ggrust_core::spec::{Figure, PlotSpec};

fn spec(json: &str) -> PlotSpec {
    match Figure::from_json(json).unwrap() {
        Figure::Plot(plot) => *plot,
        Figure::Subplots(_) => panic!("expected a plot"),
    }
}

fn frame(columns: &[(&str, Vec<f64>)]) -> DataFrame {
    let batch = RecordBatch::try_from_iter(columns.iter().map(|(name, values)| {
        (
            *name,
            Arc::new(Float64Array::from(values.clone())) as ArrayRef,
        )
    }))
    .unwrap();
    DataFrame::new(batch.schema(), vec![batch]).unwrap()
}

fn build(json: &str, plot: Option<DataFrame>, layers: Vec<Option<DataFrame>>) -> BuiltPlot {
    build_plot(&spec(json), &PlotData { plot, layers }).unwrap()
}

fn points(built: &BuiltPlot) -> Vec<&Points> {
    let panel: &Group = built
        .scene
        .nodes
        .iter()
        .find_map(|node| match node {
            Node::Group(group) if group.role == Some("panel") => Some(group),
            _ => None,
        })
        .unwrap();
    panel
        .nodes
        .iter()
        .filter_map(|node| match node {
            Node::Points(points) => Some(points),
            _ => None,
        })
        .collect()
}

#[test]
fn point_svg_is_well_formed_xml() {
    let built = build(
        r#"{"kind": "plot", "mapping": {"x": "a", "y": "b"}, "layers": [{"geom": "point"}]}"#,
        Some(frame(&[("a", vec![1.0, 2.0]), ("b", vec![1.0, 2.0])])),
        vec![],
    );
    let svg = to_svg(&built.scene);
    roxmltree::Document::parse(&svg).unwrap_or_else(|error| panic!("{error}\n{svg}"));
}

#[test]
fn alpha_replaces_the_color_alpha() {
    let alpha_of = |options: &str| {
        let built = build(
            &format!(
                r#"{{"kind": "plot", "mapping": {{"x": "a", "y": "b"}},
                    "layers": [{{"geom": "point", {options}}}]}}"#
            ),
            Some(frame(&[("a", vec![1.0]), ("b", vec![1.0])])),
            vec![],
        );
        points(&built)[0].fill.get(0).unwrap().alpha
    };
    assert_eq!(alpha_of(r##""color": "#ff000080", "alpha": 1"##), 255);
    assert_eq!(alpha_of(r#""color": "red / 0.5", "alpha": 0.5"#), 128);
    assert_eq!(alpha_of(r##""color": "#ff000080""##), 128);
}

#[test]
fn plot_columns_fill_in_for_layer_data_with_the_same_rows() {
    let built = build(
        r#"{"kind": "plot", "mapping": {"x": "a", "y": "b"}, "layers": [{"geom": "point"}]}"#,
        Some(frame(&[("a", vec![1.0, 2.0]), ("b", vec![1.0, 2.0])])),
        vec![Some(frame(&[("a", vec![5.0, 6.0])]))],
    );
    let layer_points = &points(&built)[0];
    assert_eq!(layer_points.len(), 2);
    // x comes from the layer data (5, 6), y from the plot data.
    assert!(layer_points.x[0] < layer_points.x[1]);
}

#[test]
fn plot_columns_are_not_mixed_in_when_rows_differ() {
    let error = build_plot(
        &spec(
            r#"{"kind": "plot", "mapping": {"x": "a", "y": "b"}, "layers": [{"geom": "point"}]}"#,
        ),
        &PlotData {
            plot: Some(frame(&[("a", vec![1.0, 2.0]), ("b", vec![1.0, 2.0])])),
            layers: vec![Some(frame(&[("a", vec![5.0, 6.0, 7.0])]))],
        },
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("the plot data has it, but its 2 rows differ"),
        "{error}"
    );
}

#[test]
fn constant_positions_draw_points() {
    let built = build(
        r#"{"kind": "plot", "layers": [{"geom": "point", "x": 1.5, "y": 1.5}]}"#,
        Some(frame(&[("a", vec![1.0, 2.0])])),
        vec![],
    );
    assert_eq!(points(&built)[0].len(), 2);
    assert!(built.warnings.is_empty(), "{:?}", built.warnings);
    let alone = build(
        r#"{"kind": "plot", "layers": [{"geom": "point", "x": 1.5, "y": 1.5}]}"#,
        None,
        vec![],
    );
    assert_eq!(points(&alone)[0].len(), 1);
}

#[test]
fn tiny_ranges_spread_points() {
    let built = build(
        r#"{"kind": "plot", "mapping": {"x": "a", "y": "b"}, "layers": [{"geom": "point"}]}"#,
        Some(frame(&[("a", vec![1e-20, 3e-20]), ("b", vec![1.0, 1.0])])),
        vec![],
    );
    let layer_points = &points(&built)[0];
    assert!(layer_points.x[1] - layer_points.x[0] > 100.0);
}

#[test]
fn unknown_position_options_warn() {
    let built = build(
        r#"{"kind": "plot", "mapping": {"x": "a", "y": "b"},
            "layers": [{"geom": "point", "position": {"name": "identity", "width": 3}}]}"#,
        Some(frame(&[("a", vec![1.0]), ("b", vec![1.0])])),
        vec![],
    );
    assert!(
        built.warnings.iter().any(|warning| warning
            .contains("at 'layers[0].position.width': position_identity does not support")),
        "{:?}",
        built.warnings
    );
}

#[test]
fn unused_mapped_aesthetics_warn() {
    let built = build(
        r#"{"kind": "plot", "mapping": {"x": "a", "y": "b", "shape": "a", "group": "a"},
            "layers": [{"geom": "point"}]}"#,
        Some(frame(&[("a", vec![1.0]), ("b", vec![1.0])])),
        vec![],
    );
    let joined = built.warnings.join("\n");
    assert!(
        joined.contains("at 'mapping.shape': geom_point does not use aesthetic 'shape'"),
        "{joined}"
    );
    assert!(!joined.contains("'group'"), "{joined}");
}

#[test]
fn colour_option_is_an_alias() {
    let built = build(
        r#"{"kind": "plot", "mapping": {"x": "a", "y": "b"},
            "layers": [{"geom": "point", "colour": "red"}]}"#,
        Some(frame(&[("a", vec![1.0]), ("b", vec![1.0])])),
        vec![],
    );
    assert!(built.warnings.is_empty(), "{:?}", built.warnings);
    assert_eq!(
        points(&built)[0].fill.get(0).unwrap().to_string(),
        "#ff0000"
    );
}

#[test]
fn list_mappings_warn_once_without_debug_output() {
    let built = build(
        r#"{"kind": "plot", "mapping": {"x": "a", "y": "b", "group": ["a", "b"]},
            "layers": [{"geom": "point"}]}"#,
        Some(frame(&[("a", vec![1.0]), ("b", vec![1.0])])),
        vec![],
    );
    let relevant: Vec<_> = built
        .warnings
        .iter()
        .filter(|warning| warning.contains("group"))
        .collect();
    assert_eq!(relevant.len(), 1, "{relevant:?}");
    assert!(relevant[0].contains("columns [a, b]"), "{relevant:?}");
}
