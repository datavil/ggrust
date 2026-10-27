//! End-to-end: spec + Arrow data -> scene, for the Phase 1.6 geoms (path, line,
//! segment, polygon, vline, hline, text, label, blank).

#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use arrow_array::{ArrayRef, Float64Array, RecordBatch, StringArray};
use ggrust_core::build::{BuiltPlot, PlotData, build_plot};
use ggrust_core::data::DataFrame;
use ggrust_core::scene::{Group, Node, Path, Text};
use ggrust_core::spec::{Figure, PlotSpec};

fn spec(json: &str) -> PlotSpec {
    match Figure::from_json(json).unwrap() {
        Figure::Plot(plot) => *plot,
        Figure::Subplots(_) => panic!("expected a plot"),
    }
}

fn frame(x: Vec<f64>, y: Vec<f64>, group: Vec<&str>) -> DataFrame {
    let batch = RecordBatch::try_from_iter(vec![
        ("x", Arc::new(Float64Array::from(x)) as ArrayRef),
        ("y", Arc::new(Float64Array::from(y)) as ArrayRef),
        ("g", Arc::new(StringArray::from(group)) as ArrayRef),
    ])
    .unwrap();
    DataFrame::new(batch.schema(), vec![batch]).unwrap()
}

fn build(layers: &str, data: DataFrame) -> BuiltPlot {
    let json =
        format!(r#"{{"kind": "plot", "mapping": {{"x": "x", "y": "y"}}, "layers": [{layers}]}}"#);
    build_plot(
        &spec(&json),
        &PlotData {
            plot: Some(data),
            layers: vec![],
        },
    )
    .unwrap()
}

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

fn paths(built: &BuiltPlot) -> Vec<&Path> {
    panel(built)
        .nodes
        .iter()
        .filter_map(|node| match node {
            Node::Path(path) => Some(path),
            _ => None,
        })
        .collect()
}

fn texts(built: &BuiltPlot) -> Vec<&Text> {
    panel(built)
        .nodes
        .iter()
        .filter_map(|node| match node {
            Node::Text(text) => Some(text),
            _ => None,
        })
        .collect()
}

fn simple() -> DataFrame {
    frame(
        vec![3.0, 1.0, 2.0],
        vec![3.0, 1.0, 2.0],
        vec!["a", "a", "b"],
    )
}

#[test]
fn path_keeps_data_order_and_line_sorts_by_x() {
    let path = build(
        r#"{"geom": "path"}"#,
        frame(vec![3.0, 1.0, 2.0], vec![0.0; 3], vec!["a"; 3]),
    );
    let line = build(
        r#"{"geom": "line"}"#,
        frame(vec![3.0, 1.0, 2.0], vec![0.0; 3], vec!["a"; 3]),
    );
    let path_x: Vec<f64> = paths(&path)[0].points.iter().map(|point| point.0).collect();
    let line_x: Vec<f64> = paths(&line)[0].points.iter().map(|point| point.0).collect();
    assert!(path_x[0] > path_x[1] && path_x[1] < path_x[2], "{path_x:?}");
    assert!(
        line_x.windows(2).all(|pair| pair[0] < pair[1]),
        "{line_x:?}"
    );
}

#[test]
fn path_defaults_follow_lets_plot_and_groups_split() {
    let built = build(r#"{"geom": "path"}"#, simple());
    // Default grouping only splits on discrete style mappings: one path here.
    let all = paths(&built);
    assert_eq!(all.len(), 1);
    let stroke = all[0].stroke.as_ref().unwrap();
    assert!((stroke.width - 1.65).abs() < 1e-9);
    assert_eq!(stroke.color.to_string(), "#474747");
    let grouped = build(
        r#"{"geom": "path", "mapping": {"color": "g"}}"#,
        frame(
            vec![1.0, 2.0, 3.0, 4.0],
            vec![1.0, 2.0, 3.0, 4.0],
            vec!["a", "a", "b", "b"],
        ),
    );
    assert_eq!(paths(&grouped).len(), 2);
}

#[test]
fn missing_values_break_the_line() {
    let built = build(
        r#"{"geom": "path"}"#,
        frame(
            vec![1.0, 2.0, f64::NAN, 4.0, 5.0],
            vec![1.0; 5],
            vec!["a"; 5],
        ),
    );
    assert_eq!(paths(&built).len(), 2);
}

#[test]
fn dashes_scale_with_line_width() {
    let built = build(
        r#"{"geom": "path", "linetype": "dashed", "size": 2}"#,
        simple(),
    );
    let stroke = paths(&built)[0].stroke.clone().unwrap();
    assert_eq!(stroke.dash, vec![4.0 * 4.4, 4.0 * 4.4]);
}

#[test]
fn segment_arrow_draws_a_head_and_pulls_the_tip_back() {
    let plain = build(
        r#"{"geom": "segment", "xend": 3, "yend": 3, "x": 1, "y": 1}"#,
        simple(),
    );
    assert_eq!(paths(&plain).len(), 1);
    let arrow = build(
        r#"{"geom": "segment", "xend": 3, "yend": 3, "x": 1, "y": 1,
            "arrow": {"name": "arrow", "angle": 30, "length": 10, "ends": "both", "type": "closed"}}"#,
        simple(),
    );
    let all = paths(&arrow);
    // The line plus a head at each end.
    assert_eq!(all.len(), 3);
    assert!(all[1].closed && all[1].fill.is_some());
    // Barbs are 10 px from the tip.
    let head = &all[1].points;
    let tip = head[1];
    for barb in [head[0], head[2]] {
        assert!(((barb.0 - tip.0).hypot(barb.1 - tip.1) - 10.0).abs() < 1e-9);
    }
    // The line is shorter than the plain one by the line width at each end.
    let length = |path: &Path| {
        let (a, b) = (path.points[0], path.points[1]);
        (a.0 - b.0).hypot(a.1 - b.1)
    };
    assert!((length(paths(&plain)[0]) - length(all[0]) - 2.0 * 1.65).abs() < 1e-9);
}

#[test]
fn invalid_arrow_options_name_the_path() {
    let json = r#"{"kind": "plot", "mapping": {"x": "x", "y": "y"},
        "layers": [{"geom": "segment", "x": 1, "y": 1, "xend": 2, "yend": 2,
        "arrow": {"name": "arrow", "ends": "middle"}}]}"#;
    let error = build_plot(
        &spec(json),
        &PlotData {
            plot: Some(simple()),
            layers: vec![],
        },
    )
    .map(|_| ())
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("layers[0].arrow") && error.contains("ends"),
        "{error}"
    );
}

#[test]
fn reference_lines_span_the_panel() {
    let built = build(
        r#"{"geom": "vline", "xintercept": 2}, {"geom": "hline", "yintercept": 2}"#,
        simple(),
    );
    assert!(built.warnings.is_empty(), "{:?}", built.warnings);
    let rect = panel(&built).clip.unwrap();
    let all = paths(&built);
    assert_eq!(all.len(), 2);
    let (vertical, horizontal) = (&all[0].points, &all[1].points);
    assert_eq!(vertical[0].0, vertical[1].0);
    assert_eq!(vertical[0].1, rect.y + rect.height);
    assert_eq!(vertical[1].1, rect.y);
    assert_eq!(horizontal[0].1, horizontal[1].1);
    assert_eq!(horizontal[0].0, rect.x);
    assert_eq!(horizontal[1].0, rect.x + rect.width);
}

#[test]
fn intercepts_train_the_scales() {
    let with = build(
        r#"{"geom": "point"}, {"geom": "vline", "xintercept": 10}"#,
        simple(),
    );
    let without = build(r#"{"geom": "point"}"#, simple());
    let first = |built: &BuiltPlot| {
        built
            .scene
            .nodes
            .iter()
            .filter_map(|node| match node {
                Node::Group(group) if group.role == Some("panel") => {
                    group.nodes.iter().find_map(|node| match node {
                        Node::Points(points) => Some(points.x[1]),
                        _ => None,
                    })
                }
                _ => None,
            })
            .next()
            .unwrap()
    };
    // x = 1 sits further left once the domain reaches 10.
    assert!(first(&with) < first(&without) + 1.0);
    assert_ne!(first(&with), first(&without));
}

#[test]
fn polygon_is_closed_with_lets_plot_colors_and_alpha_on_the_fill_only() {
    let built = build(
        r#"{"geom": "polygon", "alpha": 0.5}"#,
        frame(vec![1.0, 2.0, 3.0], vec![1.0, 3.0, 1.0], vec!["a"; 3]),
    );
    let polygon = paths(&built)[0];
    assert!(polygon.closed);
    assert_eq!(
        polygon.fill.unwrap().to_string().to_lowercase()[..7].to_owned(),
        "#118ed8"
    );
    assert_eq!(polygon.fill.unwrap().alpha, 128);
    assert_eq!(polygon.stroke.as_ref().unwrap().color.alpha, 255);
    assert_eq!(
        polygon.stroke.as_ref().unwrap().color.to_string(),
        "#ffffff"
    );
}

#[test]
fn text_reads_labels_from_a_column_and_sizes_like_lets_plot() {
    let built = build(
        r#"{"geom": "text", "mapping": {"label": "g"}, "nudge_y": 1}"#,
        simple(),
    );
    assert!(built.warnings.is_empty(), "{:?}", built.warnings);
    let all = texts(&built);
    assert_eq!(
        all.iter()
            .map(|text| text.content.as_str())
            .collect::<Vec<_>>(),
        ["a", "a", "b"]
    );
    assert!((all[0].size - 14.0).abs() < 1e-9);
    let plain = build(r#"{"geom": "text", "label": "hi"}"#, simple());
    assert_eq!(texts(&plain)[0].content, "hi");
}

#[test]
fn numeric_labels_drop_the_trailing_zero() {
    let built = build(r#"{"geom": "text", "mapping": {"label": "y"}}"#, simple());
    assert_eq!(texts(&built)[0].content, "3");
}

#[test]
fn label_draws_a_box_behind_the_text() {
    let built = build(
        r#"{"geom": "label", "label": "four", "size": 6}"#,
        frame(vec![1.0], vec![1.0], vec!["a"]),
    );
    let boxes = paths(&built);
    assert_eq!(boxes.len(), 1);
    let ys: Vec<f64> = boxes[0].points.iter().map(|point| point.1).collect();
    let height =
        ys.iter().cloned().fold(f64::MIN, f64::max) - ys.iter().cloned().fold(f64::MAX, f64::min);
    // 12 px font + 2 x 0.25 x 12 padding.
    assert!((height - 18.0).abs() < 1e-9, "{height}");
    assert_eq!(texts(&built).len(), 1);
}

#[test]
fn hjust_moves_the_text_box_right_of_the_point() {
    let at = |hjust: f64| {
        let built = build(
            &format!(r#"{{"geom": "text", "label": "word", "hjust": {hjust}}}"#),
            frame(vec![1.0], vec![1.0], vec!["a"]),
        );
        texts(&built)[0].x
    };
    assert!(at(0.0) > at(0.5) && at(0.5) > at(1.0));
}

#[test]
fn blank_draws_nothing_but_trains_the_scales() {
    let built = build(
        r#"{"geom": "blank", "mapping": {"x": "x", "y": "y"}}"#,
        frame(vec![0.0, 10.0], vec![0.0, 10.0], vec!["a", "a"]),
    );
    assert!(built.warnings.is_empty(), "{:?}", built.warnings);
    assert!(panel(&built).nodes.is_empty());
    // The axis shows the data range, so tick labels exist.
    let has_text = built
        .scene
        .nodes
        .iter()
        .any(|node| matches!(node, Node::Text(_) | Node::Group(_)));
    assert!(has_text);
}

#[test]
fn arrow_on_a_line_with_varying_style_has_heads_only_at_the_ends() {
    let built = build(
        r#"{"geom": "path", "mapping": {"size": "y"},
            "arrow": {"name": "arrow", "ends": "both"}}"#,
        frame(
            vec![1.0, 2.0, 3.0, 4.0],
            vec![1.0, 1.0, 5.0, 5.0],
            vec!["a"; 4],
        ),
    );
    // Two styled pieces (size 1, size 5) plus one head per outer end; heads at
    // the style change would add two more.
    assert_eq!(paths(&built).len(), 4);
}

#[test]
fn blank_with_a_color_mapping_keeps_its_legend() {
    let built = build(r#"{"geom": "blank", "mapping": {"color": "g"}}"#, simple());
    assert!(built.warnings.is_empty(), "{:?}", built.warnings);
    let legend = built
        .scene
        .nodes
        .iter()
        .any(|node| matches!(node, Node::Group(group) if group.role == Some("legend")));
    assert!(legend);
}

#[test]
fn vjust_moves_the_baseline_by_lets_plots_span() {
    let at = |vjust: f64| {
        let built = build(
            &format!(r#"{{"geom": "text", "label": "word", "size": 20, "vjust": {vjust}}}"#),
            frame(vec![1.0], vec![1.0], vec!["a"]),
        );
        texts(&built)[0].y
    };
    // 0.7 em of a 40 px font between vjust 0 and 1 (lets-plot), text lower for 1.
    assert!((at(1.0) - at(0.0) - 28.0).abs() < 1e-9);
}

#[test]
fn bold_italic_with_a_space_is_read() {
    let built = build(
        r#"{"geom": "text", "label": "w", "fontface": "Bold Italic"}"#,
        frame(vec![1.0], vec![1.0], vec!["a"]),
    );
    assert_eq!(
        texts(&built)[0].font.face,
        ggrust_core::text::FontFace::BoldItalic
    );
}

#[test]
fn label_vjust_one_hangs_the_box_from_the_point() {
    let built = build(
        r#"{"geom": "label", "label": "w", "vjust": 1}, {"geom": "point"}"#,
        frame(vec![1.0], vec![1.0], vec!["a"]),
    );
    let top = paths(&built)[0]
        .points
        .iter()
        .map(|point| point.1)
        .fold(f64::MAX, f64::min);
    let point_y = panel(&built)
        .nodes
        .iter()
        .find_map(|node| match node {
            Node::Points(points) => Some(points.y[0]),
            _ => None,
        })
        .unwrap();
    assert!((top - point_y).abs() < 1e-9, "{top} vs {point_y}");
}
