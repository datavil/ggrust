//! A plot layer: geom + stat + position + mapping + geom-specific parameters.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::{
    Aesthetic, DataMeta, DataSource, Mapping, Options, Value, from_value_tracked, join_path,
    mapping, report_unsupported,
};

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
    #[serde(default, deserialize_with = "mapping")]
    pub mapping: Mapping,
    /// Column types and categorical ordering of the layer data.
    #[serde(default)]
    pub data_meta: DataMeta,
    /// Layer data: a reference to a frame passed separately as Arrow.
    #[serde(default)]
    pub data: Option<DataSource>,
    /// Whether the layer contributes to legends.
    #[serde(default)]
    pub show_legend: Option<bool>,
    /// Whether the plot mapping is inherited.
    #[serde(default)]
    pub inherit_aes: Option<bool>,
    /// Whether a layer with its own data borrows the plot data's columns when both
    /// have the same row count (lets-plot); default true. gghighlight's grey layers
    /// set it to false, so a later facet finds no faceting column in them and draws
    /// them in every panel.
    #[serde(default)]
    pub merge_plot_data: Option<bool>,
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
    /// `sampling=`: `"none"`, a sampler spec or a chain (see [`crate::sampling`]).
    #[serde(default)]
    pub sampling: Option<Value>,
    /// ggnewscale scale group per style aesthetic (`new_scale_color()` before the
    /// layer): the layer maps through that group's scale. Missing means group 0.
    #[serde(default)]
    pub scale_group: BTreeMap<Aesthetic, u32>,
    /// ggrastr `rasterize()`: draw the layer as an embedded bitmap in SVG and PDF.
    #[serde(default)]
    pub rasterize: Option<RasterizeSpec>,
    /// ggside side layer (`geom_xside*()` / `geom_yside*()`): `"x"` or `"y"`, the
    /// axis the layer shares with the main panel (see [`super::side`]).
    #[serde(default)]
    pub ggside: Option<Value>,
    /// ggside layer: the screen side of its band (`"top"`/`"bottom"` for an x side
    /// layer, `"left"`/`"right"` for a y one), overriding `ggside(x_pos=, y_pos=)`.
    #[serde(default)]
    pub side_pos: Option<Value>,
    /// ggside layer: the name of its band; layers with the same name (and side)
    /// share a band, and bands on one side stack outward in plot order.
    #[serde(default)]
    pub side_band: Option<Value>,
    /// ggside layer: its band's thickness as a share of the main panel
    /// (overrides `ggside_panel_scale_*`; the largest of a band's layers wins).
    #[serde(default)]
    pub side_size: Option<Value>,
    /// lets-plot `ggmarginal()`: the layer draws in a margin of the panel.
    #[serde(default)]
    pub marginal: Option<Value>,
    /// lets-plot `ggmarginal()`: the margin, `"t"`, `"b"`, `"l"` or `"r"`.
    #[serde(default)]
    pub margin_side: Option<Value>,
    /// lets-plot `ggmarginal()`: the margin's share of the plotting area.
    #[serde(default)]
    pub margin_size: Option<Value>,
    /// Geom/stat specific options and constant aesthetics (`size=2`, `color="red"`).
    /// Validated by the geom and stat.
    #[serde(flatten)]
    pub parameters: Options,
}

impl LayerSpec {
    /// The ggnewscale scale group of `aesthetic` (0 without `new_scale_*()`).
    pub fn scale_group(&self, aesthetic: Aesthetic) -> u32 {
        self.scale_group.get(&aesthetic).copied().unwrap_or(0)
    }

    pub(super) fn collect_unsupported(&self, parent: &str, found: &mut Vec<String>) {
        // lets-plot samplers the engine skips (stratified, group, vertex).
        if let Some(sampling) = &self.sampling {
            found.extend(crate::sampling::unsupported_paths(
                sampling,
                &join_path(parent, "sampling"),
            ));
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

/// `rasterize(layer, dpi=...)` options of a layer.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(try_from = "Value")]
pub struct RasterizeSpec {
    /// Resolution of the bitmap in dots per inch of the output.
    pub dpi: f64,
}

/// Highest accepted `rasterize` dpi (a 600 px panel at 4800 dpi is 30,000 pixels
/// wide, near the PNG backend's side limit).
pub const MAX_RASTERIZE_DPI: f64 = 4800.0;

impl TryFrom<Value> for RasterizeSpec {
    type Error = String;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let Value::Object(options) = value else {
            return Err(format!(
                "expected rasterize options like {{\"dpi\": 300}}, got {value}"
            ));
        };
        if let Some(key) = options.keys().find(|key| key.as_str() != "dpi") {
            return Err(format!(
                "at '{key}': unknown rasterize option; accepted: 'dpi'"
            ));
        }
        let dpi = match options.get("dpi") {
            None => return Err("at 'dpi': missing; give the bitmap resolution, e.g. 300".into()),
            Some(dpi) => dpi.as_f64().ok_or_else(|| {
                format!("at 'dpi': expected a number of dots per inch, got {dpi}")
            })?,
        };
        if !(dpi.is_finite() && dpi > 0.0 && dpi <= MAX_RASTERIZE_DPI) {
            return Err(format!(
                "at 'dpi': expected dots per inch in (0, {MAX_RASTERIZE_DPI}], got {dpi}; \
                 300 suits print, 96 the screen"
            ));
        }
        Ok(RasterizeSpec { dpi })
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
