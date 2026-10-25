//! Regression tests for spec bugs found by bug-hunting agents (Phase 0.2).

#![allow(clippy::unwrap_used)]

use ggrust_core::spec::{Aesthetic, Figure, MappingValue, OneOrMany, PlotSpec, Value};

fn error_of(text: &str) -> String {
    Figure::from_json(text).unwrap_err().to_string()
}

fn plot_of(text: &str) -> Box<PlotSpec> {
    match Figure::from_json(text).unwrap() {
        Figure::Plot(plot) => plot,
        Figure::Subplots(_) => panic!("expected plot"),
    }
}

#[test]
fn hjust_and_vjust_are_aesthetics() {
    let plot = plot_of(
        r#"{"kind": "plot", "layers": [{"geom": "text", "mapping": {"hjust": "a", "vjust": "b"}}]}"#,
    );
    assert_eq!(
        plot.layers[0].mapping[&Aesthetic::Hjust].as_column(),
        Some("a")
    );
    assert_eq!(
        plot.layers[0].mapping[&Aesthetic::Vjust].as_column(),
        Some("b")
    );
}

#[test]
fn mapping_accepts_column_lists_and_other_values() {
    let plot = plot_of(
        r#"{"kind": "plot", "mapping": {"group": ["a", "b"]},
            "layers": [{"geom": "image", "mapping": {"paint_c": [0.0, 1.0]}}]}"#,
    );
    assert_eq!(
        plot.mapping[&Aesthetic::Group],
        MappingValue::Columns(vec!["a".into(), "b".into()])
    );
    assert_eq!(
        plot.layers[0].mapping[&Aesthetic::PaintC],
        MappingValue::Other(serde_json::json!([0.0, 1.0]))
    );
}

#[test]
fn previously_dropped_keys_are_reported() {
    let figure = Figure::from_json(
        r#"{"kind": "plot",
            "metainfo_list": [{"name": "font_family_info"}],
            "data_meta": {"geodataframe": {"geometry": "g"},
                          "series_annotations": [{"column": "t", "type": "datetime", "time_zone": "UTC", "extra": 1}],
                          "mapping_annotations": [{"aes": "x", "annotation": "as_discrete", "levels": ["b", "a"],
                                                   "odd": 1, "parameters": {"label": "c", "weird": 2}}]},
            "ggsize": {"width": 1, "height": 2, "dpi": 3},
            "ggtitle": {"text": "t", "hjust": 0},
            "caption": {"text": "c", "x": 1},
            "layers": [{"geom": "point", "tooltips": {"formats": [{"field": "@x", "format": ".1f", "y": 1}]},
                        "data_meta": {"strange": 1}}]}"#,
    )
    .unwrap();
    assert_eq!(
        figure.unsupported_options(),
        [
            "metainfo_list[0]",
            "data_meta.geodataframe",
            "data_meta.series_annotations[0].extra",
            "data_meta.mapping_annotations[0].odd",
            "data_meta.mapping_annotations[0].parameters.weird",
            "ggsize.dpi",
            "ggtitle.hjust",
            "caption.x",
            "layers[0].data_meta.strange",
            "layers[0].tooltips.formats[0].y",
        ]
    );
    let Figure::Plot(plot) = figure else {
        panic!("expected plot")
    };
    let annotation = &plot.data_meta.mapping_annotations[0];
    assert_eq!(
        annotation.levels,
        Some(vec![Value::from("b"), Value::from("a")])
    );
    assert_eq!(
        plot.data_meta.series_annotations[0].time_zone.as_deref(),
        Some("UTC")
    );
}

#[test]
fn theme_errors_keep_the_element_path() {
    let error = error_of(r#"{"kind": "plot", "theme": {"axis_text": {"size": "big"}}}"#);
    assert!(error.contains("at 'theme.axis_text.size': "), "{error}");
    let error = error_of(
        r#"{"kind": "subplots", "layout": {"name": "grid", "ncol": 1, "nrow": 1},
            "figures": [{"kind": "plot", "theme": {"axis_text": {"size": "big"}}}]}"#,
    );
    assert!(
        error.contains("at 'figures[0].theme.axis_text.size': "),
        "{error}"
    );
}

#[test]
fn root_error_in_nested_figure_has_a_clean_path() {
    let error = error_of(
        r#"{"kind": "subplots", "layout": {"name": "grid", "ncol": 1, "nrow": 1},
            "figures": [{"kind": "subplots", "figures": []}]}"#,
    );
    assert!(
        error.contains("at 'figures[0]': missing field `layout`"),
        "{error}"
    );
}

#[test]
fn leaf_errors_name_the_field() {
    let cases = [
        (
            r#"{"kind": "plot", "guides": {"color": {"name": "legend", "ncol": 2.5}}}"#,
            "at 'guides.color.ncol': ",
        ),
        (
            r#"{"kind": "plot", "layers": [{"geom": "point", "tooltips": {"formats": [{"field": "@x"}]}}]}"#,
            "at 'layers[0].tooltips.formats[0]': missing field `format`",
        ),
        (
            r#"{"kind": "plot", "layers": [{"geom": "point", "tooltips": "nonee"}]}"#,
            "at 'layers[0].tooltips': expected \"none\"",
        ),
        (
            r#"{"kind": "plot", "scales": [{"aesthetic": "colr"}]}"#,
            "at 'scales[0].aesthetic': unknown aesthetic 'colr'",
        ),
        (
            r#"{"kind": "plot", "scales": [{"aesthetic": ["x", "colr"]}]}"#,
            "at 'scales[0].aesthetic[1]': unknown aesthetic 'colr'",
        ),
        (
            r#"{"kind": "plot", "layers": [{"geom": "point", "position": {"width": 1}}]}"#,
            "at 'layers[0].position.name': missing position name",
        ),
    ];
    for (text, expected) in cases {
        let error = error_of(text);
        assert!(
            error.contains(expected),
            "expected {expected:?} in {error:?}"
        );
    }
}

#[test]
fn counts_accept_whole_floats_and_reject_others() {
    let plot = plot_of(
        r#"{"kind": "plot", "facet": {"name": "wrap", "facets": "a", "ncol": 2.0},
            "guides": {"color": {"name": "legend", "ncol": 2.0, "nbin": 3}}}"#,
    );
    assert_eq!(plot.facet.as_ref().and_then(|facet| facet.ncol), Some(2));
    let error = error_of(r#"{"kind": "plot", "facet": {"name": "wrap", "ncol": -1}}"#);
    assert!(
        error.contains("at 'facet.ncol': expected a non-negative whole number"),
        "{error}"
    );
    let figure = Figure::from_json(
        r#"{"kind": "subplots", "layout": {"name": "grid", "ncol": 2.0, "nrow": 1.0}, "figures": []}"#,
    )
    .unwrap();
    assert!(matches!(figure, Figure::Subplots(grid) if grid.layout.ncol == 2));
}

#[test]
fn sampling_none_is_supported() {
    let figure =
        Figure::from_json(r#"{"kind": "plot", "layers": [{"geom": "point", "sampling": "none"}]}"#)
            .unwrap();
    assert!(figure.unsupported_options().is_empty());
}

#[test]
fn guide_keys_normalise_color_aliases() {
    let plot = plot_of(r#"{"kind": "plot", "guides": {"colour": "none", "col": {"title": "D"}}}"#);
    assert_eq!(plot.guides.keys().collect::<Vec<_>>(), ["color"]);
}

#[test]
fn facet_lists_allow_nulls() {
    let plot = plot_of(
        r#"{"kind": "plot", "facet": {"name": "wrap", "facets": ["a", "b"],
            "order": [1, null], "format": [null, "{}"]}}"#,
    );
    let facet = plot.facet.as_ref().unwrap();
    assert_eq!(facet.order, Some(OneOrMany::Many(vec![Some(1), None])));
}
