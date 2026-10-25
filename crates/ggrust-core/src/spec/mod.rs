//! Typed plot specification, parsed from lets_plot-style `as_dict()` JSON.
//!
//! What it does: turns the spec emitted by the Python API into Rust types, with
//! errors that name the exact path (`layers[2].position`) that failed.
//!
//! What it does not do: hold data. The `data` keys are skipped here; frames
//! travel separately as Arrow (see the `data` module). It also does not check
//! geom/stat/scale specific options: those land in `parameters` maps and are
//! validated by the component that consumes them.
//!
//! Keys that no component consumes land in `unsupported` maps and are listed by
//! [`Figure::unsupported_options`], so nothing is silently ignored.

mod aesthetic;
mod coord;
mod facet;
mod guide;
mod layer;
mod plot;
mod scale;
mod subplots;
mod theme;

use std::collections::BTreeMap;

use serde::Deserialize;

pub use aesthetic::{Aesthetic, UnknownAesthetic};
pub use coord::CoordSpec;
pub use facet::FacetSpec;
pub use guide::GuideSpec;
pub use layer::{LayerSpec, PositionSpec, TooltipsSpec};
pub use plot::{
    CaptionSpec, DataMeta, MappingAnnotation, MappingAnnotationParameters, PlotSpec,
    SeriesAnnotation, SizeSpec, TitleSpec, ToolbarSpec,
};
pub use scale::ScaleSpec;
pub use subplots::{GridLayoutSpec, SubplotsSpec};
pub use theme::{ElementSpec, ThemeSpec, ThemeValue};

use crate::{Error, Result};

/// A dynamic JSON value, used for options whose type depends on context.
pub type Value = serde_json::Value;

/// Options keyed by name, kept in a stable (sorted) order.
pub type Options = BTreeMap<String, Value>;

/// Aesthetic to column name.
pub type Mapping = BTreeMap<Aesthetic, String>;

/// A figure: a single plot, or a grid of figures.
///
/// Selected by the `kind` key (`"plot"` or `"subplots"`).
#[derive(Debug, Clone)]
pub enum Figure {
    /// `ggplot(...) + ...`
    Plot(Box<PlotSpec>),
    /// `gggrid([...])`
    Subplots(Box<SubplotsSpec>),
}

impl Figure {
    /// Parse a figure from JSON text.
    ///
    /// Errors name the path of the offending value, e.g. `layers[0].mapping`.
    pub fn from_json(text: &str) -> Result<Self> {
        let value: Value = serde_json::from_str(text)
            .map_err(|error| Error::InvalidSpec(format!("not valid JSON: {error}")))?;
        Figure::from_value(value).map_err(Error::InvalidSpec)
    }

    /// Paths of every option that ggrust does not support, e.g. `theme.axis_text.margin`.
    pub fn unsupported_options(&self) -> Vec<String> {
        let mut found = Vec::new();
        match self {
            Figure::Plot(plot) => plot.collect_unsupported("", &mut found),
            Figure::Subplots(subplots) => subplots.collect_unsupported("", &mut found),
        }
        found
    }

    /// Dispatch on `kind`, tracking the path of any error inside the figure.
    ///
    /// Errors are formatted `at '<path>': <message>`; nested figures join paths.
    fn from_value(value: Value) -> std::result::Result<Self, String> {
        let kind = value.get("kind").and_then(Value::as_str).map(str::to_owned);
        match kind.as_deref() {
            Some("plot") => deserialize_tracked(value).map(Figure::Plot),
            Some("subplots") => deserialize_tracked(value).map(Figure::Subplots),
            Some(other) => Err(format!(
                "at 'kind': unknown figure kind '{other}'; expected 'plot' or 'subplots'"
            )),
            None => Err("at 'kind': missing figure kind; expected 'plot' or 'subplots'".into()),
        }
    }
}

impl<'de> Deserialize<'de> for Figure {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        Figure::from_value(value).map_err(serde::de::Error::custom)
    }
}

/// Deserialize `value`, reporting failures as `at '<path>': <message>`.
fn deserialize_tracked<T: serde::de::DeserializeOwned>(
    value: Value,
) -> std::result::Result<Box<T>, String> {
    serde_path_to_error::deserialize(value)
        .map(Box::new)
        .map_err(|error| {
            let path = error.path().to_string();
            let message = error.into_inner().to_string();
            nest_error_path(&path, &message)
        })
}

/// Join an outer path with the path inside a nested figure error.
///
/// `("figures[0]", "at 'layers[1]': bad")` -> `"at 'figures[0].layers[1]': bad"`.
fn nest_error_path(outer: &str, message: &str) -> String {
    let outer = if outer == "." { "" } else { outer };
    match message
        .strip_prefix("at '")
        .and_then(|rest| rest.split_once("': "))
    {
        Some((inner, rest)) => format!("at '{}': {rest}", join_path(outer, inner)),
        None if outer.is_empty() => format!("at '.': {message}"),
        None => format!("at '{outer}': {message}"),
    }
}

/// A value given either as a single item or as a list (`"a"` or `["a", "b"]`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    /// A single item.
    One(T),
    /// A list of items.
    Many(Vec<T>),
}

impl<T> OneOrMany<T> {
    /// The items as a slice, whichever form was given.
    pub fn as_slice(&self) -> &[T] {
        match self {
            OneOrMany::One(item) => std::slice::from_ref(item),
            OneOrMany::Many(items) => items,
        }
    }
}

/// Join a parent path and a key: `("layers[0]", "size")` -> `"layers[0].size"`.
fn join_path(parent: &str, key: &str) -> String {
    if parent.is_empty() {
        key.to_owned()
    } else {
        format!("{parent}.{key}")
    }
}

/// Append the path of every key in `unsupported` to `found`.
fn report_unsupported(parent: &str, unsupported: &Options, found: &mut Vec<String>) {
    found.extend(unsupported.keys().map(|key| join_path(parent, key)));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error_of(text: &str) -> String {
        Figure::from_json(text).unwrap_err().to_string()
    }

    #[test]
    fn error_names_the_path_of_an_unknown_aesthetic() {
        let error = error_of(
            r#"{"kind": "plot", "layers": [{"geom": "point", "mapping": {"colr": "a"}}]}"#,
        );
        assert!(error.contains("at 'layers[0].mapping"), "{error}");
        assert!(error.contains("unknown aesthetic 'colr'"), "{error}");
    }

    #[test]
    fn error_for_missing_geom() {
        let error = error_of(r#"{"kind": "plot", "layers": [{"mapping": {}}]}"#);
        assert!(error.contains("missing field `geom`"), "{error}");
    }

    #[test]
    fn error_for_unknown_kind() {
        let error = error_of(r#"{"kind": "bunch"}"#);
        assert!(
            error.contains("at 'kind': unknown figure kind 'bunch'"),
            "{error}"
        );
    }

    #[test]
    fn error_path_reaches_into_nested_figures() {
        let error = error_of(
            r#"{"kind": "subplots", "layout": {"name": "grid", "ncol": 1, "nrow": 1},
                "figures": [{"kind": "plot", "layers": [{"geom": "point", "mapping": {"colr": "a"}}]}]}"#,
        );
        assert!(
            error.contains("at 'figures[0].layers[0].mapping"),
            "{error}"
        );
    }

    #[test]
    fn error_for_invalid_json() {
        assert!(error_of("{").contains("not valid JSON"));
    }

    #[test]
    fn colour_alias_maps_to_color() {
        let Figure::Plot(plot) =
            Figure::from_json(r#"{"kind": "plot", "mapping": {"colour": "a"}}"#).unwrap()
        else {
            panic!("expected plot");
        };
        assert_eq!(plot.mapping[&Aesthetic::Color], "a");
    }

    #[test]
    fn unsupported_options_are_reported_with_paths() {
        let figure = Figure::from_json(
            r#"{
                "kind": "subplots",
                "layout": {"name": "grid", "ncol": 1, "nrow": 1, "magic": 1},
                "figures": [{
                    "kind": "plot",
                    "future_option": true,
                    "layers": [{"geom": "point", "sampling": {"name": "random"},
                                "tooltips": {"lines": ["@x"], "sparkle": 1}}],
                    "theme": {"axis_text": {"size": 3, "glow": 1}},
                    "guides": {"color": {"name": "legend", "shimmer": 2}},
                    "coord": {"name": "cartesian", "clip": "off"},
                    "facet": {"name": "wrap", "facets": "a", "labeller": "x"},
                    "ggtoolbar": {"size_zoomin": 1, "extra": 0}
                }]
            }"#,
        )
        .unwrap();
        assert_eq!(
            figure.unsupported_options(),
            [
                "layout.magic",
                "figures[0].future_option",
                "figures[0].layers[0].sampling",
                "figures[0].layers[0].tooltips.sparkle",
                "figures[0].theme.axis_text.glow",
                "figures[0].guides.color.shimmer",
                "figures[0].coord.clip",
                "figures[0].facet.labeller",
                "figures[0].ggtoolbar.extra",
            ]
        );
    }

    #[test]
    fn tooltips_none_is_disabled() {
        let Figure::Plot(plot) = Figure::from_json(
            r#"{"kind": "plot", "layers": [{"geom": "point", "tooltips": "none"}]}"#,
        )
        .unwrap() else {
            panic!("expected plot");
        };
        assert!(matches!(
            plot.layers[0].tooltips,
            Some(TooltipsSpec::Disabled(_))
        ));
    }
}
