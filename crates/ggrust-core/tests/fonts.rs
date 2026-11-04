//! End-to-end: fonts registered at run time (`register_font`) are measured and
//! drawn with in layout, SVG (embedded subset), PNG and PDF.
//!
//! The registry is process-wide and tests run in parallel: every test registers
//! under its own family name (`family=`), so tests never see each other's fonts.

#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use arrow_array::{ArrayRef, Float64Array, RecordBatch, StringArray};
use ggrust_core::Error;
use ggrust_core::build::{BuiltPlot, PlotData, build_plot};
use ggrust_core::data::DataFrame;
use ggrust_core::render::{to_pdf, to_png, to_svg};
use ggrust_core::scene::Node;
use ggrust_core::spec::{Figure, PlotSpec};
use ggrust_core::text::{
    Font, FontFace, FontFamily, face_warning, family_warning, measure, register_font,
    register_font_dir, registered_fonts,
};

fn fonts_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fonts")
}

fn serif_regular() -> PathBuf {
    fonts_dir().join("LiberationSerif-Regular.ttf")
}

fn serif_bold() -> PathBuf {
    fonts_dir().join("LiberationSerif-Bold.ttf")
}

fn build(family: &str) -> BuiltPlot {
    let json = format!(
        r#"{{"kind": "plot", "mapping": {{"x": "x", "y": "y"}},
            "layers": [{{"geom": "point"}}],
            "ggtitle": {{"text": "Wavy Title"}},
            "theme": {{"text": {{"family": "{family}", "blank": false}},
                       "plot_title": {{"face": "bold", "blank": false}}}}}}"#
    );
    let Figure::Plot(plot) = Figure::from_json(&json).unwrap() else {
        panic!("expected a plot")
    };
    let spec: PlotSpec = *plot;
    let batch = RecordBatch::try_from_iter(vec![
        (
            "x",
            Arc::new(Float64Array::from(vec![1.0, 2.0, 3.0])) as ArrayRef,
        ),
        (
            "y",
            Arc::new(StringArray::from(vec![
                "a long category name",
                "WWWW wide",
                "iiii narrow",
            ])) as ArrayRef,
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

fn panel_left(built: &BuiltPlot) -> f64 {
    let panel = built.scene.groups_with_role("panel")[0];
    panel.clip.expect("panel clip").x
}

fn texts_with(built: &BuiltPlot, content: &str) -> Vec<Font> {
    fn walk(nodes: &[Node], content: &str, found: &mut Vec<Font>) {
        for node in nodes {
            match node {
                Node::Group(group) => walk(&group.nodes, content, found),
                Node::Text(text) if text.content == content => found.push(text.font),
                _ => {}
            }
        }
    }
    let mut found = Vec::new();
    walk(&built.scene.nodes, content, &mut found);
    found
}

#[test]
fn registered_family_is_read_from_the_file() {
    let names = register_font(&serif_regular(), None).unwrap();
    assert_eq!(names, ["Liberation Serif"]);
    let (family, known) = FontFamily::from_name("liberation SERIF");
    assert!(known);
    assert!(matches!(family, FontFamily::Registered(_)));
    assert!(
        registered_fonts()
            .iter()
            .any(|(name, faces)| name == "Liberation Serif" && faces.contains(&FontFace::Plain))
    );
}

#[test]
fn registered_font_metrics_drive_layout() {
    register_font(&serif_regular(), Some("Layout Serif")).unwrap();
    let serif = Font {
        family: FontFamily::from_name("Layout Serif").0,
        face: FontFace::Plain,
    };
    // Times metrics: "W" is 944/1000 em in Liberation Serif, 944 in Arial too,
    // but "a" is 444 (serif) vs 556 (sans).
    let a = measure("a", 1000.0, serif).width;
    assert!((a - 444.0).abs() < 1.0, "{a}");
    let sans = build("sans");
    let custom = build("Layout Serif");
    assert!(
        custom
            .warnings
            .iter()
            .all(|warning| !warning.contains("is not registered"))
    );
    // Narrower serif axis labels give the panel more room on the left.
    assert!(
        panel_left(&custom) < panel_left(&sans) - 3.0,
        "{} vs {}",
        panel_left(&custom),
        panel_left(&sans)
    );
    let fonts = texts_with(&custom, "a long category name");
    assert_eq!(fonts.len(), 1);
    assert_eq!(fonts[0].family, serif.family);
}

#[test]
fn every_output_draws_the_registered_font() {
    register_font(&serif_regular(), Some("Output Serif")).unwrap();
    register_font(&serif_bold(), Some("Output Serif")).unwrap();
    let custom = build("Output Serif");
    let sans = build("sans");

    let svg = to_svg(&custom.scene);
    assert!(
        svg.contains("@font-face{font-family:'ggrust-"),
        "no web font"
    );
    assert!(svg.contains("Output Serif"));
    assert!(svg.contains("unicode-range:"));
    // Liberation Sans text is not embedded.
    assert!(!to_svg(&sans.scene).contains("@font-face"));

    let png = to_png(&custom.scene, 1.0).unwrap();
    assert_ne!(png, to_png(&sans.scene, 1.0).unwrap());

    let pdf = to_pdf(&custom.scene, 1.0, 96.0).unwrap();
    let pdf = String::from_utf8_lossy(&pdf);
    assert!(
        pdf.contains("+LiberationSerif"),
        "serif not embedded in the PDF"
    );
    // The bold title draws the registered bold face.
    assert!(pdf.contains("+LiberationSerif-Bold"));
}

#[test]
fn same_png_through_every_registration_path() {
    // A directory registration and a file registration of the same face draw alike.
    register_font(&serif_regular(), Some("Path Serif A")).unwrap();
    let registered = register_font_dir(&fonts_dir()).unwrap();
    assert_eq!(registered.families, ["Liberation Serif"]);
    assert!(registered.skipped.is_empty());
    register_font(&serif_regular(), Some("Path Serif B")).unwrap();
    let first = to_png(&build("Path Serif A").scene, 1.0).unwrap();
    let second = to_png(&build("Path Serif B").scene, 1.0).unwrap();
    assert_eq!(first, second);
}

#[test]
fn missing_face_uses_the_nearest_and_warns() {
    register_font(&serif_regular(), Some("Regular Only Serif")).unwrap();
    let family = FontFamily::from_name("Regular Only Serif").0;
    let bold = Font {
        family,
        face: FontFace::Bold,
    };
    assert_eq!(bold.resolved_face(), FontFace::Plain);
    let warning = face_warning(bold, "theme").unwrap();
    assert!(warning.contains("no bold face registered"), "{warning}");
    assert!(warning.contains("plain face was used"), "{warning}");
    // The bold plot title warns once.
    let built = build("Regular Only Serif");
    let count = built
        .warnings
        .iter()
        .filter(|warning| warning.contains("no bold face"))
        .count();
    assert_eq!(count, 1, "{:?}", built.warnings);
}

#[test]
fn unknown_family_warns_with_a_suggestion() {
    register_font(&serif_regular(), Some("Brand Serif")).unwrap();
    let warning = family_warning("Brand Serf", "theme").unwrap();
    assert!(
        warning.contains("'Brand Serf' is not registered"),
        "{warning}"
    );
    assert!(warning.contains("did you mean 'Brand Serif'?"), "{warning}");
    assert!(warning.contains("Liberation Sans was used"), "{warning}");
    assert!(family_warning("Arial", "theme").is_none());
    let built = build("Brand Serf");
    assert!(
        built
            .warnings
            .iter()
            .any(|warning| warning.contains("did you mean 'Brand Serif'?")),
        "{:?}",
        built.warnings
    );
}

#[test]
fn register_errors_say_what_and_how_to_fix() {
    let missing = register_font(&fonts_dir().join("Nope.ttf"), None).unwrap_err();
    assert!(matches!(missing, Error::FontNotFound { .. }), "{missing}");
    assert!(missing.to_string().contains("Nope.ttf"));

    let not_font = register_font(&fonts_dir().join("OFL.txt"), None).unwrap_err();
    assert!(matches!(not_font, Error::InvalidFont { .. }), "{not_font}");
    assert!(
        not_font.to_string().contains("not a font file"),
        "{not_font}"
    );

    let directory = register_font(&fonts_dir(), None).unwrap_err();
    assert!(
        directory.to_string().contains("register_font_dir"),
        "{directory}"
    );

    let empty = register_font(&serif_regular(), Some("  ")).unwrap_err();
    assert!(empty.to_string().contains("non-empty"), "{empty}");

    let no_directory = register_font_dir(&fonts_dir().join("nothing")).unwrap_err();
    assert!(matches!(no_directory, Error::FontNotFound { .. }));
    let no_fonts = register_font_dir(&fonts_dir().join("../fixtures")).unwrap_err();
    assert!(no_fonts.to_string().contains("no font files"), "{no_fonts}");
}
