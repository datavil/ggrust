//! End-to-end: lets_plot-style spec + Arrow data -> scene -> PNG, for geom_point.

#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::Path as FilePath;
use std::sync::Arc;

use arrow_array::{ArrayRef, Float64Array, RecordBatch, StringArray};
use ggrust_core::build::{BuiltPlot, PlotData, build_plot};
use ggrust_core::data::DataFrame;
use ggrust_core::render::to_png;
use ggrust_core::scene::{Group, Node, Points};
use ggrust_core::spec::{Figure, PlotSpec};
use resvg::tiny_skia::Pixmap;

fn spec(json: &str) -> PlotSpec {
    match Figure::from_json(json).unwrap() {
        Figure::Plot(plot) => *plot,
        Figure::Subplots(_) => panic!("expected a plot"),
    }
}

fn frame(columns: Vec<(&str, ArrayRef)>) -> DataFrame {
    let batch = RecordBatch::try_from_iter(columns).unwrap();
    DataFrame::new(batch.schema(), vec![batch]).unwrap()
}

fn cells() -> DataFrame {
    frame(vec![
        (
            "UMAP1",
            Arc::new(Float64Array::from(vec![1.0, 2.0, 3.0, 4.5])) as ArrayRef,
        ),
        (
            "UMAP2",
            Arc::new(Float64Array::from(vec![2.0, 1.0, 4.0, 3.0])),
        ),
        (
            "cluster",
            Arc::new(StringArray::from(vec!["B", "T", "NK", "T"])),
        ),
    ])
}

fn build(json: &str, data: DataFrame) -> BuiltPlot {
    build_plot(
        &spec(json),
        &PlotData {
            plot: Some(data),
            layers: vec![],
        },
    )
    .unwrap()
}

/// The panel group (role "panel") of a built plot.
fn panel(built: &BuiltPlot) -> &Group {
    built
        .scene
        .nodes
        .iter()
        .find_map(|node| match node {
            Node::Group(group) if group.role == Some("panel") => Some(group),
            _ => None,
        })
        .expect("a panel group")
}

fn points(built: &BuiltPlot) -> &Points {
    panel(built)
        .nodes
        .iter()
        .find_map(|node| match node {
            Node::Points(points) => Some(points),
            _ => None,
        })
        .unwrap()
}

const SCATTER: &str = r#"{"kind": "plot", "mapping": {"x": "UMAP1", "y": "UMAP2"},
    "layers": [{"geom": "point"}]}"#;

#[test]
fn scatter_maps_data_into_the_panel_with_lets_plot_defaults() {
    let built = build(SCATTER, cells());
    assert!(built.warnings.is_empty(), "{:?}", built.warnings);
    assert_eq!((built.scene.width, built.scene.height), (600.0, 400.0));
    let points = points(&built);
    let rect = panel(&built).clip.unwrap();
    // x domain 1..4.5 expanded by 5% on each side.
    let expected_x = rect.x + rect.width * (0.05 / 1.1);
    assert!((points.x[0] - expected_x).abs() < 1e-9, "{}", points.x[0]);
    // y = 4 is the top value: 5% padding below the panel top.
    let expected_y = rect.y + rect.height * (0.05 / 1.1);
    assert!((points.y[2] - expected_y).abs() < 1e-9, "{}", points.y[2]);
    // Diameter 3 x 2.2 = 6.6 px, color #474747 as in lets-plot.
    assert!((points.size.get(0) - 6.6).abs() < 1e-12);
    assert_eq!(points.fill.get(0).unwrap().to_string(), "#474747");
}

#[test]
fn constant_options_and_png_render() {
    let built = build(
        r##"{"kind": "plot", "mapping": {"x": "UMAP1", "y": "UMAP2"},
            "ggsize": {"width": 300, "height": 200},
            "layers": [{"geom": "point", "color": "#e41a1c", "size": 5, "alpha": 0.5}]}"##,
        cells(),
    );
    assert!(built.warnings.is_empty(), "{:?}", built.warnings);
    let points = points(&built);
    assert_eq!(points.size.get(0), 11.0);
    assert_eq!(points.fill.get(0).unwrap().alpha, 128);
    let png = to_png(&built.scene, 2.0).unwrap();
    let output = FilePath::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/visual/output");
    fs::create_dir_all(&output).unwrap();
    fs::write(output.join("build_point_constant.png"), &png).unwrap();
    let pixmap = Pixmap::decode_png(&png).unwrap();
    let centre = pixmap
        // 2 px off the centre: the point sits on grid lines (x = 1, y = 2 are breaks).
        .pixel(
            ((points.x[0] + 2.0) * 2.0) as u32,
            ((points.y[0] + 2.0) * 2.0) as u32,
        )
        .unwrap()
        .demultiply();
    // Half-transparent red (228, 26, 28 at alpha 128) over white, +/- 1 for rounding.
    for (actual, expected) in [
        (centre.red(), 241),
        (centre.green(), 141),
        (centre.blue(), 141),
    ] {
        assert!(actual.abs_diff(expected) <= 1, "{actual} vs {expected}");
    }
}

#[test]
fn unimplemented_features_and_unknown_options_warn() {
    let built = build(
        r#"{"kind": "plot", "mapping": {"x": "UMAP1", "y": "UMAP2", "color": "cluster"},
            "theme": {"name": "classic", "strip_text": {"size": 3, "blank": false}}, "guides": {"color": {"name": "legend", "order": 1}},
            "layers": [{"geom": "point", "colr": "red", "tooltips": {"variables": ["cluster"]}},
                       {"geom": "point", "tooltips": "none"}]}"#,
        cells(),
    );
    let joined = built.warnings.join("\n");
    // `tooltips="none"` asks for what static output already does: no warning.
    assert!(!joined.contains("at 'layers[1]'"), "{joined}");
    for expected in [
        "at 'theme.strip_text': not supported yet; ignored",
        "at 'guides.color': this guide option is not supported yet",
        "at 'layers[0]': 'tooltips' is not supported yet",
        "at 'layers[0].colr': geom_point does not support option 'colr'",
    ] {
        assert!(
            joined.contains(expected),
            "missing {expected:?} in:\n{joined}"
        );
    }
}

#[test]
fn missing_values_are_skipped_and_reported() {
    let data = frame(vec![
        (
            "x",
            Arc::new(Float64Array::from(vec![Some(1.0), None, Some(3.0)])) as ArrayRef,
        ),
        ("y", Arc::new(Float64Array::from(vec![1.0, 2.0, f64::NAN]))),
    ]);
    let json =
        r#"{"kind": "plot", "mapping": {"x": "x", "y": "y"}, "layers": [{"geom": "point"}]}"#;
    let built = build(json, data.clone());
    assert_eq!(built.warnings.len(), 1);
    assert!(built.warnings[0].contains("removed 2 rows containing missing"));
    let silent = build(
        r#"{"kind": "plot", "mapping": {"x": "x", "y": "y"}, "layers": [{"geom": "point", "na_rm": true}]}"#,
        data,
    );
    assert!(silent.warnings.is_empty());
    let svg = ggrust_core::render::to_svg(&built.scene);
    assert_eq!(svg.matches("<circle").count(), 1);
}

#[test]
fn layer_data_overrides_plot_data_and_inherit_aes() {
    let layer_frame = frame(vec![
        (
            "a",
            Arc::new(Float64Array::from(vec![10.0, 20.0])) as ArrayRef,
        ),
        ("b", Arc::new(Float64Array::from(vec![1.0, 2.0]))),
    ]);
    let built = build_plot(
        &spec(
            r#"{"kind": "plot", "mapping": {"x": "UMAP1", "y": "UMAP2"},
                "layers": [{"geom": "point"},
                           {"geom": "point", "inherit_aes": false, "mapping": {"x": "a", "y": "b"}}]}"#,
        ),
        &PlotData {
            plot: Some(cells()),
            layers: vec![None, Some(layer_frame)],
        },
    )
    .unwrap();
    let sizes: Vec<usize> = panel(&built)
        .nodes
        .iter()
        .filter_map(|node| match node {
            Node::Points(points) => Some(points.len()),
            _ => None,
        })
        .collect();
    assert_eq!(sizes, [4, 2]);
}

#[test]
fn errors_name_the_path() {
    let error_of = |json: &str| {
        build_plot(
            &spec(json),
            &PlotData {
                plot: Some(cells()),
                layers: vec![],
            },
        )
        .unwrap_err()
        .to_string()
    };
    let cases = [
        (
            r#"{"kind": "plot", "layers": [{"geom": "pont", "mapping": {"x": "UMAP1", "y": "UMAP2"}}]}"#,
            "at 'layers[0].geom': unknown geom 'pont'; supported: point",
        ),
        (
            r#"{"kind": "plot", "layers": [{"geom": "point", "mapping": {"x": "UMAP1", "y": "UMAP3"}}]}"#,
            "at 'layers[0].mapping.y': column 'UMAP3' not found in data; available columns: UMAP1, UMAP2, cluster",
        ),
        (
            r#"{"kind": "plot", "layers": [{"geom": "point", "mapping": {"x": "UMAP1"}}]}"#,
            "at 'layers[0]': geom_point requires aesthetics x, y (mapped to columns or given as constants); missing: y",
        ),
        (
            r#"{"kind": "plot", "scales": [{"aesthetic": "x", "discrete": true}],
                "layers": [{"geom": "point", "mapping": {"x": "UMAP1", "y": "UMAP2"}}]}"#,
            "at 'layers[0].mapping.x': the x axis is discrete (scale_x_discrete()) but only numeric",
        ),
        (
            r#"{"kind": "plot", "mapping": {"x": "UMAP1", "y": "UMAP2"}, "layers": [{"geom": "point", "size": "big"}]}"#,
            "at 'layers[0].size': expected a finite number",
        ),
        (
            r#"{"kind": "plot", "mapping": {"x": "UMAP1", "y": "UMAP2"}, "layers": [{"geom": "point", "stat": "ydensity"}]}"#,
            "at 'layers[0].stat': stat 'ydensity' is not supported by ggrust yet",
        ),
    ];
    for (json, expected) in cases {
        let error = error_of(json);
        assert!(
            error.contains(expected),
            "expected {expected:?} in {error:?}"
        );
    }
}

#[test]
fn mapping_without_data_is_an_error() {
    let error = build_plot(&spec(SCATTER), &PlotData::default())
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("is mapped but the layer has no data"),
        "{error}"
    );
}
