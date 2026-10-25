//! Guides: legends, colorbars, and axis titles set through `labs()`.

use serde::Deserialize;

use super::{Options, Value, report_unsupported};

/// A guide, given as a name (`"none"`, `"legend"`) or with options.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum GuideSpec {
    /// `"none"`, `"legend"`, `"colorbar"`.
    Name(String),
    /// `guide_legend(...)`, `guide_colorbar(...)`, or `{"title": ...}` from `labs()`.
    Full(Box<GuideOptions>),
}

/// Options of a legend or colorbar guide.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct GuideOptions {
    /// `legend` or `colorbar`; absent for a title-only entry from `labs()`.
    pub name: Option<String>,
    /// Guide (or axis) title.
    pub title: Option<String>,
    /// Legend columns.
    pub ncol: Option<u32>,
    /// Legend rows.
    pub nrow: Option<u32>,
    /// Fill legend keys row by row.
    pub byrow: Option<bool>,
    /// Aesthetic overrides for legend keys, e.g. `{"size": 3}`.
    pub override_aes: Option<Options>,
    /// Colorbar width in pixels.
    pub barwidth: Option<f64>,
    /// Colorbar height in pixels.
    pub barheight: Option<f64>,
    /// Number of colorbar bins.
    pub nbin: Option<u32>,
    /// Legend order among guides.
    pub order: Option<i64>,
    /// Manual legend entries.
    pub manual_key: Option<Value>,
    /// Keys no component consumes.
    #[serde(flatten)]
    pub unsupported: Options,
}

impl GuideSpec {
    pub(super) fn collect_unsupported(&self, parent: &str, found: &mut Vec<String>) {
        if let GuideSpec::Full(options) = self {
            report_unsupported(parent, &options.unsupported, found);
        }
    }
}
