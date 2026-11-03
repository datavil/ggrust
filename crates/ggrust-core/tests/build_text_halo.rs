//! End-to-end: text halos (`halo_width`, `halo_color`) on geom_text and
//! geom_text_repel, measured against lets-plot 4.11 `to_svg()`.

#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use arrow_array::{ArrayRef, Float64Array, RecordBatch, StringArray};
use ggrust_core::build::{BuiltPlot, PlotData, build_plot};
use ggrust_core::data::DataFrame;
use ggrust_core::scene::{Color, Node, Text};
use ggrust_core::spec::{Figure, PlotSpec};

fn build(layers: &str, theme: &str) -> BuiltPlot {
    let json = format!(
        r#"{{"kind": "plot", "mapping": {{"x": "x", "y": "y", "label": "l"}},
            "layers": [{layers}]{theme}}}"#
    );
    let Figure::Plot(plot) = Figure::from_json(&json).unwrap() else {
        panic!("expected a plot")
    };
    let spec: PlotSpec = *plot;
    let batch = RecordBatch::try_from_iter(vec![
        (
            "x",
            Arc::new(Float64Array::from(vec![1.0, 2.0])) as ArrayRef,
        ),
        (
            "y",
            Arc::new(Float64Array::from(vec![1.0, 2.0])) as ArrayRef,
        ),
        (
            "l",
            Arc::new(StringArray::from(vec!["Ab", "C\nd"])) as ArrayRef,
        ),
    ])
    .unwrap();
    let data = DataFrame::new(batch.schema(), vec![batch]).unwrap();
    build_plot(
        &spec,
        &PlotData {
            plot: Some(data),
            layers: vec![],
        },
    )
    .unwrap()
}

/// The panel's texts in drawing order.
fn texts(built: &BuiltPlot) -> Vec<&Text> {
    let panel = built.scene.groups_with_role("panel").into_iter().next();
    panel
        .expect("a panel group")
        .flattened()
        .into_iter()
        .filter_map(|node| match node {
            Node::Text(text) => Some(text),
            _ => None,
        })
        .collect()
}

#[test]
fn halo_outlines_draw_under_each_rows_lines() {
    let built = build(r#"{"geom": "text", "halo_width": 3}"#, "");
    assert!(built.warnings.is_empty(), "{:?}", built.warnings);
    let order: Vec<(&str, bool)> = texts(&built)
        .iter()
        .map(|text| (text.content.as_str(), text.halo.is_some()))
        .collect();
    // Per row: every line's halo, then the lines (lets-plot's element order).
    assert_eq!(
        order,
        [
            ("Ab", true),
            ("Ab", false),
            ("C", true),
            ("d", true),
            ("C", false),
            ("d", false)
        ]
    );
    // Stroke 2 x halo_width; default color: plot background (no panel fill).
    assert_eq!(texts(&built)[0].halo, Some((Color::WHITE, 6.0)));
}

#[test]
fn halo_color_defaults_to_the_panel_then_plot_background() {
    let layer = r#"{"geom": "text", "halo_width": 2}"#;
    let panel = build(
        layer,
        r#", "theme": {"panel_background": {"fill": "yellow"}, "plot_background": {"fill": "cyan"}}"#,
    );
    assert_eq!(texts(&panel)[0].halo, Some((Color::rgb(255, 255, 0), 4.0)));
    let plot = build(layer, r#", "theme": {"plot_background": {"fill": "cyan"}}"#);
    assert_eq!(texts(&plot)[0].halo, Some((Color::rgb(0, 255, 255), 4.0)));
    let blank = build(layer, r#", "theme": {"plot_background": "blank"}"#);
    assert_eq!(texts(&blank)[0].halo, Some((Color::WHITE, 4.0)));
}

#[test]
fn halo_color_is_faded_by_alpha_and_zero_width_draws_none() {
    let built = build(
        r#"{"geom": "text", "halo_width": 3, "halo_color": "red", "alpha": 0.5}"#,
        "",
    );
    assert_eq!(
        texts(&built)[0].halo,
        Some((Color::rgba(255, 0, 0, 128), 6.0))
    );
    for layer in [
        r#"{"geom": "text", "halo_color": "red"}"#,
        r#"{"geom": "text", "halo_width": 0}"#,
    ] {
        let built = build(layer, "");
        assert!(built.warnings.is_empty(), "{:?}", built.warnings);
        assert!(texts(&built).iter().all(|text| text.halo.is_none()));
        assert_eq!(texts(&built).len(), 3);
    }
}

#[test]
fn text_repel_draws_halos_and_label_does_not() {
    let repel = build(
        r#"{"geom": "text_repel", "halo_width": 3, "halo_color": "red", "seed": 1}"#,
        "",
    );
    assert!(repel.warnings.is_empty(), "{:?}", repel.warnings);
    assert_eq!(texts(&repel)[0].halo, Some((Color::rgb(255, 0, 0), 6.0)));
    // lets-plot's geom_label has no halo option: warned and not drawn.
    let label = build(r#"{"geom": "label", "halo_width": 3}"#, "");
    assert!(texts(&label).iter().all(|text| text.halo.is_none()));
    assert!(
        label
            .warnings
            .iter()
            .any(|warning| warning.contains("halo_width")),
        "{:?}",
        label.warnings
    );
}
