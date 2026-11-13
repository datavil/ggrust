//! Per-guide `position` (ggplot2 3.5 `guide_*(position=)`): guides at several
//! sides of one plot, inside the panel, and `legend_position="none"` hiding all.

#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use arrow_array::{ArrayRef, Float64Array, RecordBatch, StringArray};
use ggrust_core::build::{BuiltPlot, PlotData, build_plot};
use ggrust_core::data::DataFrame;
use ggrust_core::scene::{Node, Rect};
use ggrust_core::spec::{Figure, PlotSpec};

fn spec(json: &str) -> PlotSpec {
    match Figure::from_json(json).unwrap() {
        Figure::Plot(plot) => *plot,
        Figure::Subplots(_) => panic!("expected a plot"),
    }
}

fn pets() -> DataFrame {
    let batch = RecordBatch::try_from_iter(vec![
        (
            "x",
            Arc::new(Float64Array::from(vec![1.0, 2.0, 3.0, 4.0])) as ArrayRef,
        ),
        ("y", Arc::new(Float64Array::from(vec![2.0, 1.0, 4.0, 3.0]))),
        (
            "pet",
            Arc::new(StringArray::from(vec!["cat", "dog", "cat", "fox"])),
        ),
        (
            "home",
            Arc::new(StringArray::from(vec!["flat", "house", "barn", "flat"])),
        ),
    ])
    .unwrap();
    DataFrame::new(batch.schema(), vec![batch]).unwrap()
}

/// The plot with `guides` and `theme` (JSON object bodies) over color = pet and
/// shape = home.
fn build(guides: &str, theme: &str) -> BuiltPlot {
    let json = format!(
        r#"{{"kind": "plot", "mapping": {{"x": "x", "y": "y", "color": "pet", "shape": "home"}},
            "layers": [{{"geom": "point"}}], "guides": {{{guides}}}, "theme": {{{theme}}}}}"#
    );
    build_plot(
        &spec(&json),
        &PlotData {
            plot: Some(pets()),
            layers: vec![],
        },
    )
    .unwrap()
}

fn panel(built: &BuiltPlot) -> Rect {
    built.scene.groups_with_role("panel")[0].clip.unwrap()
}

/// Anchor of the text `content` (a legend label or title).
fn text_at(built: &BuiltPlot, content: &str) -> Option<(f64, f64)> {
    fn find(nodes: &[Node], content: &str) -> Option<(f64, f64)> {
        nodes.iter().find_map(|node| match node {
            Node::Text(text) if text.content == content => Some((text.x, text.y)),
            Node::Group(group) => find(&group.nodes, content),
            _ => None,
        })
    }
    find(&built.scene.nodes, content)
}

#[test]
fn guides_go_to_their_own_sides() {
    let built = build(
        r#""color": {"name": "legend", "position": "bottom"},
           "shape": {"name": "legend", "position": "left"}"#,
        "",
    );
    let panel = panel(&built);
    let (pet_x, pet_y) = text_at(&built, "pet").unwrap();
    let (home_x, _) = text_at(&built, "home").unwrap();
    assert!(
        pet_y > panel.y + panel.height,
        "pet title at {pet_y}, panel {panel:?}"
    );
    assert!(pet_x > panel.x, "a bottom legend starts along the panel");
    assert!(home_x < panel.x, "home title at {home_x}, panel {panel:?}");
    // A bottom legend lays its keys in a row (ggplot2 and lets-plot defaults).
    let (_, cat_y) = text_at(&built, "cat").unwrap();
    let (_, fox_y) = text_at(&built, "fox").unwrap();
    assert!((cat_y - fox_y).abs() < 1e-9);
}

#[test]
fn unpositioned_guides_follow_legend_position() {
    let built = build(
        r#""color": {"name": "legend", "position": "top"}"#,
        r#""legend_position": "left""#,
    );
    let panel = panel(&built);
    assert!(text_at(&built, "pet").unwrap().1 < panel.y);
    assert!(text_at(&built, "home").unwrap().0 < panel.x);
    // Nothing is left on the right: the panel runs to the plot margin.
    let plain = build(
        r#""color": {"name": "legend", "position": "left"}"#,
        r#""legend_position": "left""#,
    );
    assert!(panel.x + panel.width > self::panel(&plain).x + self::panel(&plain).width - 1e-9);
}

#[test]
fn a_guide_at_legend_position_changes_nothing() {
    let plain = build("", "");
    let same = build(r#""color": {"name": "legend", "position": "right"}"#, "");
    assert_eq!(panel(&plain), panel(&same));
    assert_eq!(text_at(&plain, "pet"), text_at(&same, "pet"));
    assert_eq!(text_at(&plain, "home"), text_at(&same, "home"));
}

#[test]
fn inside_guides_sit_in_the_panel_at_legend_position_inside() {
    let built = build(
        r#""color": {"name": "legend", "position": "inside"}"#,
        r#""legend_position_inside": [0.1, 0.9], "legend_justification": [0, 1]"#,
    );
    let panel = panel(&built);
    let (pet_x, pet_y) = text_at(&built, "pet").unwrap();
    assert!(pet_x > panel.x && pet_x < panel.x + panel.width * 0.4);
    assert!(pet_y > panel.y && pet_y < panel.y + panel.height * 0.4);
    // The other guide stays on the right.
    assert!(text_at(&built, "home").unwrap().0 > panel.x + panel.width);
}

#[test]
fn a_guide_theme_can_move_its_inside_point() {
    let built = build(
        r#""color": {"name": "legend", "position": "inside",
                     "theme": {"legend_position_inside": [1, 0], "legend_justification": [1, 0]}}"#,
        "",
    );
    let panel = panel(&built);
    let (pet_x, pet_y) = text_at(&built, "pet").unwrap();
    assert!(pet_x > panel.x + panel.width * 0.6, "{pet_x} in {panel:?}");
    assert!(pet_y > panel.y + panel.height * 0.6, "{pet_y} in {panel:?}");
}

#[test]
fn legend_position_inside_string_uses_the_inside_point() {
    let built = build(
        "",
        r#""legend_position": "inside", "legend_position_inside": [0.5, 0.5]"#,
    );
    let panel = panel(&built);
    let plain = self::panel(&build("", r#""legend_position": "none""#));
    assert_eq!(panel, plain, "inside legends take no room");
    let (pet_x, _) = text_at(&built, "pet").unwrap();
    assert!(pet_x > panel.x && pet_x < panel.x + panel.width);
}

#[test]
fn legend_position_none_hides_positioned_guides_too() {
    let built = build(
        r#""color": {"name": "legend", "position": "bottom"}"#,
        r#""legend_position": "none""#,
    );
    assert_eq!(text_at(&built, "pet"), None);
    assert_eq!(text_at(&built, "home"), None);
}

#[test]
fn unknown_positions_are_spec_errors() {
    let json = r#"{"kind": "plot", "mapping": {"x": "x", "y": "y", "color": "pet"},
        "layers": [{"geom": "point"}], "guides": {"color": {"name": "legend", "position": "botom"}}}"#;
    let error = build_plot(
        &spec(json),
        &PlotData {
            plot: Some(pets()),
            layers: vec![],
        },
    )
    .err()
    .unwrap()
    .to_string();
    assert!(error.contains("guides.color.position"), "{error}");
    assert!(error.contains("did you mean 'bottom'"), "{error}");
}

#[test]
fn a_numeric_legend_position_is_the_inside_point() {
    let built = build(
        r#""color": {"name": "legend", "position": "inside"}"#,
        r#""legend_position": [1, 1], "legend_justification": [1, 1]"#,
    );
    let panel = panel(&built);
    let (pet_x, pet_y) = text_at(&built, "pet").unwrap();
    assert!(pet_x > panel.x + panel.width * 0.6, "{pet_x} in {panel:?}");
    assert!(pet_y < panel.y + panel.height * 0.4, "{pet_y} in {panel:?}");
}

#[test]
fn legends_on_opposite_sides_share_the_room() {
    let labels: Vec<String> = (0..60)
        .map(|index| format!("category number {index}"))
        .collect();
    let others: Vec<String> = (0..60)
        .map(|index| format!("another label {index}"))
        .collect();
    let values: Vec<f64> = (0..60).map(f64::from).collect();
    let batch = RecordBatch::try_from_iter(vec![
        (
            "x",
            Arc::new(Float64Array::from(values.clone())) as ArrayRef,
        ),
        ("y", Arc::new(Float64Array::from(values))),
        ("a", Arc::new(StringArray::from(labels))),
        ("b", Arc::new(StringArray::from(others))),
    ])
    .unwrap();
    let data = DataFrame::new(batch.schema(), vec![batch]).unwrap();
    for (first, second) in [("top", "bottom"), ("left", "right")] {
        let json = format!(
            r#"{{"kind": "plot", "mapping": {{"x": "x", "y": "y", "color": "a", "shape": "b"}},
                "layers": [{{"geom": "point"}}], "ggsize": {{"width": 500, "height": 400}},
                "guides": {{"color": {{"name": "legend", "position": "{first}"}},
                            "shape": {{"name": "legend", "position": "{second}"}}}}}}"#
        );
        let built = build_plot(
            &spec(&json),
            &PlotData {
                plot: Some(data.clone()),
                layers: vec![],
            },
        )
        .unwrap();
        fn extent(nodes: &[Node], far: &mut (f64, f64)) {
            for node in nodes {
                match node {
                    Node::Text(text) => *far = (far.0.max(text.x), far.1.max(text.y)),
                    Node::Group(group) => extent(&group.nodes, far),
                    _ => {}
                }
            }
        }
        let mut far = (0.0, 0.0);
        extent(&built.scene.nodes, &mut far);
        assert!(
            far.1 <= built.scene.height,
            "{first}/{second}: text at {far:?}"
        );
        assert!(panel(&built).width >= 50.0 && panel(&built).height >= 50.0);
    }
}

#[test]
fn bad_inside_points_are_spec_errors() {
    let json = r#"{"kind": "plot", "mapping": {"x": "x", "y": "y"},
        "layers": [{"geom": "point"}], "theme": {"legend_position_inside": "top"}}"#;
    let error = build_plot(
        &spec(json),
        &PlotData {
            plot: Some(pets()),
            layers: vec![],
        },
    )
    .err()
    .unwrap()
    .to_string();
    assert!(error.contains("theme.legend_position_inside"), "{error}");
    assert!(error.contains("expected two numbers"), "{error}");
}
