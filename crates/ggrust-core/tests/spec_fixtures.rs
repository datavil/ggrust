//! Parse every lets_plot-generated fixture and check the fields cellestial relies on.
//!
//! Regenerate fixtures with `uv run python tests/fixtures/generate_spec_fixtures.py`.

use std::fs;
use std::path::{Path, PathBuf};

use ggrust_core::spec::{
    Aesthetic, Figure, GuideSpec, PlotSpec, SubplotsSpec, ThemeValue, TooltipsSpec, Value,
};

fn fixture_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/specs")
}

fn load(name: &str) -> Figure {
    let path = fixture_directory().join(format!("{name}.json"));
    let text = fs::read_to_string(&path).expect("fixture exists");
    Figure::from_json(&text).unwrap_or_else(|error| panic!("{name}: {error}"))
}

fn plot(name: &str) -> Box<PlotSpec> {
    match load(name) {
        Figure::Plot(plot) => plot,
        Figure::Subplots(_) => panic!("{name} is a grid, expected a plot"),
    }
}

fn subplots(name: &str) -> Box<SubplotsSpec> {
    match load(name) {
        Figure::Subplots(subplots) => subplots,
        Figure::Plot(_) => panic!("{name} is a plot, expected a grid"),
    }
}

#[test]
fn every_fixture_parses_with_no_unsupported_options() {
    let mut count = 0;
    for entry in fs::read_dir(fixture_directory()).expect("fixture directory") {
        let path = entry.expect("entry").path();
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .expect("name");
        let unsupported = load(name).unsupported_options();
        assert!(
            unsupported.is_empty(),
            "{name}: unsupported {unsupported:?}"
        );
        count += 1;
    }
    assert!(count >= 17, "expected at least 17 fixtures, found {count}");
}

#[test]
fn umap_categorical_fields() {
    let plot = plot("umap_categorical");
    assert_eq!(plot.mapping[&Aesthetic::Color].as_column(), Some("cluster"));
    assert_eq!(
        plot.ggsize.map(|size| (size.width, size.height)),
        Some((500.0, 400.0))
    );
    assert_eq!(
        plot.ggtoolbar
            .as_ref()
            .and_then(|toolbar| toolbar.size_zoomin),
        Some(-1.0)
    );
    assert_eq!(plot.coord.as_ref().and_then(|coord| coord.ratio), Some(1.0));

    let layer = &plot.layers[0];
    assert_eq!(layer.geom, "point");
    assert_eq!(layer.parameters["size"], Value::from(1.5));
    let Some(TooltipsSpec::Lines(tooltips)) = &layer.tooltips else {
        panic!("expected tooltip lines");
    };
    assert_eq!(
        tooltips.lines.as_deref(),
        Some(&["@cluster".to_owned()][..])
    );
    assert_eq!(
        tooltips.variables.as_deref(),
        Some(&["sample".to_owned()][..])
    );
    assert_eq!(tooltips.formats[0].format, ".2f");

    assert_eq!(plot.scales[0].aesthetic.as_slice(), &[Aesthetic::Color]);
    assert_eq!(
        plot.scales[0].scale_mapper_kind.as_deref(),
        Some("color_brewer")
    );
    assert_eq!(plot.scales[0].parameters["palette"], Value::from("Set2"));

    let GuideSpec::Full(legend) = &plot.guides["color"] else {
        panic!("expected a full color guide");
    };
    assert_eq!(legend.ncol, Some(2));
    assert_eq!(legend.title.as_deref(), Some("Cluster"));
    let GuideSpec::Full(x_axis) = &plot.guides["x"] else {
        panic!("labs(x=...) is stored as a title-only guide");
    };
    assert_eq!(x_axis.title.as_deref(), Some("UMAP1"));

    let theme = plot.theme.as_ref().expect("theme");
    assert_eq!(theme.name.as_deref(), Some("classic"));
    assert_eq!(theme.elements["axis_text"], ThemeValue::Blank);
    assert_eq!(
        theme.elements["legend_position"],
        ThemeValue::Scalar(Value::from("right"))
    );
}

#[test]
fn violin_box_positions_and_feature_list_layers() {
    let plot = plot("violin_box");
    let geoms: Vec<&str> = plot
        .layers
        .iter()
        .map(|layer| layer.geom.as_str())
        .collect();
    // geom_boxplot expands to a boxplot layer plus an outlier point layer.
    assert_eq!(geoms, ["violin", "boxplot", "point", "point"]);
    let dodge = plot.layers[1].position.as_ref().expect("dodge");
    assert_eq!(dodge.name, "dodge");
    assert_eq!(dodge.parameters["width"], Value::from(0.9));
    let jitter = plot.layers[3].position.as_ref().expect("jitterdodge");
    assert_eq!(jitter.name, "jitterdodge");
    assert_eq!(jitter.parameters["seed"], Value::from(42));
    assert!(
        plot.scales
            .iter()
            .any(|scale| scale.trans.as_deref() == Some("log2"))
    );
}

#[test]
fn string_position_becomes_name_only() {
    let plot = plot("distributions");
    let position = plot.layers[0].position.as_ref().expect("position");
    assert_eq!(position.name, "dodge");
    assert!(position.parameters.is_empty());
}

#[test]
fn void_theme_blank_strings_and_elements() {
    let plot = plot("void_theme");
    let theme = plot.theme.as_ref().expect("theme");
    assert_eq!(theme.elements["axis"], ThemeValue::Blank);
    let ThemeValue::Element(title) = &theme.elements["axis_title"] else {
        panic!("expected an element");
    };
    assert_eq!(title.face.as_deref(), Some("bold"));
    assert_eq!(title.margin, Some(serde_json::json!([1, 2, 3, 4])));
    assert_eq!(
        theme.elements["legend_position"],
        ThemeValue::Scalar(serde_json::json!([1, 1]))
    );
    let annotation = &plot.data_meta.mapping_annotations[0];
    assert_eq!(annotation.aes, Aesthetic::Color);
    assert_eq!(annotation.parameters.order, Some(-1));
}

#[test]
fn ordered_levels_come_through_series_annotations() {
    for name in ["pandas_categorical", "polars_enum"] {
        let plot = plot(name);
        let cluster = plot
            .data_meta
            .series_annotations
            .iter()
            .find(|annotation| annotation.column == "cluster")
            .expect("cluster annotation");
        assert_eq!(
            cluster.factor_levels,
            Some(vec![Value::from("T"), Value::from("NK"), Value::from("B")]),
            "{name}"
        );
    }
}

#[test]
fn polars_categorical_has_no_levels_in_lets_plot_specs() {
    // lets_plot drops polars category order; ggrust must read it from Arrow instead.
    let plot = plot("umap_categorical");
    let cluster = plot
        .data_meta
        .series_annotations
        .iter()
        .find(|annotation| annotation.column == "cluster")
        .expect("cluster annotation");
    assert_eq!(cluster.series_type.as_deref(), Some("str"));
    assert!(cluster.factor_levels.is_none());
}

#[test]
fn faceted_wrap_and_cartesian_limits() {
    let plot = plot("faceted");
    let facet = plot.facet.as_ref().expect("facet");
    assert_eq!(facet.name, "wrap");
    assert_eq!(
        facet.facets.as_ref().map(|f| f.as_slice().to_vec()),
        Some(vec!["sample".to_owned()])
    );
    assert_eq!(facet.scales.as_deref(), Some("free_y"));
    let coord = plot.coord.as_ref().expect("coord");
    assert_eq!(coord.ylim, Some([None, Some(1.5)]));
    assert_eq!(
        plot.caption.as_ref().and_then(|c| c.text.as_deref()),
        Some("caption text")
    );
}

#[test]
fn grid_layout_empty_cells_and_nesting() {
    let grid = subplots("grid");
    assert_eq!((grid.layout.ncol, grid.layout.nrow), (2, 3));
    assert_eq!(grid.layout.guides.as_deref(), Some("collect"));
    assert_eq!(grid.layout.align, Some(true));
    // lets_plot pads `figures` with nulls up to nrow * ncol.
    assert_eq!(grid.figures.len(), 6);
    assert!(grid.figures[2].is_none());
    assert!(matches!(grid.figures[4], Some(Figure::Subplots(_))));
    assert!(grid.figures[5].is_none());
}

#[test]
fn inline_data_is_skipped() {
    let plot = plot("with_inline_data");
    assert!(plot.data.is_some());
    assert!(plot.layers[0].data.is_some());
    assert!(!plot.layers[0].parameters.contains_key("data"));
}
