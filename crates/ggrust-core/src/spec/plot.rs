//! A single plot: `ggplot(...) + layers + scales + theme + ...`.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::de::IgnoredAny;

use super::{
    Aesthetic, CoordSpec, FacetSpec, GuideSpec, LayerSpec, Mapping, Options, ScaleSpec, ThemeSpec,
    Value, join_path, report_unsupported,
};

/// Everything `PlotSpec.as_dict()` produces for one plot.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PlotSpec {
    /// Figure kind, always `plot`. Consumed by [`super::Figure`] dispatch.
    pub kind: String,
    /// Inline data. Skipped: frames are passed separately as Arrow.
    pub data: Option<IgnoredAny>,
    /// Plot-level aesthetic mapping, inherited by layers.
    pub mapping: Mapping,
    /// Column types and categorical ordering of the plot data.
    pub data_meta: DataMeta,
    /// Layers, in drawing order.
    pub layers: Vec<LayerSpec>,
    /// Scales, in the order they were added (later ones win).
    pub scales: Vec<ScaleSpec>,
    /// Theme, if any was set.
    pub theme: Option<ThemeSpec>,
    /// Guides keyed by aesthetic name; also carries axis titles from `labs()`.
    pub guides: BTreeMap<String, GuideSpec>,
    /// Coordinate system.
    pub coord: Option<CoordSpec>,
    /// Faceting.
    pub facet: Option<FacetSpec>,
    /// `ggsize(width, height)` in pixels.
    pub ggsize: Option<SizeSpec>,
    /// `ggtitle(...)` / `labs(title=..., subtitle=...)`.
    pub ggtitle: Option<TitleSpec>,
    /// `labs(caption=...)`.
    pub caption: Option<CaptionSpec>,
    /// `ggtb(...)`: interactive zoom/pan toolbar.
    pub ggtoolbar: Option<ToolbarSpec>,
    /// lets-plot front-end metadata. Accepted and ignored.
    pub metainfo_list: Vec<IgnoredAny>,
    /// Keys no component consumes.
    #[serde(flatten)]
    pub unsupported: Options,
}

impl PlotSpec {
    pub(super) fn collect_unsupported(&self, parent: &str, found: &mut Vec<String>) {
        report_unsupported(parent, &self.unsupported, found);
        for (index, layer) in self.layers.iter().enumerate() {
            layer.collect_unsupported(&join_path(parent, &format!("layers[{index}]")), found);
        }
        if let Some(theme) = &self.theme {
            theme.collect_unsupported(&join_path(parent, "theme"), found);
        }
        for (aesthetic, guide) in &self.guides {
            guide.collect_unsupported(&join_path(parent, &format!("guides.{aesthetic}")), found);
        }
        if let Some(coord) = &self.coord {
            report_unsupported(&join_path(parent, "coord"), &coord.unsupported, found);
        }
        if let Some(facet) = &self.facet {
            report_unsupported(&join_path(parent, "facet"), &facet.unsupported, found);
        }
        if let Some(toolbar) = &self.ggtoolbar {
            report_unsupported(&join_path(parent, "ggtoolbar"), &toolbar.unsupported, found);
        }
    }
}

/// Column types and categorical ordering (`as_discrete`, pandas/polars categoricals).
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct DataMeta {
    /// Per-column annotations.
    pub series_annotations: Vec<SeriesAnnotation>,
    /// Per-aesthetic annotations, e.g. `as_discrete`.
    pub mapping_annotations: Vec<MappingAnnotation>,
}

/// Type (and category order) of one data column.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SeriesAnnotation {
    /// Column name.
    pub column: String,
    /// lets-plot type name: `int`, `float`, `str`, `bool`, `datetime`, ...
    #[serde(rename = "type")]
    pub series_type: Option<String>,
    /// Category levels, in order.
    pub factor_levels: Option<Vec<Value>>,
    /// `1` ascending, `-1` descending.
    pub order: Option<i64>,
}

/// An annotation on a mapped aesthetic, e.g. `color=as_discrete("cluster")`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MappingAnnotation {
    /// The annotated aesthetic.
    pub aes: Aesthetic,
    /// Annotation kind; lets-plot only emits `as_discrete`.
    pub annotation: String,
    /// Annotation options.
    #[serde(default)]
    pub parameters: MappingAnnotationParameters,
}

/// Options of `as_discrete(...)`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct MappingAnnotationParameters {
    /// Legend/axis label.
    pub label: Option<String>,
    /// Column to order categories by.
    pub order_by: Option<String>,
    /// `1` ascending, `-1` descending.
    pub order: Option<i64>,
}

/// Plot size in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct SizeSpec {
    /// Width in pixels.
    pub width: f64,
    /// Height in pixels.
    pub height: f64,
}

/// Plot title and subtitle.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TitleSpec {
    /// Title text.
    pub text: Option<String>,
    /// Subtitle text.
    pub subtitle: Option<String>,
}

/// Plot caption.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CaptionSpec {
    /// Caption text.
    pub text: Option<String>,
}

/// `ggtb(size_basis=..., size_zoomin=...)`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct ToolbarSpec {
    /// Which scale(s) symbol sizes follow when zooming.
    pub size_basis: Option<String>,
    /// How symbol sizes react to zoom-in; `-1` means unlimited growth.
    pub size_zoomin: Option<f64>,
    /// Keys no component consumes.
    #[serde(flatten)]
    pub unsupported: Options,
}
