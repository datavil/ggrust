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
        .groups_with_role("panel")
        .into_iter()
        .next()
        .unwrap();
    panel
        .flattened()
        .into_iter()
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
fn merge_plot_data_false_keeps_plot_columns_out() {
    // gghighlight's grey layers: same row count, but no column from the plot data.
    let error = build_plot(
        &spec(
            r#"{"kind": "plot", "mapping": {"x": "a", "y": "b"},
                "layers": [{"geom": "point", "merge_plot_data": false}]}"#,
        ),
        &PlotData {
            plot: Some(frame(&[("a", vec![1.0, 2.0]), ("b", vec![1.0, 2.0])])),
            layers: vec![Some(frame(&[("a", vec![5.0, 6.0])]))],
        },
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("'b'"), "{error}");
}

#[test]
fn constant_positions_draw_points() {
    let built = build(
        r#"{"kind": "plot", "layers": [{"geom": "point", "x": 1.5, "y": 1.5}]}"#,
        Some(frame(&[("a", vec![1.0, 2.0])])),
        vec![],
    );
    // lets-plot drops the data of a layer without mapped columns: one point.
    assert_eq!(points(&built)[0].len(), 1);
    assert!(built.warnings.is_empty(), "{:?}", built.warnings);
    let alone = build(
        r#"{"kind": "plot", "layers": [{"geom": "point", "x": 1.5, "y": 1.5}]}"#,
        None,
        vec![],
    );
    assert_eq!(points(&alone)[0].len(), 1);
}

#[test]
fn constants_win_over_mappings_like_lets_plot() {
    // x = 2.5 replaces the inherited x mapping; y stays mapped (3 rows).
    let built = build(
        r#"{"kind": "plot", "mapping": {"x": "a", "y": "b"},
            "layers": [{"geom": "point", "x": 2.5}]}"#,
        Some(frame(&[
            ("a", vec![1.0, 2.0, 3.0]),
            ("b", vec![1.0, 2.0, 3.0]),
        ])),
        vec![],
    );
    let layer_points = &points(&built)[0];
    assert_eq!(layer_points.len(), 3);
    assert!(
        layer_points
            .x
            .iter()
            .all(|x| (x - layer_points.x[0]).abs() < 1e-9)
    );
    // A constant color means the mapped column is never looked up (no error).
    let built = build(
        r#"{"kind": "plot", "mapping": {"x": "a", "y": "b", "color": "missing"},
            "layers": [{"geom": "point", "color": "red"}]}"#,
        Some(frame(&[("a", vec![1.0, 2.0]), ("b", vec![1.0, 2.0])])),
        vec![],
    );
    assert_eq!(points(&built)[0].len(), 2);
    assert!(built.warnings.is_empty(), "{:?}", built.warnings);
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
        r#"{"kind": "plot", "mapping": {"x": "a", "y": "b", "linetype": "a", "group": "a"},
            "layers": [{"geom": "point"}]}"#,
        Some(frame(&[("a", vec![1.0]), ("b", vec![1.0])])),
        vec![],
    );
    let joined = built.warnings.join("\n");
    assert!(
        joined.contains("at 'mapping.linetype': geom_point does not use aesthetic 'linetype'"),
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
fn list_mappings_are_data_vectors_with_a_readable_length_error() {
    // lets-plot reads a list (except for `group`) as data, so two values cannot
    // colour one row; lets-plot draws nothing, ggrust says why.
    let error = build_plot(
        &spec(
            r#"{"kind": "plot", "mapping": {"x": "a", "y": "b", "color": ["a", "b"]},
                "layers": [{"geom": "point"}]}"#,
        ),
        &PlotData {
            plot: Some(frame(&[("a", vec![1.0]), ("b", vec![1.0])])),
            layers: vec![],
        },
    )
    .err()
    .unwrap()
    .to_string();
    assert_eq!(
        error,
        "invalid plot specification at 'mapping.color': the list has 2 values but the data has 1 rows"
    );
}

fn axis_titles(built: &BuiltPlot) -> Vec<String> {
    let titles: &Group = built
        .scene
        .nodes
        .iter()
        .find_map(|node| match node {
            Node::Group(group) if group.role == Some("titles") => Some(group),
            _ => None,
        })
        .unwrap();
    titles
        .nodes
        .iter()
        .filter_map(|node| match node {
            Node::Text(text) => Some(text.content.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn constant_positions_get_aesthetic_titles() {
    // Bug hunt 0.6: lets-plot titles the axes "x" and "y".
    let built = build(
        r#"{"kind": "plot", "layers": [{"geom": "point", "x": 1, "y": 2}]}"#,
        None,
        vec![],
    );
    assert_eq!(axis_titles(&built), ["x", "y"]);
}

#[test]
fn axis_title_follows_lets_plot_default_scale_name() {
    // Columns mapped across layers, ignoring one named like the aesthetic.
    let data = frame(&[("x", vec![1.0]), ("y", vec![2.0])]);
    let built = build(
        r#"{"kind": "plot", "layers": [
            {"geom": "point", "mapping": {"x": "x", "y": "y"}},
            {"geom": "point", "mapping": {"x": "y", "y": "x"}}]}"#,
        Some(data.clone()),
        vec![],
    );
    assert_eq!(axis_titles(&built), ["y", "x"]);
    let data = frame(&[("a", vec![1.0]), ("b", vec![2.0])]);
    let several = build(
        r#"{"kind": "plot", "layers": [
            {"geom": "point", "mapping": {"x": "a", "y": "b"}},
            {"geom": "point", "mapping": {"x": "b", "y": "a"}}]}"#,
        Some(data),
        vec![],
    );
    assert_eq!(axis_titles(&several), ["x", "y"]);
}

#[test]
fn the_panel_never_shrinks_below_lets_plots_minimum() {
    // Bug hunt 0.10.2: 25 long vertical x labels left a 27 px panel at 600 x 400.
    let labels: Vec<String> = (0..25)
        .map(|index| format!("GO:{index:07} regulation of something biological process {index}"))
        .collect();
    let batch = RecordBatch::try_from_iter([
        (
            "term",
            Arc::new(arrow_array::StringArray::from(labels.clone())) as ArrayRef,
        ),
        (
            "score",
            Arc::new(Float64Array::from(
                (0..25).map(f64::from).collect::<Vec<_>>(),
            )),
        ),
    ])
    .unwrap();
    let data = DataFrame::new(batch.schema(), vec![batch]).unwrap();
    let built = build(
        r#"{"kind": "plot", "mapping": {"x": "term", "y": "score"}, "layers": [{"geom": "point"}]}"#,
        Some(data),
        vec![],
    );
    let panel = built
        .scene
        .groups_with_role("panel")
        .into_iter()
        .find_map(|group| group.clip)
        .unwrap();
    assert!(panel.height >= 50.0 && panel.width >= 50.0, "{panel:?}");
}

#[test]
fn unknown_lets_plot_names_say_so() {
    let data = || PlotData {
        plot: Some(frame(&[("a", vec![1.0]), ("b", vec![1.0])])),
        layers: vec![],
    };
    let error = |layer: &str| {
        let json = format!(
            r#"{{"kind": "plot", "mapping": {{"x": "a", "y": "b"}}, "layers": [{layer}]}}"#
        );
        match build_plot(&spec(&json), &data()) {
            Ok(_) => panic!("{layer} built"),
            Err(error) => error.to_string(),
        }
    };
    let typo = error(r#"{"geom": "violn"}"#);
    assert!(typo.contains("unknown geom 'violn'"), "{typo}");
}

/// Lengths of the two-point paths in the panel: (horizontal, vertical).
fn segment_lengths(built: &BuiltPlot) -> (Vec<f64>, Vec<f64>) {
    let panel: &Group = built
        .scene
        .groups_with_role("panel")
        .into_iter()
        .next()
        .unwrap();
    let (mut horizontal, mut vertical) = (Vec::new(), Vec::new());
    for node in panel.flattened() {
        if let Node::Path(path) = node
            && let [(x0, y0), (x1, y1)] = path.points[..]
        {
            match y0 == y1 {
                true => horizontal.push((x1 - x0).abs()),
                false => vertical.push((y1 - y0).abs()),
            }
        }
    }
    (horizontal, vertical)
}

fn interval_frame() -> DataFrame {
    frame(&[
        ("b", vec![1.0, 2.0, 3.0]),
        ("lo", vec![1.0, 2.0, 1.5]),
        ("hi", vec![3.0, 4.0, 3.5]),
    ])
}

#[test]
fn x_ranges_make_error_bars_horizontal() {
    for orientation in ["", r#""orientation": "y","#] {
        let built = build(
            &format!(
                r#"{{"kind": "plot", "layers": [{{"geom": "errorbar", {orientation}
                "mapping": {{"y": "b", "xmin": "lo", "xmax": "hi"}}}}]}}"#
            ),
            Some(interval_frame()),
            vec![],
        );
        let (horizontal, vertical) = segment_lengths(&built);
        // One range per row along x, two fences per row across it.
        assert_eq!((horizontal.len(), vertical.len()), (3, 6), "{orientation}");
        assert!(built.warnings.is_empty(), "{:?}", built.warnings);
    }
}

#[test]
fn y_ranges_win_over_x_ranges() {
    let built = build(
        r#"{"kind": "plot", "layers": [{"geom": "linerange",
            "mapping": {"x": "b", "ymin": "lo", "ymax": "hi", "xmin": "lo"}}]}"#,
        Some(interval_frame()),
        vec![],
    );
    let (horizontal, vertical) = segment_lengths(&built);
    assert_eq!((horizontal.len(), vertical.len()), (0, 3));
}

#[test]
fn a_constant_x_range_makes_the_layer_horizontal() {
    let built = build(
        r#"{"kind": "plot", "layers": [{"geom": "linerange", "xmin": 0.5,
            "mapping": {"y": "b", "xmax": "hi"}}]}"#,
        Some(interval_frame()),
        vec![],
    );
    let (horizontal, vertical) = segment_lengths(&built);
    assert_eq!((horizontal.len(), vertical.len()), (3, 0));
    assert!(built.warnings.is_empty(), "{:?}", built.warnings);
}

#[test]
fn a_missing_interval_aesthetic_names_both_orientations() {
    let error = build_plot(
        &spec(r#"{"kind": "plot", "layers": [{"geom": "errorbar", "mapping": {"x": "b"}}]}"#),
        &PlotData {
            plot: Some(interval_frame()),
            layers: vec![],
        },
    )
    .err()
    .unwrap()
    .to_string();
    assert!(error.contains("missing: ymin, ymax"), "{error}");
    assert!(error.contains("y, xmin and xmax"), "{error}");
}

#[test]
fn orientation_y_with_vertical_mapping_names_what_it_needs() {
    let error = build_plot(
        &spec(
            r#"{"kind": "plot", "layers": [{"geom": "errorbar", "orientation": "y",
                "mapping": {"x": "b", "ymin": "lo", "ymax": "hi"}}]}"#,
        ),
        &PlotData {
            plot: Some(interval_frame()),
            layers: vec![],
        },
    )
    .err()
    .unwrap()
    .to_string();
    assert!(error.contains("with orientation='y'"), "{error}");
    assert!(error.contains("missing: y, xmin, xmax"), "{error}");
}

fn colorbar_warnings(scale: &str, value: &[f64]) -> Vec<String> {
    let json = format!(
        r#"{{"kind": "plot", "mapping": {{"x": "b", "y": "b", "color": "v"}},
            "layers": [{{"geom": "point"}}], "scales": [{scale}]}}"#
    );
    let built = build(
        &json,
        Some(frame(&[("b", vec![1.0, 2.0, 3.0]), ("v", value.to_vec())])),
        vec![],
    );
    built.warnings
}

#[test]
fn lablim_on_a_continuous_color_scale_warns_with_its_path() {
    let warnings = colorbar_warnings(
        r#"{"aesthetic": "color", "lablim": 2}"#,
        &[1000.0, 2000.0, 3000.0],
    );
    let [warning] = &warnings[..] else {
        panic!("{warnings:?}");
    };
    assert!(warning.contains("at 'scales[0].lablim'"), "{warning}");
    assert!(warning.contains("only cuts discrete labels"), "{warning}");
    assert!(warning.contains("format="), "{warning}");
}

#[test]
fn lablim_zero_means_no_limit_and_does_not_warn() {
    let warnings = colorbar_warnings(
        r#"{"aesthetic": "color", "lablim": 0}"#,
        &[1000.0, 2000.0, 3000.0],
    );
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn flavors_leave_scale_colors_alone() {
    let fills = |theme: &str| {
        let built = build(
            &format!(
                r#"{{"kind": "plot", "mapping": {{"x": "b", "y": "b", "color": "b"}},
                    "layers": [{{"geom": "point"}}]{theme}}}"#
            ),
            Some(frame(&[("b", vec![1.0, 2.0, 3.0])])),
            vec![],
        );
        let points = points(&built)[0];
        format!("{:?} {:?}", points.fill, points.stroke)
    };
    let standard = fills("");
    for flavor in [
        "darcula",
        "solarized_light",
        "solarized_dark",
        "high_contrast_light",
        "high_contrast_dark",
    ] {
        let theme = format!(r#", "theme": {{"flavor": "{flavor}"}}"#);
        assert_eq!(fills(&theme), standard, "{flavor}");
    }
}
