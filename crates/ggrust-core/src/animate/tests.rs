//! Animations end to end: states computed apart, frames drawing their states with
//! fixed axes and legends, label templates, still frames and spec errors.

use std::sync::Arc;

use arrow_array::{ArrayRef, Float64Array, RecordBatch, StringArray};

use super::*;
use crate::build::build_plot;
use crate::data::DataFrame;
use crate::render::FrameSource;
use crate::scene::{Group, Node};
use crate::spec::Figure;

/// Fruit sold per market stall over three years: `year` (number), `fruit`
/// (category), `sold` and `price`. 2021 has 4 rows, 2022 has 3, 2023 has 5.
fn stalls() -> PlotData {
    let rows: &[(f64, &str, f64, f64)] = &[
        (2021.0, "apple", 10.0, 1.0),
        (2021.0, "pear", 4.0, 1.5),
        (2021.0, "apple", 6.0, 1.2),
        (2021.0, "plum", 3.0, 2.0),
        (2022.0, "apple", 12.0, 1.1),
        (2022.0, "pear", 7.0, 1.6),
        (2022.0, "plum", 5.0, 2.2),
        (2023.0, "apple", 15.0, 1.3),
        (2023.0, "pear", 9.0, 1.4),
        (2023.0, "pear", 2.0, 1.9),
        (2023.0, "plum", 8.0, 2.4),
        (2023.0, "plum", 1.0, 2.5),
    ];
    let columns: Vec<(&str, ArrayRef)> = vec![
        (
            "year",
            Arc::new(Float64Array::from(
                rows.iter().map(|row| row.0).collect::<Vec<_>>(),
            )),
        ),
        (
            "fruit",
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.1).collect::<Vec<_>>(),
            )),
        ),
        (
            "sold",
            Arc::new(Float64Array::from(
                rows.iter().map(|row| row.2).collect::<Vec<_>>(),
            )),
        ),
        (
            "price",
            Arc::new(Float64Array::from(
                rows.iter().map(|row| row.3).collect::<Vec<_>>(),
            )),
        ),
    ];
    let batch = RecordBatch::try_from_iter(columns).unwrap();
    PlotData {
        plot: Some(DataFrame::new(batch.schema(), vec![batch]).unwrap()),
        layers: vec![],
    }
}

fn plot(json: &str) -> PlotSpec {
    match Figure::from_json(json).unwrap() {
        Figure::Plot(plot) => *plot,
        Figure::Subplots(_) => panic!("expected a plot"),
    }
}

/// A point plot of the stalls animated by `transition` (JSON object text).
fn points(transition: &str, extra: &str) -> PlotSpec {
    plot(&format!(
        r#"{{"kind": "plot",
            "mapping": {{"x": "price", "y": "sold", "color": "fruit"}},
            "ggsize": {{"width": 400, "height": 300}},
            "layers": [{{"geom": "point"}}],
            "animation": {{"transition": {transition}}}{extra}}}"#
    ))
}

/// Points drawn in the panels of a scene (legend keys left out).
fn point_count(nodes: &[Node]) -> usize {
    fn count(nodes: &[Node], in_panel: bool) -> usize {
        nodes
            .iter()
            .map(|node| match node {
                Node::Points(points) if in_panel => points.x.len(),
                Node::Group(group) => count(&group.nodes, in_panel || group.role == Some("panel")),
                _ => 0,
            })
            .sum()
    }
    count(nodes, false)
}

/// The scene without the panel's marks.
fn without_marks(nodes: &[Node]) -> Vec<Node> {
    nodes
        .iter()
        .map(|node| match node {
            Node::Group(group) if group.role == Some("panel") => Node::Group(Group {
                nodes: Vec::new(),
                ..group.clone()
            }),
            Node::Group(group) => Node::Group(Group {
                nodes: without_marks(&group.nodes),
                ..group.clone()
            }),
            other => other.clone(),
        })
        .collect()
}

/// Every text of a scene.
fn texts(nodes: &[Node]) -> Vec<String> {
    nodes
        .iter()
        .flat_map(|node| match node {
            Node::Text(text) => vec![text.content.clone()],
            Node::Group(group) => texts(&group.nodes),
            _ => Vec::new(),
        })
        .collect()
}

#[test]
fn manual_frames_draw_one_state_each_with_fixed_axes() {
    let spec = points(r#"{"name": "manual", "frames": "year"}"#, "");
    let animation = Animation::new(&spec, &stalls()).unwrap();
    assert_eq!(animation.timeline().frames, vec![0, 1, 2]);
    assert!((animation.timeline().fps - 0.3).abs() < 1e-12);
    let scenes: Vec<Scene> = (0..3)
        .map(|frame| animation.output_scene(frame).unwrap())
        .collect();
    let counts: Vec<usize> = scenes
        .iter()
        .map(|scene| point_count(&scene.nodes))
        .collect();
    assert_eq!(counts, vec![4, 3, 5]);
    // Axes, grid and legend do not move: the same as the still plot of all rows.
    let still = build_plot(&spec, &stalls()).unwrap();
    for scene in &scenes {
        assert_eq!(
            without_marks(&scene.nodes),
            without_marks(&still.scene.nodes)
        );
    }
    // A still output of an animated plot says so.
    assert!(
        still
            .warnings
            .iter()
            .any(|warning| warning.contains("transition_manual()")),
        "{:?}",
        still.warnings
    );
}

#[test]
fn cumulative_frames_keep_earlier_states() {
    let spec = points(
        r#"{"name": "manual", "frames": "year", "cumulative": true}"#,
        "",
    );
    let animation = Animation::new(&spec, &stalls()).unwrap();
    let counts: Vec<usize> = (0..3)
        .map(|frame| point_count(&animation.output_scene(frame).unwrap().nodes))
        .collect();
    assert_eq!(counts, vec![4, 7, 12]);
}

#[test]
fn label_templates_fill_in_per_frame() {
    let spec = points(
        r#"{"name": "manual", "frames": "year"}"#,
        r#", "ggtitle": {"text": "Year {current_frame}", "subtitle": "frame {frame} of {nframes}, {progress}"},
            "caption": {"text": "next: {next_frame}"}"#,
    );
    let animation = Animation::new(&spec, &stalls()).unwrap();
    let first = texts(&animation.output_scene(0).unwrap().nodes);
    assert!(first.contains(&"Year 2021".to_owned()), "{first:?}");
    assert!(
        first.contains(&"frame 1 of 3, 0.333333333333333".to_owned()),
        "{first:?}"
    );
    assert!(first.contains(&"next: 2022".to_owned()), "{first:?}");
    let last = texts(&animation.output_scene(2).unwrap().nodes);
    assert!(last.contains(&"Year 2023".to_owned()), "{last:?}");
    assert!(last.contains(&"next: ".to_owned()), "{last:?}");

    let spec = points(
        r#"{"name": "manual", "frames": "year"}"#,
        r#", "ggtitle": {"text": "Year {curent_frame}"}"#,
    );
    let error = Animation::new(&spec, &stalls()).err().unwrap().to_string();
    assert!(error.contains("ggtitle.text"), "{error}");
    assert!(error.contains("did you mean 'current_frame'"), "{error}");
}

#[test]
fn stats_and_positions_run_per_state() {
    // Stacked counts per fruit: each year counts its own rows only.
    let spec = plot(
        r#"{"kind": "plot", "mapping": {"x": "fruit"},
            "layers": [{"geom": "bar"}],
            "animation": {"transition": {"name": "manual", "frames": "year"}}}"#,
    );
    let animation = Animation::new(&spec, &stalls()).unwrap();
    let prepared = &animation.prepared;
    let rows = &prepared.layer_frames()[0][0];
    let by_state = prepared.state_rows()[0][0].clone().unwrap();
    let count = |state: usize| -> Vec<f64> {
        let taken = rows.take(&by_state[state]);
        let y = taken.data.continuous(crate::spec::Aesthetic::Y).unwrap();
        y.values().to_vec()
    };
    // 2021: apple 2, pear 1, plum 1; 2022: 1 each; 2023: apple 1, pear 2, plum 2.
    assert_eq!(count(0), vec![2.0, 1.0, 1.0]);
    assert_eq!(count(1), vec![1.0, 1.0, 1.0]);
    assert_eq!(count(2), vec![1.0, 2.0, 2.0]);
}

#[test]
fn layers_without_the_column_draw_in_every_frame() {
    let spec = plot(
        r#"{"kind": "plot", "mapping": {"x": "price", "y": "sold"},
            "layers": [
                {"geom": "point"},
                {"geom": "point", "data": {"frame": 1}, "mapping": {"x": "a", "y": "b"}}
            ],
            "animation": {"transition": {"name": "manual", "frames": "year"}}}"#,
    );
    let mut data = stalls();
    let columns: Vec<(&str, ArrayRef)> = vec![
        ("a", Arc::new(Float64Array::from(vec![1.5, 2.0]))),
        ("b", Arc::new(Float64Array::from(vec![5.0, 6.0]))),
    ];
    let batch = RecordBatch::try_from_iter(columns).unwrap();
    data.layers = vec![
        None,
        Some(DataFrame::new(batch.schema(), vec![batch]).unwrap()),
    ];
    let animation = Animation::new(&spec, &data).unwrap();
    let counts: Vec<usize> = (0..3)
        .map(|frame| point_count(&animation.output_scene(frame).unwrap().nodes))
        .collect();
    assert_eq!(counts, vec![6, 5, 7]);
}

#[test]
fn discrete_states_keep_their_level_order() {
    let spec = points(r#"{"name": "manual", "frames": "fruit"}"#, "");
    let animation = Animation::new(&spec, &stalls()).unwrap();
    assert_eq!(animation.prepared.state_labels(), ["apple", "pear", "plum"]);
    let variables = animation.frame_variables(1);
    assert!(variables.contains(&("current_frame", "pear".to_owned())));
    assert!(variables.contains(&("previous_frame", "apple".to_owned())));
}

#[test]
fn facets_split_each_state() {
    let spec = plot(
        r#"{"kind": "plot", "mapping": {"x": "price", "y": "sold"},
            "layers": [{"geom": "point"}],
            "facet": {"name": "wrap", "facets": "fruit"},
            "animation": {"transition": {"name": "manual", "frames": "year"}}}"#,
    );
    let animation = Animation::new(&spec, &stalls()).unwrap();
    let rows = animation.prepared.state_rows();
    // Panels apple, pear, plum; per year.
    let counts: Vec<Vec<usize>> = rows
        .iter()
        .map(|layers| layers[0].as_ref().unwrap().iter().map(Vec::len).collect())
        .collect();
    assert_eq!(counts, vec![vec![2, 1, 1], vec![1, 1, 2], vec![1, 1, 2]]);
}

#[test]
fn still_frames_draw_the_planned_frame() {
    let spec = points(
        r#"{"name": "manual", "frames": "year"}"#,
        r#", "ggtitle": {"text": "{current_frame}"}"#,
    );
    let mut still = spec.clone();
    still.animation = Some(serde_json::json!({
        "transition": {"name": "manual", "frames": "year"},
        "animate": {"rewind": true},
        "frame": 4
    }));
    // Rewind: 0 1 2 2 1 0; output frame 4 shows scene frame 1 (2022).
    let built = build_plot(&still, &stalls()).unwrap();
    assert_eq!(point_count(&built.scene.nodes), 3);
    assert!(texts(&built.scene.nodes).contains(&"2022".to_owned()));
    still.animation = Some(serde_json::json!({
        "transition": {"name": "manual", "frames": "year"},
        "frame": 3
    }));
    let error = build_plot(&still, &stalls()).err().unwrap().to_string();
    assert!(error.contains("past the last frame"), "{error}");
    drop(spec);
}

#[test]
fn missing_columns_and_grids_are_spec_errors() {
    let spec = points(r#"{"name": "manual", "frames": "yeer"}"#, "");
    let error = Animation::new(&spec, &stalls()).err().unwrap().to_string();
    assert!(error.contains("animation.transition.frames"), "{error}");
    assert!(error.contains("did you mean 'year'"), "{error}");
    let grid = Figure::from_json(
        r#"{"kind": "subplots", "layout": {"name": "grid", "ncol": 1, "nrow": 1},
            "figures": [{"kind": "plot", "layers": [],
              "animation": {"transition": {"name": "manual", "frames": "year"}}}]}"#,
    )
    .unwrap();
    let error = crate::grid::build_figure(&grid, &[])
        .err()
        .unwrap()
        .to_string();
    assert!(error.contains("figures[0].animation"), "{error}");
}

#[test]
fn writers_decode_to_the_frames() {
    let spec = points(r#"{"name": "manual", "frames": "year"}"#, r#""#);
    let animation = Animation::new(&spec, &stalls()).unwrap();
    assert_eq!(FrameSource::timeline(&animation), &[0, 1, 2]);
    let gif = crate::render::to_gif(&animation, 1.0).unwrap();
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = options.read_info(gif.as_slice()).unwrap();
    let mut delays = Vec::new();
    while let Some(frame) = decoder.read_next_frame().unwrap() {
        delays.push(frame.delay);
        assert_eq!((frame.width, frame.height), (400, 300));
    }
    // 0.3 fps: 3.33 s per frame, rounded from the frame start times.
    assert_eq!(delays, vec![333, 334, 333]);
    let html = crate::render::to_html_animation(
        &animation,
        &crate::render::HtmlOptions {
            id: "anim".into(),
            document: crate::render::HtmlDocument::Fragment,
        },
    )
    .unwrap();
    assert!(html.contains("application/vnd.ggrust-animation+json"));
    assert!(html.contains("\"kind\":\"svg\""));
    assert!(html.contains("\"timeline\":[0,1,2]"));
}
