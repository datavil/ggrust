//! Guides: legends, colorbars, and axis titles set through `labs()`.

use serde::Deserialize;

use super::{Options, ThemeSpec, Value, from_value_tracked, optional_count, report_unsupported};

/// A guide, given as a name (`"none"`, `"legend"`) or with options.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "Value")]
pub enum GuideSpec {
    /// `"none"`, `"legend"`, `"colorbar"`, `"bins"`.
    Name(String),
    /// `guide_legend(...)`, `guide_colorbar(...)`, or `{"title": ...}` from `labs()`.
    Full(Box<GuideOptions>),
}

impl GuideSpec {
    /// Whether a style aesthetic's guide uses only options ggrust draws: `"none"`,
    /// `"legend"`, `"colorbar"`, a title, `guide_legend(ncol, nrow, byrow, reverse,
    /// direction, order, position, theme)` and `guide_colorbar(barwidth, barheight, nbin,
    /// label_position, display, alpha, draw_ulim, draw_llim, shape, show, minor_ticks)`
    /// plus those four, and
    /// `guide_bins(show_limits)` with the legend's options.
    /// Anything else (`override_aes`, ...) is not built yet.
    pub fn is_built_style_guide(&self) -> bool {
        match self {
            GuideSpec::Name(name) => {
                matches!(name.as_str(), "none" | "legend" | "colorbar" | "bins")
            }
            GuideSpec::Full(options) => {
                let colorbar = options.name.as_deref() == Some("colorbar");
                let bins = options.name.as_deref() == Some("bins");
                let rest = GuideOptions {
                    name: options
                        .name
                        .clone()
                        .filter(|name| !matches!(name.as_str(), "legend" | "colorbar" | "bins")),
                    title: None,
                    ncol: None,
                    nrow: None,
                    byrow: None,
                    override_aes: None,
                    barwidth: options.barwidth.filter(|_| !colorbar),
                    barheight: options.barheight.filter(|_| !colorbar),
                    nbin: options.nbin.filter(|_| !colorbar),
                    label_position: options.label_position.clone().filter(|_| !colorbar),
                    display: options.display.clone().filter(|_| !colorbar),
                    alpha: options.alpha.filter(|_| !colorbar),
                    draw_ulim: options.draw_ulim.filter(|_| !colorbar),
                    draw_llim: options.draw_llim.filter(|_| !colorbar),
                    shape: options.shape.clone().filter(|_| !colorbar),
                    show: options.show.clone().filter(|_| !colorbar),
                    minor_ticks: options.minor_ticks.filter(|_| !colorbar),
                    show_limits: options.show_limits.filter(|_| !bins),
                    reverse: None,
                    direction: None,
                    position: None,
                    order: None,
                    theme: None,
                    unsupported: Default::default(),
                    ..(**options).clone()
                };
                rest == GuideOptions::default()
            }
        }
    }
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
    #[serde(deserialize_with = "optional_count")]
    pub ncol: Option<u32>,
    /// Legend rows.
    #[serde(deserialize_with = "optional_count")]
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
    #[serde(deserialize_with = "optional_count")]
    pub nbin: Option<u32>,
    /// Colorbar label placement: `"ends"` puts the lowest and highest break labels
    /// beside the bar's ends (a ggrust option).
    pub label_position: Option<String>,
    /// Legend order among guides: 1 to 99 first, in order; 0 (or none) last.
    pub order: Option<i64>,
    /// List the keys backwards; a colorbar puts high values at the bottom (left).
    pub reverse: Option<bool>,
    /// `"horizontal"` or `"vertical"`: this guide's layout over `legend_direction`.
    pub direction: Option<String>,
    /// `"top"`, `"right"`, `"bottom"`, `"left"` or `"inside"`: where this guide
    /// goes, over `legend_position` (ggplot2 3.5).
    pub position: Option<String>,
    /// `theme(legend_*=...)` elements for this guide only.
    pub theme: Option<ThemeSpec>,
    /// Colorbar `display`: `"rectangles"` (flat segments), `"gradient"`, `"raster"`.
    pub display: Option<String>,
    /// Colorbar opacity, 0 to 1.
    pub alpha: Option<f64>,
    /// Colorbar: draw the ticks of a break at the upper end.
    pub draw_ulim: Option<bool>,
    /// Colorbar: draw the ticks of a break at the lower end.
    pub draw_llim: Option<bool>,
    /// Colorbar end caps: `"triangle"` (legendry `guide_colbar(shape=)`; ggrust).
    pub shape: Option<String>,
    /// Colorbar caps shown: `true`, `false` or `[lower, upper]` (legendry
    /// `guide_colbar(show=)`); unset, both ends when `shape` is given.
    pub show: Option<Value>,
    /// Colorbar minor ticks between the breaks (ggrust; ggplot2 `minor.ticks`).
    pub minor_ticks: Option<bool>,
    /// `guide_bins(show_limits=)`: label the two outer edges (ggplot2).
    pub show_limits: Option<bool>,
    /// Manual legend entries.
    pub manual_key: Option<Value>,
    /// Keys no component consumes.
    #[serde(flatten)]
    pub unsupported: Options,
}

impl TryFrom<Value> for GuideSpec {
    type Error = String;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::String(name) => Ok(GuideSpec::Name(name)),
            Value::Object(_) => from_value_tracked(value).map(GuideSpec::Full),
            other => Err(format!(
                "expected a guide name or guide_*() options, got {other}"
            )),
        }
    }
}

impl GuideSpec {
    pub(super) fn collect_unsupported(&self, parent: &str, found: &mut Vec<String>) {
        // `guide_axis()` / `guide_axis_mark()` / `guide_axis_nested()` options are
        // read (and checked) with the position scales.
        if let GuideSpec::Full(options) = self
            && !matches!(
                options.name.as_deref(),
                Some("axis" | "axis_mark" | "axis_nested")
            )
        {
            report_unsupported(parent, &options.unsupported, found);
        }
        // `guide_axis()` reads `title` and `cap`; its other ggplot2 options are not built.
        if let GuideSpec::Full(options) = self
            && options.name.as_deref() == Some("axis")
        {
            let mut rest = options.unsupported.clone();
            rest.remove("cap");
            report_unsupported(parent, &rest, found);
        }
    }
}
