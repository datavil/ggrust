//! A plot layer: geom + stat + position + mapping + geom-specific parameters.

use serde::Deserialize;
use serde::de::IgnoredAny;

use super::{DataMeta, Mapping, Options, Value, from_value_tracked, join_path, report_unsupported};

/// One `geom_*()` layer.
#[derive(Debug, Clone, Deserialize)]
pub struct LayerSpec {
    /// Geom name, e.g. `point`, `violin`.
    pub geom: String,
    /// Stat name; `None` means the geom's default stat.
    #[serde(default)]
    pub stat: Option<String>,
    /// Position adjustment; `None` means the geom's default.
    #[serde(default)]
    pub position: Option<PositionSpec>,
    /// Layer-level mapping, merged over the plot mapping (if `inherit_aes`).
    #[serde(default)]
    pub mapping: Mapping,
    /// Column types and categorical ordering of the layer data.
    #[serde(default)]
    pub data_meta: DataMeta,
    /// Inline data. Skipped: frames are passed separately as Arrow.
    #[serde(default)]
    pub data: Option<IgnoredAny>,
    /// Whether the layer contributes to legends.
    #[serde(default)]
    pub show_legend: Option<bool>,
    /// Whether the plot mapping is inherited.
    #[serde(default)]
    pub inherit_aes: Option<bool>,
    /// Drop rows with missing values silently.
    #[serde(default)]
    pub na_rm: Option<bool>,
    /// Manual legend key (`layer_key(...)` or a label).
    #[serde(default)]
    pub manual_key: Option<Value>,
    /// Hover tooltips, `layer_tooltips()` or `"none"`.
    #[serde(default)]
    pub tooltips: Option<TooltipsSpec>,
    /// Annotation labels, `layer_labels()`.
    #[serde(default)]
    pub labels: Option<TooltipsSpec>,
    /// Sampling strategy. Accepted; currently unused.
    #[serde(default)]
    pub sampling: Option<Value>,
    /// Geom/stat specific options and constant aesthetics (`size=2`, `color="red"`).
    /// Validated by the geom and stat.
    #[serde(flatten)]
    pub parameters: Options,
}

impl LayerSpec {
    pub(super) fn collect_unsupported(&self, parent: &str, found: &mut Vec<String>) {
        // ggrust never samples, so only an explicit "none" matches what it does.
        if self
            .sampling
            .as_ref()
            .is_some_and(|sampling| sampling != "none")
        {
            found.push(join_path(parent, "sampling"));
        }
        self.data_meta
            .collect_unsupported(&join_path(parent, "data_meta"), found);
        for (key, tooltips) in [("tooltips", &self.tooltips), ("labels", &self.labels)] {
            if let Some(TooltipsSpec::Lines(lines)) = tooltips {
                let path = join_path(parent, key);
                report_unsupported(&path, &lines.unsupported, found);
                for (index, format) in lines.formats.iter().enumerate() {
                    let path = join_path(&path, &format!("formats[{index}]"));
                    report_unsupported(&path, &format.unsupported, found);
                }
            }
        }
    }
}

/// A position adjustment, given as a name (`"jitter"`) or with options.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "Value")]
pub struct PositionSpec {
    /// Position name, e.g. `dodge`, `jitterdodge`.
    pub name: String,
    /// Position specific options, e.g. `width`, `seed`. Validated by the position.
    pub parameters: Options,
}

impl TryFrom<Value> for PositionSpec {
    type Error = String;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::String(name) => Ok(PositionSpec {
                name,
                parameters: Options::new(),
            }),
            Value::Object(mut parameters) => match parameters.remove("name") {
                Some(Value::String(name)) => Ok(PositionSpec {
                    name,
                    parameters: parameters.into_iter().collect(),
                }),
                Some(other) => Err(format!("at 'name': expected a string, got {other}")),
                None => Err("at 'name': missing position name".into()),
            },
            other => Err(format!(
                "expected a position name or position_*() options, got {other}"
            )),
        }
    }
}

/// Tooltip (or label) configuration.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "Value")]
pub enum TooltipsSpec {
    /// `tooltips="none"`: hover tooltips disabled.
    Disabled,
    /// `layer_tooltips()...`
    Lines(Box<TooltipLines>),
}

impl TryFrom<Value> for TooltipsSpec {
    type Error = String;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::String(text) if text == "none" => Ok(TooltipsSpec::Disabled),
            Value::Object(_) => from_value_tracked(value).map(TooltipsSpec::Lines),
            other => Err(format!(
                "expected \"none\" or layer_tooltips() options, got {other}"
            )),
        }
    }
}

/// Content and look of a tooltip built with `layer_tooltips()`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct TooltipLines {
    /// Line templates, e.g. `"@cluster"`, `"value|@{expr}"`.
    pub lines: Option<Vec<String>>,
    /// Value formats per field.
    pub formats: Vec<TooltipFormat>,
    /// Variables shown as `name: value` lines.
    pub variables: Option<Vec<String>>,
    /// Title line template.
    pub title: Option<String>,
    /// Fixed anchor, e.g. `top_right`.
    pub tooltip_anchor: Option<String>,
    /// Minimum width in pixels.
    pub tooltip_min_width: Option<f64>,
    /// Tooltip box color.
    pub tooltip_color: Option<String>,
    /// Do not split long lines into a side tooltip.
    pub disable_splitting: Option<bool>,
    /// Keys no component consumes.
    #[serde(flatten)]
    pub unsupported: Options,
}

/// A value format for one tooltip field.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TooltipFormat {
    /// Field reference, e.g. `@x` or `^color`.
    pub field: String,
    /// Format string, e.g. `.2f`, `{.1%}`.
    pub format: String,
    /// Keys no component consumes.
    #[serde(flatten)]
    pub unsupported: Options,
}
